#![windows_subsystem = "windows"]
mod functions;
use eframe::egui;
use rfd::FileDialog;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::string::ToString;
use which;

#[derive(Default)]
struct ProcessingState {
    message: String,
    terminal_output: String,
    progress: f32,
    done: bool,
}

struct MainWindow {
    ffmpeg_path: String,
    selected_path: Option<String>,
    limitation: String, // to check if input is empty, we need to use String here
    message: String,
    file_node_type: String,
    terminal_output: String, // New field to store terminal outputs
    progress: f32,
    is_processing: bool,
    processing_state: Option<Arc<Mutex<ProcessingState>>>,
}

impl Default for MainWindow {
    fn default() -> Self {
        Self {
            ffmpeg_path: "Empty for ffmpeg in PATH".to_string(),
            selected_path: None,
            limitation: "-14".to_string(), // default value
            message: String::new(),
            file_node_type: String::new(),
            terminal_output: String::new(), // Initialize the new field
            progress: 0.0,
            is_processing: false,
            processing_state: None,
        }
    }
}

impl eframe::App for MainWindow {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(processing_state) = &self.processing_state {
            if let Ok(state) = processing_state.lock() {
                self.message = state.message.clone();
                self.terminal_output = state.terminal_output.clone();
                self.progress = state.progress;
                if state.done {
                    self.is_processing = false;
                }
            }
        }
        if self.is_processing {
            ctx.request_repaint_after(Duration::from_millis(100));
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            // row: select ffmpeg binary
            ui.horizontal(|ui| {
                // ffmpeg binary selection button
                if ui.add_enabled(!self.is_processing, egui::Button::new("Select ffmpeg binary")).clicked() {
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
                if ui.add_enabled(!self.is_processing, egui::Button::new("Select file")).clicked() {
                    if let Some(path) = FileDialog::new().pick_file() {
                        self.selected_path = Some(path.display().to_string());
                        self.file_node_type = "file".to_string();
                    }
                }
                // folder selection button
                if ui.add_enabled(!self.is_processing, egui::Button::new("Select folder")).clicked() {
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
                if ui.add_enabled(!self.is_processing, egui::Button::new("Start processing")).clicked() {
                    let processing_state = Arc::new(Mutex::new(ProcessingState {
                        message: "Starting processing...".to_string(),
                        terminal_output: "=== Processing Started ===\n".to_string(),
                        progress: 0.0,
                        done: false,
                    }));
                    self.processing_state = Some(processing_state.clone());
                    self.is_processing = true;
                    self.progress = 0.0;

                    let selected_path = self.selected_path.clone();
                    let file_node_type = self.file_node_type.clone();
                    let limitation = self.limitation.clone();
                    let ffmpeg_path = self.ffmpeg_path.clone();

                    thread::spawn(move || {
                        let update_state = |message: String, terminal_output: String, progress: f32, done: bool| {
                            if let Ok(mut state) = processing_state.lock() {
                                state.message = message;
                                state.terminal_output = terminal_output;
                                state.progress = progress;
                                state.done = done;
                            }
                        };

                        let limitation = if limitation.is_empty() { "-14".to_string() } else { limitation };
                        let limitation = match limitation.parse::<f32>() {
                            Ok(value) => value,
                            Err(err) => {
                                update_state(
                                    format!("Invalid input for loudness limitation. Please enter a valid number. {}", err),
                                    format!("=== Processing Started ===\nERROR: Invalid input for loudness limitation. Please enter a valid number. {}\n", err),
                                    0.0,
                                    true,
                                );
                                return;
                            }
                        };

                        let ffmpeg_path = if ffmpeg_path.is_empty() || ffmpeg_path == "Empty for ffmpeg in PATH" {
                            match which::which("ffmpeg") {
                                Ok(path) => path.display().to_string(),
                                Err(_) => {
                                    update_state(
                                        "Please select ffmpeg binary.".to_string(),
                                        "=== Processing Started ===\nERROR: Please select ffmpeg binary.\n".to_string(),
                                        0.0,
                                        true,
                                    );
                                    return;
                                }
                            }
                        } else {
                            ffmpeg_path
                        };

                        let Some(path) = selected_path else {
                            update_state(
                                "No file or folder selected.".to_string(),
                                "=== Processing Started ===\nERROR: No file or folder selected.\n".to_string(),
                                0.0,
                                true,
                            );
                            return;
                        };

                        if file_node_type == "file" {
                            let mut terminal_output = format!("=== Processing Started ===\nProcessing file: {}\n", path);
                            match functions::ffmpeg_process(&path, &ffmpeg_path, limitation, &mut terminal_output) {
                                Ok(val) => {
                                    terminal_output.push_str(&format!("SUCCESS: Processed {}\n", val));
                                    update_state(format!("Success: {}", val), terminal_output, 1.0, true);
                                }
                                Err(err) => {
                                    terminal_output.push_str(&format!("ERROR: {}\n", err));
                                    update_state(format!("Error: {}", err), terminal_output, 1.0, true);
                                }
                            }
                        } else if file_node_type == "folder" {
                            let mut terminal_output = format!("=== Processing Started ===\nProcessing folder: {}\n", path);
                            let message = format!("Processing folder: {}", path);
                            update_state(message.clone(), terminal_output.clone(), 0.0, false);
                            match functions::ffmpeg_process_dir(
                                &path,
                                &ffmpeg_path,
                                limitation,
                                &mut terminal_output,
                                |progress, status, output| {
                                    update_state(status.to_string(), output.to_string(), progress, false);
                                },
                            ) {
                                Ok(val) => {
                                    terminal_output.push_str(&format!("SUCCESS: Processed folder {}\n", val));
                                    update_state(format!("Success: {}", val), terminal_output, 1.0, true);
                                }
                                Err(err) => {
                                    terminal_output.push_str(&format!("ERROR: {}\n", err));
                                    update_state(format!("Error: {}", err), terminal_output, 1.0, true);
                                }
                            }
                        } else {
                            update_state(
                                "Please selected a valid file or folder.".to_string(),
                                "=== Processing Started ===\nERROR: Please selected a valid file or folder.\n".to_string(),
                                0.0,
                                true,
                            );
                        }
                    });
                }
                ui.label(&self.message);
            });

            let progress_width = (ui.available_width() - 8.0).max(0.0);
            ui.add_sized(
                [progress_width, 20.0],
                egui::ProgressBar::new(self.progress).show_percentage(),
            );

            // Terminal output display box
            ui.separator();
            ui.label("Terminal Output:");
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.terminal_output)
                            .desired_width(f32::INFINITY)
                            .desired_rows(10)
                            .interactive(false)
                            .code_editor()
                    );
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
