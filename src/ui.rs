use crate::auth::AuthMode;
use crate::client_auth::{AuthState, ClientAuth};
use crate::db::Database;
use crate::models::UserOverride;
use crate::navigation::{AppView, Navigator};
use crate::sync::SyncEngine;
use eframe::egui;
use std::path::PathBuf;

pub struct MonolithApp {
    db: Database,
    auth: ClientAuth,
    nav: Navigator,
    search: String,
    edit_game_id: Option<i64>,
    edit_description: String,
    edit_cover: String,
    notice: Option<String>,
    cache_path: PathBuf,
    username: String,
    password: String,
    bearer_token: String,
    use_sso: bool,
}

impl MonolithApp {
    pub fn new(db: Database, auth: ClientAuth, cache_path: PathBuf) -> Self {
        Self {
            db,
            auth,
            nav: Navigator::default(),
            search: String::new(),
            edit_game_id: None,
            edit_description: String::new(),
            edit_cover: String::new(),
            notice: None,
            cache_path,
            username: String::new(),
            password: String::new(),
            bearer_token: String::new(),
            use_sso: false,
        }
    }

    fn begin_edit(&mut self, game_id: i64) {
        let Some(user_id) = self.active_user_id() else {
            return;
        };
        if let Ok(Some(game)) = self.db.resolved_game(user_id, game_id) {
            self.edit_game_id = Some(game_id);
            self.edit_description = game.description;
            self.edit_cover = game.cover_art.unwrap_or_default();
        }
    }

    fn save_edit(&mut self, game_id: i64) {
        let Some(user_id) = self.active_user_id() else {
            return;
        };
        let override_value = UserOverride {
            user_id,
            game_id,
            description: non_empty(&self.edit_description),
            cover_art: non_empty(&self.edit_cover),
        };
        self.notice = Some(
            match self.db.save_override(&override_value).and_then(|_| {
                SyncEngine::new(&self.db, user_id, &self.cache_path)
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
        self.auth.poll();
        if matches!(self.auth.state(), AuthState::Authenticating) || self.auth.auth_mode().is_none()
        {
            context.request_repaint_after(std::time::Duration::from_millis(100));
        }
        if matches!(self.auth.state(), AuthState::Authenticated(_)) {
            self.password.clear();
            self.bearer_token.clear();
        }

        if self.active_user_id().is_none() {
            egui::CentralPanel::default().show(context, |ui| self.render_login(ui));
            return;
        }

        let mut logout = false;
        egui::TopBottomPanel::top("header").show(context, |ui| {
            ui.horizontal(|ui| {
                if ui.button("MONOLITH").clicked() {
                    self.nav.home();
                }
                if self.nav.current() != &AppView::Home && ui.button("← Retour").clicked() {
                    self.nav.back();
                }
                ui.separator();
                match self.auth.state() {
                    AuthState::Authenticated(session) => {
                        ui.label(format!("{} · {:?}", session.username, session.role));
                        logout = ui.button("Déconnexion").clicked();
                    }
                    AuthState::Offline { user_id } => {
                        ui.label(format!("Hors ligne · profil {user_id}"));
                        logout = ui.button("Quitter le mode hors ligne").clicked();
                    }
                    _ => {}
                }
                if let Some(notice) = &self.notice {
                    ui.colored_label(egui::Color32::LIGHT_GREEN, notice);
                }
            });
        });
        if logout {
            if let Err(error) = self.auth.logout() {
                self.notice = Some(format!("Déconnexion incomplète : {error}"));
            }
            self.nav.home();
            return;
        }

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
    fn active_user_id(&self) -> Option<i64> {
        match self.auth.state() {
            AuthState::Authenticated(session) => Some(session.user_id),
            AuthState::Offline { user_id } => Some(*user_id),
            _ => None,
        }
    }

    fn can_edit(&self) -> bool {
        match self.auth.state() {
            AuthState::Authenticated(session) => session.role.can_write(),
            AuthState::Offline { .. } => true,
            _ => false,
        }
    }

    fn render_login(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(55.0);
            ui.heading(egui::RichText::new("MONOLITH").size(40.0).strong());
            ui.label("Connexion au catalogue · accès hors ligne toujours disponible");
            ui.add_space(24.0);

            if matches!(self.auth.state(), AuthState::Authenticating) {
                ui.spinner();
                ui.label("Authentification et synchronisation du cache…");
                return;
            }
            if let AuthState::Error(error) = self.auth.state() {
                ui.colored_label(egui::Color32::LIGHT_RED, error);
                ui.add_space(8.0);
            }

            let mode = self.auth.auth_mode();
            let allow_local = matches!(mode, Some(AuthMode::Local | AuthMode::Hybrid));
            let allow_sso = matches!(mode, Some(AuthMode::Sso | AuthMode::Hybrid));

            if mode.is_none() {
                ui.label("Détection de la politique d’authentification…");
                if let Some(error) = self.auth.policy_error() {
                    ui.colored_label(
                        egui::Color32::YELLOW,
                        format!("Backend indisponible : {error}"),
                    );
                    if ui.button("Réessayer").clicked() {
                        let _ = self.auth.probe_policy();
                    }
                }
            } else {
                if allow_local && allow_sso {
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.use_sso, false, "Compte local");
                        ui.selectable_value(&mut self.use_sso, true, "SSO / OIDC");
                    });
                    ui.add_space(8.0);
                } else {
                    self.use_sso = allow_sso;
                }

                if allow_local && !self.use_sso {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.username)
                            .hint_text("Nom d’utilisateur")
                            .desired_width(320.0),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut self.password)
                            .password(true)
                            .hint_text("Mot de passe")
                            .desired_width(320.0),
                    );
                    if ui.button("Connexion locale").clicked() {
                        if let Err(error) =
                            self.auth.begin_local_login(&self.username, &self.password)
                        {
                            self.notice = Some(error.to_string());
                        }
                    }
                } else if allow_sso {
                    ui.label("Collez un jeton OIDC fourni par votre portail SSO.");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.bearer_token)
                            .password(true)
                            .hint_text("Jeton Bearer")
                            .desired_width(420.0),
                    );
                    if ui.button("Connexion SSO").clicked() {
                        if let Err(error) = self.auth.begin_bearer_login(&self.bearer_token) {
                            self.notice = Some(error.to_string());
                        }
                    }
                }
            }

            ui.add_space(18.0);
            ui.separator();
            ui.label("Le mode hors ligne utilise le dernier catalogue local.");
            if ui.button("Continuer hors ligne").clicked() {
                self.password.clear();
                self.bearer_token.clear();
                self.auth.continue_offline(1);
            }
        });
    }

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
        let Some(user_id) = self.active_user_id() else {
            return;
        };
        match self.db.resolved_games(user_id) {
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
        let Some(user_id) = self.active_user_id() else {
            return;
        };
        match self.db.resolved_game(user_id, game_id) {
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
                if ui
                    .add_enabled(
                        self.can_edit(),
                        egui::Button::new("Modifier mes métadonnées"),
                    )
                    .clicked()
                {
                    self.begin_edit(game_id);
                }
                if !self.can_edit() {
                    ui.label("Compte en lecture seule.");
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
