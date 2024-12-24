use egui::{Context, RichText};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::sync::mpsc::{channel, Receiver, Sender};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    pub question: String,
    pub hint1: String,
    pub hint2: String,
    pub answer: String,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Show {
    question: bool,
    hint1: bool,
    hint2: bool,
    answer: bool,
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(Deserialize, Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct MyApp {
    pixels_per_point: f32,
    questions: Option<Vec<Question>>,
    question_nr: usize,
    prev_question_nr: usize,
    show: Show,
    #[serde(skip)]
    file_io: (Sender<String>, Receiver<String>),
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            pixels_per_point: 4.0,
            questions: None,
            question_nr: 0,
            prev_question_nr: 0,
            show: Default::default(),
            file_io: channel(),
        }
    }
}
impl MyApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            let mut app: Self = eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
            app.show = Default::default();
            return app;
        }
        Default::default()
    }
}

impl eframe::App for MyApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_pixels_per_point(self.pixels_per_point);
        if self.question_nr != self.prev_question_nr {
            self.prev_question_nr = self.question_nr;
            self.show = Default::default();
        }

        // Parsing questions from file picker
        if let Ok(quiz) = self.file_io.1.try_recv() {
            if let Ok(quiz) = serde_json::from_str::<Vec<Question>>(&quiz) {
                if quiz.len() >= 1 {
                    self.questions = Some(quiz);
                    self.question_nr = 0;
                }
            }
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                if ui.button("+").highlight().clicked() {
                    self.pixels_per_point += 0.1;
                }
                if ui.button("−").highlight().clicked() {
                    self.pixels_per_point = (self.pixels_per_point - 0.1).max(0.1);
                }

                ui.separator();
                if ui.button("Quiz öffnen").clicked() {
                    let ctx = ctx.clone();
                    let tx = self.file_io.0.clone();
                    file_dialog(tx, ctx); // opens the file dialog in a background thread
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let question = self.questions.as_ref().map(|q| q.get(self.question_nr));
            if let Some(Some(question)) = question {
                ui.horizontal(|ui| {
                    ui.label("Frage: ");
                    if ui.button("<<").clicked() {
                        self.question_nr = self.question_nr.saturating_sub(1);
                    }
                    if let Some(questions) = self.questions.as_ref() {
                        ui.add(
                            egui::widgets::DragValue::new(&mut self.question_nr)
                                .range(0..=questions.len()),
                        );
                    }
                    if ui.button(">>").clicked() {
                        self.question_nr = self.question_nr.saturating_add(1);
                    }
                });

                if ui.button("Frage: ").clicked() {
                    self.show.question ^= true;
                }
                match self.show.question {
                    true => ui.label(RichText::new(&question.question)),
                    false => ui.label(""),
                };

                if ui.button("Hinweis 1: ").clicked() {
                    self.show.hint1 ^= true;
                }
                match self.show.hint1 {
                    true => ui.label(&question.hint1),
                    false => ui.label(""),
                };

                if ui.button("Hinweis 2: ").clicked() {
                    self.show.hint2 ^= true;
                }
                match self.show.hint2 {
                    true => ui.label(&question.hint2),
                    false => ui.label(""),
                };

                if ui.button("Antwort: ").clicked() {
                    self.show.answer ^= true;
                }
                match self.show.answer {
                    true => ui.label(&question.answer),
                    false => ui.label(""),
                };
            };
        });
    }
}

fn file_dialog(tx: Sender<String>, ctx: Context) {
    let task = rfd::AsyncFileDialog::new().pick_file();
    execute(async move {
        let file = task.await;
        if let Some(file) = file {
            let data = file.read().await;
            if let Ok(text) = String::from_utf8(data) {
                let _ = tx.send(text);
                ctx.request_repaint();
            }
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn execute<F: Future<Output = ()> + Send + 'static>(f: F) {
    // this is stupid... use any executor of your choice instead
    std::thread::spawn(move || smol::block_on(f));
}

#[cfg(target_arch = "wasm32")]
fn execute<F: Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}
