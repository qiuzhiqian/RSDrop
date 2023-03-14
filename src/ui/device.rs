use eframe::egui;

use crate::components::fusion_label::FusionLabel;

pub struct Device {
    pub name: String,
    pub ip: String,
    pub r#type: String,
    pub id: String,
}

impl Default for Device {
    fn default() -> Self {
        Self{
            name: "".to_string(),
            ip: "".to_string(),
            r#type: "".to_string(),
            id: "".to_string(),
        }
    }
}

/// |---------| |---------------------|
/// |         | |---------------------|
/// |         |
/// |         | |-----| |-----| |-----|
/// |---------| |-----| |-----| |-----|
impl Device {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(0x3e,0x48,0x47))
            .rounding(egui::Rounding::same(5.0))
            .inner_margin(egui::Vec2::splat(2.0))
            .show(ui, |ui| {
            let bg = egui::Color32::from_rgb(0x65,0xfc,0xd5);
            ui.horizontal(|ui| {
                ui.add(FusionLabel::new(format!("type: {}",self.r#type)).fill(bg).min_size(egui::vec2(64.0, 64.0))).on_hover_text("profile");
                ui.vertical(|ui| {
                    let type_text= egui::RichText::new(format!("{}",self.name))
                        .color(egui::Color32::WHITE)
                        .strong()
                        .size(20.0);//format!("{}",self.name).into();
                    ui.label(type_text);
                    ui.horizontal(|ui| {
                        let type_text: egui::WidgetText = format!("{}",self.r#type).into();
                        ui.add(FusionLabel::new(type_text.color(egui::Color32::BLACK)).fill(bg)).on_hover_text("profile");
                        let ip_text: egui::WidgetText = format!("{}",self.ip).into();
                        ui.add(FusionLabel::new(ip_text.color(egui::Color32::BLACK)).fill(bg)).on_hover_text("profile");
                    });
                });
            });
        });
    }
}