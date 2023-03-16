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
    cts: Option<tokio::sync::mpsc::Sender<String>>
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            discovery_ip: "".to_string(),
            devices: Arc::new(Mutex::new(Vec::<RemoteTcpDevice>::new())),
            backend_run: false,
            cts: None,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if !self.backend_run {
                let devices = self.devices.clone();
                // start backend
                let cts = start_backend(ctx.clone(),devices).expect("backend run failed");
                self.cts = Some(cts);
                self.backend_run = true;
            }
            ui.add(egui::TextEdit::singleline(&mut self.discovery_ip).hint_text("192.168.1.100"));
            if ui.button("add").on_hover_text("add a device").clicked() {
                let sender = self.cts.clone().unwrap();
                let ip = self.discovery_ip.clone();
                let mut rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async move {sender.send(ip).await.expect("send failed");});
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

fn start_backend(ctx: egui::Context,devices: Arc<Mutex<Vec<RemoteTcpDevice>>>) -> std::io::Result<(tokio::sync::mpsc::Sender<String>)> {
    let mut controller = controller::Controller::new(ctx);
    controller.set_device_container(devices);
    let (ctx,crx) = controller.gen_ctx();
    std::thread::spawn(move ||{
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {controller.start_loop().await.expect("abc");});
    });
    
    Ok(ctx)
}