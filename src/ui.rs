mod device;

use crate::device::RemoteTcpDevice;
use crate::controller;
use std::sync::{Arc,Mutex};
use log::{debug,info};
use eframe::egui;

struct MyApp {
    discovery_ip: String,
    devices:  Arc<Mutex<Vec<RemoteTcpDevice>>>,
    backend_run: bool,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            discovery_ip: "".to_string(),
            devices: Arc::new(Mutex::new(Vec::<RemoteTcpDevice>::new())),
            backend_run: false,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if !self.backend_run {
                let devices = self.devices.clone();
                // start backend
                start_backend(ctx.clone(),devices).expect("backend run failed");
                self.backend_run = true;
            }
            ui.add(egui::TextEdit::singleline(&mut self.discovery_ip).hint_text("192.168.1.100"));
            if ui.button("add").on_hover_text("add a device").clicked() {
                debug!("abc");
            }

            ui.vertical(|ui|{
                let devices = self.devices.lock().unwrap();
                if devices.is_empty() {
                    return;
                }
                let len = devices.len();
                let row = (len-1) / 2 + 1;
                
                for i in 0..row {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Min),|ui|{
                        for j in 0..2 {
                            if let Some(dev) = devices.get(i*2+j) {
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
    let app = MyApp::default();

    if let Err(e) = eframe::run_native(
        "RSDrop",
        options,
        Box::new(|_cc| Box::new(app)),
    ) {
        println!("run app failed. Err:{}",e);
    }
    Ok(())
}

fn start_backend(ctx: egui::Context,devices: Arc<Mutex<Vec<RemoteTcpDevice>>>) -> std::io::Result<()> {
    std::thread::spawn(move ||{
        let runtime = tokio::runtime::Runtime::new().unwrap();
        if let Err(e) = runtime.block_on(async move{
            let mut controller = controller::Controller::new(ctx);
            controller.set_device_container(devices);
            let rx = controller.start_discovery_service().await?;
            controller.start_service().await?;
            let res: tokio::io::Result<()> = tokio::select!{
                _ = controller.cmd_loop() => {
                    debug!("cmd loop end");
                    Ok(())
                }
                _ = controller.sync_device_loop(rx) => {
                    debug!("sync device loop end");
                    Ok(())
                }
            };
            res
        }) {
            info!("runtime err: {}",e);
        }
    });
    
    Ok(())
}