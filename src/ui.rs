use crate::db::Database;
use crate::models::UserOverride;
use crate::navigation::{AppView, Navigator};
use crate::sync::SyncEngine;
use eframe::egui;
use std::path::PathBuf;

pub struct MonolithApp {
    db: Database,
    user_id: i64,
    nav: Navigator,
    search: String,
    edit_game_id: Option<i64>,
    edit_description: String,
    edit_cover: String,
    notice: Option<String>,
    cache_path: PathBuf,
}

impl MonolithApp {
    pub fn new(db: Database, user_id: i64, cache_path: PathBuf) -> Self {
        Self {
            db,
            user_id,
            nav: Navigator::default(),
            search: String::new(),
            edit_game_id: None,
            edit_description: String::new(),
            edit_cover: String::new(),
            notice: None,
            cache_path,
        }
    }

    fn begin_edit(&mut self, game_id: i64) {
        if let Ok(Some(game)) = self.db.resolved_game(self.user_id, game_id) {
            self.edit_game_id = Some(game_id);
            self.edit_description = game.description;
            self.edit_cover = game.cover_art.unwrap_or_default();
        }
    }

    fn save_edit(&mut self, game_id: i64) {
        let override_value = UserOverride {
            user_id: self.user_id,
            game_id,
            description: non_empty(&self.edit_description),
            cover_art: non_empty(&self.edit_cover),
        };
        self.notice = Some(
            match self.db.save_override(&override_value).and_then(|_| {
                SyncEngine::new(&self.db, self.user_id, &self.cache_path)
                    .refresh_local_cache()
                    .map(|_| ())
                    .map_err(|error| rusqlite::Error::ToSqlConversionFailure(error.into()))
            }) {
                Ok(()) => "Surcharge sauvegardée et cache actualisé".into(),
                Err(error) => format!("Échec de sauvegarde : {error}"),
            },
        );
        self.edit_game_id = None;
    }
}

impl eframe::App for MonolithApp {
    fn update(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("header").show(context, |ui| {
            ui.horizontal(|ui| {
                if ui.button("MONOLITH").clicked() {
                    self.nav.home();
                }
                if self.nav.current() != &AppView::Home && ui.button("← Retour").clicked() {
                    self.nav.back();
                }
                ui.separator();
                ui.label("Local-first · profil 1");
                if let Some(notice) = &self.notice {
                    ui.colored_label(egui::Color32::LIGHT_GREEN, notice);
                }
            });
        });

        let current = self.nav.current().clone();
        egui::CentralPanel::default().show(context, |ui| match current {
            AppView::Home => self.render_home(ui),
            AppView::Systems => self.render_systems(ui),
            AppView::Catalog { system_id } => self.render_catalog(ui, system_id),
            AppView::Details { game_id } => self.render_details(ui, game_id),
        });
    }
}

impl MonolithApp {
    fn render_home(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(80.0);
            ui.heading(egui::RichText::new("MONOLITH").size(42.0).strong());
            ui.label("Votre bibliothèque de jeux, disponible localement.");
            ui.add_space(30.0);
            if ui
                .add_sized([240.0, 52.0], egui::Button::new("PARCOURIR LES CONSOLES"))
                .clicked()
            {
                self.nav.open_systems();
            }
        });
    }

    fn render_systems(&mut self, ui: &mut egui::Ui) {
        ui.heading("Consoles");
        ui.add_space(12.0);
        match self.db.systems() {
            Ok(systems) if systems.is_empty() => {
                ui.label("Aucune console. Le catalogue attend sa première synchronisation.");
            }
            Ok(systems) => {
                ui.horizontal_wrapped(|ui| {
                    for system in systems {
                        let label = format!("{}\n{} jeu(x)", system.name, system.game_count);
                        if ui
                            .add_sized([180.0, 90.0], egui::Button::new(label))
                            .clicked()
                        {
                            self.nav.open_catalog(system.system_id);
                        }
                    }
                });
            }
            Err(error) => {
                ui.colored_label(egui::Color32::RED, format!("SQLite : {error}"));
            }
        }
    }

    fn render_catalog(&mut self, ui: &mut egui::Ui, system_id: i64) {
        ui.horizontal(|ui| {
            ui.heading("Catalogue");
            ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("Rechercher…"));
        });
        let needle = self.search.to_lowercase();
        match self.db.resolved_games(self.user_id) {
            Ok(games) => {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for game in games.into_iter().filter(|game| {
                            game.system_id == system_id
                                && game.title.to_lowercase().contains(&needle)
                        }) {
                            let cover = game.cover_art.as_deref().unwrap_or("jaquette absente");
                            let label = format!("{}\n{}", game.title, cover);
                            if ui
                                .add_sized([180.0, 120.0], egui::Button::new(label))
                                .clicked()
                            {
                                self.nav.open_details(game.game_id);
                            }
                        }
                    });
                });
            }
            Err(error) => {
                ui.colored_label(egui::Color32::RED, format!("Catalogue : {error}"));
            }
        }
    }

    fn render_details(&mut self, ui: &mut egui::Ui, game_id: i64) {
        match self.db.resolved_game(self.user_id, game_id) {
            Ok(Some(game)) => {
                ui.heading(&game.title);
                ui.label(format!("{} · langue {}", game.system_name, game.language));
                ui.separator();
                ui.label(&game.description);
                ui.label(format!(
                    "Jaquette : {}",
                    game.cover_art.as_deref().unwrap_or("absente")
                ));
                ui.add_space(16.0);
                if ui.button("Modifier mes métadonnées").clicked() {
                    self.begin_edit(game_id);
                }
                if self.edit_game_id == Some(game_id) {
                    ui.separator();
                    ui.label("Description personnalisée");
                    ui.add(egui::TextEdit::multiline(&mut self.edit_description).desired_rows(8));
                    ui.label("Chemin ou URL de la jaquette");
                    ui.text_edit_singleline(&mut self.edit_cover);
                    ui.horizontal(|ui| {
                        if ui.button("Sauvegarder").clicked() {
                            self.save_edit(game_id);
                        }
                        if ui.button("Annuler").clicked() {
                            self.edit_game_id = None;
                        }
                    });
                }
            }
            Ok(None) => {
                ui.label("Jeu introuvable.");
            }
            Err(error) => {
                ui.colored_label(egui::Color32::RED, format!("SQLite : {error}"));
            }
        }
    }
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}
