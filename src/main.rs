#![windows_subsystem = "windows"]
mod functions;
use std::string::ToString;
use eframe::egui;
use rfd::FileDialog;
use which;

#[derive(Default)]
struct MainWindow {
    ffmpeg_path: String,
    selected_path: Option<String>,
    limitation: String, // to check if input is empty, we need to use String here
    message: String,
    file_node_type: String,
}

impl MainWindow {
    fn new() -> Self {
        Self {
            ffmpeg_path: "Empty for ffmpeg in PATH".to_string(),
            selected_path: None,
            limitation: "-14".to_string(), // default value
            message: String::new(),
            file_node_type: String::new(),
        }
    }
}

impl eframe::App for MainWindow {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // row: select ffmpeg binary
            ui.horizontal(|ui| {
                // ffmpeg binary selection button
                if ui.button("Select ffmpeg binary").clicked() {
                    if let Some(path) = FileDialog::new().pick_file() {
                        self.ffmpeg_path = path.display().to_string();
                    }
                    if !self.ffmpeg_path.is_empty() {
                        self.ffmpeg_path = self.ffmpeg_path.to_string();
                    }
                }
                ui.label(&self.ffmpeg_path);
            });
            // row: select file or folder
            ui.horizontal(|ui| {
                // file selection button
                if ui.button("Select file").clicked() {
                    if let Some(path) = FileDialog::new().pick_file() {
                        self.selected_path = Some(path.display().to_string());
                        self.file_node_type = "file".to_string();
                    }
                }
                // folder selection button
                if ui.button("Select folder").clicked() {
                    if let Some(path) = FileDialog::new().pick_folder() {
                        self.selected_path = Some(path.display().to_string());
                        self.file_node_type = "folder".to_string();
                    }
                }
                // show selected file or folder
                if let Some(file) = &self.selected_path {
                    ui.label(format!("Selected: {}", file));
                }

            });
            // row: Loudness limitation settings
            ui.horizontal(|ui| {
                // loudness limitation input
                ui.label("Input loudness limitation:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.limitation)
                        .hint_text("-14")
                        .desired_width(60.0),
                );
                ui.label("LKFS.  (e.g. ITU-R BS.1770-4 standard is -14 LKFS)");
            });
            // start button and message
            ui.horizontal(|ui| {
                if ui.button("Start processing").clicked() {
                    println!("Start button clicked. Path: {}, {}, Limitation: {}LKFS", &self.selected_path.as_deref().unwrap_or("-14"), self.file_node_type, self.limitation);
                    // limitation <- parse input text
                    // when it is not empty (empty means default: -14.0)
                    if self.limitation.is_empty() {
                        self.limitation = "-14".to_string(); // default value
                    };
                    // check if limitation is a valid number
                    if let Err(err) = self.limitation.parse::<f32>() {
                        self.message = format!("Invalid input for loudness limitation. Please enter a valid number. {}", err).to_string();
                        return;
                    }
                    // check if ffmpeg path is set
                    if self.ffmpeg_path.is_empty() {
                        // check if ffmpeg is in PATH
                        if let Ok(ffmpeg_path) = which::which("ffmpeg") {
                            self.ffmpeg_path = ffmpeg_path.display().to_string();
                        } else {
                            self.message = "Please select ffmpeg binary.".to_string();
                        }
                    }
                    // check if selected path is empty, file or folder
                    if let Some(path) = &self.selected_path {
                        if self.file_node_type == "file" {
                            // process single file
                            println!("Processing file: {}", path);
                            self.message = format!("Processing file: {}", path);
                            match functions::ffmpeg_process(path, &self.ffmpeg_path, self.limitation.parse::<f32>().unwrap()) {
                                Ok(val) => {
                                    self.message = format!("Success: {}", val.to_string());
                                }
                                Err(err) => {
                                    self.message = format!("Error: {}", err.to_string());
                                }
                            };   // just unwrap limitation here, because we already checked if limitation is a valid number
                        } else if self.file_node_type == "folder" {
                            // process all files in folder
                            println!("Processing folder: {}", path);
                            self.message = format!("Processing folder: {}", path);
                            match functions::ffmpeg_process_dir(path, &self.ffmpeg_path, self.limitation.parse::<f32>().unwrap()){
                                Ok(val) => {
                                    self.message = format!("Success: {}", val.to_string());
                                },
                                Err(err) => {
                                    self.message = format!("Error: {}", err.to_string());
                                }
                            };  // same as above
                        } else {
                            self.message = "Please selected a valid file or folder.".to_string();
                        }
                    } else {
                        self.message = "No file or folder selected.".to_string();
                    }
                }
                ui.label(&self.message);
            });
        });

    }
}


fn main() {
    // functions::ffmpeg_process already contained file processing

    let options = eframe::NativeOptions{
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 400.0]),
            ..Default::default()
    };
    eframe::run_native(
        "Audio Loudness Limiter",
        options,
        Box::new(|_cc| Ok(Box::new(MainWindow::default()))),
    ).unwrap();
}

