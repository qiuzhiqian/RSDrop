use eframe::egui::*;

/// Clickable button with text.
///
/// See also [`Ui::button`].
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// # fn do_stuff() {}
///
/// if ui.add(egui::Button::new("Click me")).clicked() {
///     do_stuff();
/// }
///
/// // A greyed-out and non-interactive button:
/// if ui.add_enabled(false, egui::Button::new("Can't click this")).clicked() {
///     unreachable!();
/// }
/// # });
/// ```
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct FusionLabel {
    text: WidgetText,
    shortcut_text: WidgetText,
    wrap: Option<bool>,
    /// None means default for interact
    fill: Option<Color32>,
    stroke: Option<Stroke>,
    small: bool,
    frame: Option<bool>,
    min_size: Vec2,
    image: Option<widgets::Image>,
}

impl FusionLabel {
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self {
            text: text.into(),
            shortcut_text: Default::default(),
            wrap: None,
            fill: None,
            stroke: None,
            small: false,
            frame: None,
            min_size: Vec2::ZERO,
            image: None,
        }
    }

    /// Creates a button with an image to the left of the text. The size of the image as displayed is defined by the provided size.
    #[allow(clippy::needless_pass_by_value)]
    pub fn image_and_text(
        texture_id: TextureId,
        image_size: impl Into<Vec2>,
        text: impl Into<WidgetText>,
    ) -> Self {
        Self {
            image: Some(widgets::Image::new(texture_id, image_size)),
            ..Self::new(text)
        }
    }

    /// If `true`, the text will wrap to stay within the max width of the [`Ui`].
    ///
    /// By default [`Self::wrap`] will be true in vertical layouts
    /// and horizontal layouts with wrapping,
    /// and false on non-wrapping horizontal layouts.
    ///
    /// Note that any `\n` in the text will always produce a new line.
    #[inline]
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = Some(wrap);
        self
    }

    /// Override background fill color. Note that this will override any on-hover effects.
    /// Calling this will also turn on the frame.
    pub fn fill(mut self, fill: impl Into<Color32>) -> Self {
        self.fill = Some(fill.into());
        self.frame = Some(true);
        self
    }

    /// Override button stroke. Note that this will override any on-hover effects.
    /// Calling this will also turn on the frame.
    pub fn stroke(mut self, stroke: impl Into<Stroke>) -> Self {
        self.stroke = Some(stroke.into());
        self.frame = Some(true);
        self
    }

    /// Make this a small button, suitable for embedding into text.
    pub fn small(mut self) -> Self {
        self.text = self.text.text_style(TextStyle::Body);
        self.small = true;
        self
    }

    /// Turn off the frame
    pub fn frame(mut self, frame: bool) -> Self {
        self.frame = Some(frame);
        self
    }

    /// Set the minimum size of the button.
    pub fn min_size(mut self, min_size: Vec2) -> Self {
        self.min_size = min_size;
        self
    }

    /// Show some text on the right side of the button, in weak color.
    ///
    /// Designed for menu buttons, for setting a keyboard shortcut text (e.g. `Ctrl+S`).
    ///
    /// The text can be created with [`Context::format_shortcut`].
    pub fn shortcut_text(mut self, shortcut_text: impl Into<WidgetText>) -> Self {
        self.shortcut_text = shortcut_text.into();
        self
    }
}

impl Widget for FusionLabel {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self {
            text,
            shortcut_text,
            wrap,
            fill,
            stroke,
            small,
            frame,
            min_size,
            image,
        } = self;

        let frame = frame.unwrap_or_else(|| ui.visuals().button_frame);

        let mut button_padding = ui.spacing().button_padding;
        if small {
            button_padding.y = 0.0;
        }

        let mut text_wrap_width = ui.available_width() - 2.0 * button_padding.x;
        if let Some(image) = image {
            text_wrap_width -= image.size().x + ui.spacing().icon_spacing;
        }
        if !shortcut_text.is_empty() {
            text_wrap_width -= 60.0; // Some space for the shortcut text (which we never wrap).
        }

        let text = text.into_galley(ui, wrap, text_wrap_width, TextStyle::Button);
        let shortcut_text = (!shortcut_text.is_empty())
            .then(|| shortcut_text.into_galley(ui, Some(false), f32::INFINITY, TextStyle::Button));

        let mut desired_size = text.size();
        if let Some(image) = image {
            desired_size.x += image.size().x + ui.spacing().icon_spacing;
            desired_size.y = desired_size.y.max(image.size().y);
        }
        if let Some(shortcut_text) = &shortcut_text {
            desired_size.x += ui.spacing().item_spacing.x + shortcut_text.size().x;
            desired_size.y = desired_size.y.max(shortcut_text.size().y);
        }
        if !small {
            desired_size.y = desired_size.y.at_least(ui.spacing().interact_size.y);
        }
        desired_size += 2.0 * button_padding;
        desired_size = desired_size.at_least(min_size);

        let (rect, response) = ui.allocate_at_least(desired_size, Sense::hover());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, text.text()));

        if ui.is_rect_visible(rect) {
            let visuals = ui.style().interact(&response);

            if frame {
                let fill = fill.unwrap_or(visuals.bg_fill);
                let stroke = stroke.unwrap_or(visuals.bg_stroke);
                ui.painter().rect(
                    rect.expand(visuals.expansion),
                    visuals.rounding,
                    fill,
                    stroke,
                );
            }

            let text_pos = if let Some(image) = image {
                let icon_spacing = ui.spacing().icon_spacing;
                pos2(
                    rect.min.x + button_padding.x + image.size().x + icon_spacing,
                    rect.center().y - 0.5 * text.size().y,
                )
            } else {
                ui.layout()
                    .align_size_within_rect(text.size(), rect.shrink2(button_padding))
                    .min
            };
            text.paint_with_visuals(ui.painter(), text_pos, visuals);

            if let Some(shortcut_text) = shortcut_text {
                let shortcut_text_pos = pos2(
                    rect.max.x - button_padding.x - shortcut_text.size().x,
                    rect.center().y - 0.5 * shortcut_text.size().y,
                );
                shortcut_text.paint_with_fallback_color(
                    ui.painter(),
                    shortcut_text_pos,
                    ui.visuals().weak_text_color(),
                );
            }

            if let Some(image) = image {
                let image_rect = Rect::from_min_size(
                    pos2(rect.min.x, rect.center().y - 0.5 - (image.size().y / 2.0)),
                    image.size(),
                );
                image.paint_at(ui, image_rect);
            }
        }

        response
    }
}
