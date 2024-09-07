use eframe::egui;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

struct FileManager {
    current_dir: PathBuf,
    entries: Vec<FileEntry>,
}

struct FileEntry {
    name: String,
    is_dir: bool,
    size: u64,
    modified: String,
}

impl Default for FileManager {
    fn default() -> Self {
        Self {
            current_dir: std::env::current_dir().unwrap(),
            entries: Vec::new(),
        }
    }
}

impl FileManager {
    fn read_dir(&mut self) {
        self.entries.clear();
        if let Ok(entries) = fs::read_dir(&self.current_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let file_type = entry.file_type().unwrap();
                    let name = entry.file_name().into_string().unwrap();
                    self.entries.push(FileEntry {
                        name,
                        is_dir: file_type.is_dir(),
                        size: if file_type.is_dir() {
                            fs::read_dir(entry.path())
                                .map(|entries| entries.count())
                                .unwrap_or(0) as u64
                        } else {
                            entry.metadata().map(|m| m.len()).unwrap_or(0)
                        },
                        modified: entry
                            .metadata()
                            .and_then(|m| m.modified())
                            .map(|t| {
                                let datetime: chrono::DateTime<chrono::Local> = t.into();
                                let now = chrono::Local::now();
                                let today = now.date_naive();
                                let yesterday = today.pred_opt();

                                if datetime.date_naive() == today {
                                    format!("Today at {}", datetime.format("%H:%M"))
                                } else if Some(datetime.date_naive()) == yesterday {
                                    format!("Yesterday at {}", datetime.format("%H:%M"))
                                } else {
                                    datetime.format("%d/%m/%Y at %H:%M").to_string()
                                }
                            })
                            .unwrap_or_else(|_| String::from("Unknown")),
                    });
                }
            }
            self.entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir));
        }
    }

    fn can_navigate_up(&self) -> bool {
        self.current_dir.parent().is_some()
    }
}

impl eframe::App for FileManager {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                let current_dir_clone = self.current_dir.clone();
                let mut path_so_far = PathBuf::new();
                for (i, component) in current_dir_clone.components().enumerate() {
                    if let Some(component_str) = component.as_os_str().to_str() {
                        if i > 0 && i != 1 {
                            ui.label("/");
                        }
                        path_so_far.push(component_str);
                        let btn = ui.selectable_label(false, component_str);
                        if btn.clicked() {
                            self.current_dir = path_so_far.clone();
                            self.read_dir();
                        }
                    }
                }
            });
            ui.separator();

            // Navigate to the parent directory
            if self.can_navigate_up() && ui.button("Up").clicked() {
                if let Some(parent) = self.current_dir.parent() {
                    self.current_dir = parent.to_path_buf();
                    self.read_dir();
                }
            }

            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    egui::Grid::new("file_manager_grid")
                        .striped(true)
                        .min_col_width(ui.available_width() / 3.0)
                        .show(ui, |ui| {
                            ui.label("Name");
                            ui.label("Size");
                            ui.label("Modified");
                            ui.end_row();

                            let mut clicked_dir: Option<PathBuf> = None;

                            for entry in &self.entries {
                                let icon = if entry.is_dir { "📁" } else { "📄" };
                                let label = format!("{} {}", icon, entry.name);
                                let nme_lbl = ui.selectable_label(false, label);

                                if nme_lbl.clicked() {
                                    if entry.is_dir {
                                        clicked_dir = Some(self.current_dir.join(&entry.name));
                                    }
                                }

                                if nme_lbl.double_clicked() {
                                    if !entry.is_dir {
                                        println!("Opening file: {}", entry.name);
                                        let result = open_file(&self.current_dir, &entry.name);
                                        if let Err(e) = result {
                                            eprintln!("Failed to open file: {}", e);
                                        }
                                    }
                                }

                                // File or directory size
                                if entry.is_dir {
                                    ui.label(format!("{} items", entry.size));
                                } else {
                                    ui.label(format!("{}", format_file_size(entry.size)));
                                }

                                // Modified date
                                ui.label(&entry.modified);
                                ui.end_row();
                            }

                            if let Some(new_dir) = clicked_dir {
                                self.current_dir = new_dir;
                                self.read_dir();
                            }
                        });
                });
        });

        if self.entries.is_empty() {
            self.read_dir();
        }
    }
}

fn format_file_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if size < KB {
        format!("{} bytes", size)
    } else if size < MB {
        format!("{:.2} KiB", size as f64 / KB as f64)
    } else if size < GB {
        format!("{:.2} MiB", size as f64 / MB as f64)
    } else if size < TB {
        format!("{:.2} GiB", size as f64 / GB as f64)
    } else {
        format!("{:.2} TiB", size as f64 / TB as f64)
    }
}

fn open_file(current_dir: &PathBuf, file_name: &str) -> std::io::Result<()> {
    let file_path = current_dir.join(file_name);

    #[cfg(target_os = "windows")]
    let result = Command::new("notepad").arg(file_path).spawn();

    #[cfg(target_os = "macos")]
    let result = Command::new("open").arg("-t").arg(file_path).spawn();

    #[cfg(target_os = "linux")]
    let result = Command::new("xdg-open")
        .arg(file_path.to_str().unwrap())
        .spawn();

    result.map(|_| ())
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([600.0, 400.0]),
        ..Default::default()
    };
    eframe::run_native(
        "File Manager",
        options,
        Box::new(|_cc| Ok(Box::<FileManager>::default())),
    )
}
