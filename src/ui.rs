mod device;

use eframe::egui;

struct MyApp {
    input: String,
    devices: Vec<crate::device::RemoteTcpDevice>,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            input: "".to_string(),
            devices: Vec::new(),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("click").clicked() {
                println!("abc");
            }

            ui.vertical(|ui|{
                if self.devices.is_empty() {
                    return;
                }
                let len = self.devices.len();
                let row = (len-1) / 2 + 1;
                
                for i in 0..row {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Min),|ui|{
                        for j in 0..2 {
                            if let Some(dev) = self.devices.get(i*2+j) {
                                device::Device{
                                    name: dev.device.name.clone(),
                                    ip: dev.addr.ip().to_string(),
                                    r#type: dev.device.r#type.clone(),
                                    id: dev.device.id.clone(),
                                }.ui(ui);
                            } else {
                                break;
                            }
                        }
                    });
                }
            });
        });
    }
}

pub fn start() -> std::io::Result<()> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(500.0, 480.0)),
        resizable: false,
        ..Default::default()
    };
    if let Err(e) = eframe::run_native(
        "RSDrop",
        options,
        Box::new(|_cc| Box::new(MyApp::default())),
    ) {
        println!("run app failed. Err:{}",e);
    }
    Ok(())
}