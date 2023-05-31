use eframe::egui;

const ICON: &[u8; 15473] = include_bytes!("../assets/indi_logo.png");

fn main() -> Result<(), eframe::Error> {
    // let's make sure the needed file/dirs for the config are there
    indiserver_ui::ensure_config_exists();
    let _conf = indiserver_ui::Config::new();

    let icon = image::load_from_memory(ICON).unwrap().to_rgba8();
    let (i_width, i_height) = icon.dimensions();
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(280.0, 450.0)),
        icon_data: Some(eframe::IconData {
            rgba: icon.into_raw(),
            width: i_width,
            height: i_height,
        }),
        ..Default::default()
    };
    eframe::run_native(
        "INDI Server GUI",
        options,
        Box::new(|_cc| Box::new(IndiUI::new())),
    )
}

pub fn toggle_ui(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let desired_size = ui.spacing().interact_size.y * egui::vec2(2.2, 1.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }

    response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, *on, ""));

    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool(response.id, *on);
        let visuals = ui.style().interact_selectable(&response, *on);
        let rect = rect.expand(visuals.expansion);
        let radius = 0.5 * rect.height();
        ui.painter()
            .rect(rect, radius, visuals.bg_fill, visuals.bg_stroke);
        let circle_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
        let center = egui::pos2(circle_x, rect.center().y);
        ui.painter()
            .circle(center, 0.75 * radius, visuals.bg_fill, visuals.fg_stroke);
    }

    response
}

pub fn toggle(on: &mut bool) -> impl egui::Widget + '_ {
    move |ui: &mut egui::Ui| toggle_ui(ui, on)
}

struct IndiUI {
    binaries: Vec<(String, String, bool)>,
    filter: String,
    server_on: bool,
    server_on_text: String,
    server_off_text: String,
    indi_proc: Option<std::process::Child>,
    can_start_indi: bool,
    config: indiserver_ui::Config,
}

impl IndiUI {
    fn new() -> Self {
        let conf = indiserver_ui::Config::new();
        let drivers = conf.drivers();
        Self {
            binaries: {
                let mut available_drivers = indiserver_ui::fetch_indi_binaries();

                for driver in available_drivers.iter_mut() {
                    if drivers.contains(&driver.1) {
                        driver.2 = true;
                    }
                }

                available_drivers
            },
            filter: String::new(),
            server_on: false,
            server_on_text: String::from("\n  Stop INDI server  \n"),
            server_off_text: String::from("\n  Start INDI server  \n"),
            indi_proc: None,
            can_start_indi: false,
            config: conf,
        }
    }
}

impl eframe::App for IndiUI {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("color theme:");
                egui::widgets::global_dark_light_mode_buttons(ui);
            });

            ui.with_layout(egui::Layout::top_down_justified(egui::Align::TOP), |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Indiserver GUI");
                });
                ui.separator();
            });

            ui.horizontal(|ui| {
                let name_label = ui.label("Filter: ");
                ui.text_edit_singleline(&mut self.filter)
                    .labelled_by(name_label.id);
            });

            ui.separator();

            ui.vertical_centered(|ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        egui::Grid::new("binaries")
                            .num_columns(2)
                            .spacing([10.0, 10.0])
                            .striped(true)
                            .show(ui, |ui| {
                                let mut global_status = 0;
                                for el in self.binaries.iter_mut() {
                                    // Check now if we need to display the item or not, only items
                                    // that match the filters are shown on the UI
                                    if &self.filter == "" || el.0.contains(&self.filter) {
                                        ui.label(el.0.to_owned());
                                        ui.horizontal(|ui| {
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::RIGHT),
                                                |ui| {
                                                    ui.add(toggle(&mut el.2));
                                                },
                                            );
                                        });
                                        ui.end_row();
                                    };

                                    if el.2 {
                                        global_status += 1
                                    }
                                }

                                match global_status {
                                    0 => self.can_start_indi = false,
                                    _ => self.can_start_indi = true,
                                }
                            });
                    });
            });
        });

        egui::TopBottomPanel::bottom("footer").show(ctx, |ui| {
            ui.add_space(11.0);
            ui.vertical_centered(|ui| {
                let text = if self.server_on {
                    self.server_on_text.to_owned()
                } else {
                    self.server_off_text.to_owned()
                };
                let button = egui::Button::new(egui::widget_text::RichText::new(text).size(11.0))
                    .min_size(egui::vec2(50.0, 20.0));
                if ui.add_enabled(self.can_start_indi, button).clicked() {
                    match self.server_on {
                        true => {
                            if let Some(ref mut child) = self.indi_proc {
                                child.kill().expect("failed to kill INDI server");
                                self.server_on = !self.server_on;
                            };
                        }
                        false => {
                            // Which drivers we want to start
                            let mut to_start = Vec::new();
                            for (_name, path, status) in self.binaries.clone().into_iter() {
                                if status {
                                    to_start.push(path);
                                }
                            }

                            if !to_start.is_empty() {
                                // Save the driver config in the config file
                                self.config.add_drivers_to_config(&to_start);

                                // Now start INDI server
                                match indiserver_ui::start_indi(to_start) {
                                    Ok(handle) => {
                                        self.indi_proc = Some(handle);
                                        self.server_on = !self.server_on;
                                    }
                                    Err(_) => (),
                                }
                            };
                        }
                    }
                };
            });
            ui.add_space(7.0);
        });
    }
}
