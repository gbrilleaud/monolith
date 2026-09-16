use crate::auth::AuthMode;
use crate::cache::load_cache;
use crate::client_auth::{AuthState, ClientAuth};
use crate::client_inventory::{ClientInventory, InventoryRefreshState};
use crate::cover::cover_uri;
use crate::db::Database;
use crate::models::{GameMetadata, LaunchAvailability, ScanRoot, UserOverride};
use crate::navigation::{AppView, Navigator};
use crate::sync::SyncEngine;
use eframe::egui;
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
};

pub struct MonolithApp {
    db: Database,
    auth: ClientAuth,
    nav: Navigator,
    search: String,
    edit_game_id: Option<i64>,
    edit_description: String,
    edit_cover: String,
    cover_picker: Option<Receiver<Option<PathBuf>>>,
    notice: Option<String>,
    database_path: PathBuf,
    library_roots: Vec<ScanRoot>,
    inventory: Option<ClientInventory>,
    cache_path: PathBuf,
    imported_catalog_user_id: Option<i64>,
    username: String,
    password: String,
    bearer_token: String,
    use_sso: bool,
    show_association_backoffice: bool,
    association_search: String,
    selected_rom_path: Option<String>,
}

impl MonolithApp {
    pub fn new(
        db: Database,
        auth: ClientAuth,
        database_path: PathBuf,
        library_roots: Vec<ScanRoot>,
        cache_path: PathBuf,
    ) -> Self {
        Self {
            db,
            auth,
            nav: Navigator::default(),
            search: String::new(),
            edit_game_id: None,
            edit_description: String::new(),
            edit_cover: String::new(),
            cover_picker: None,
            notice: None,
            database_path,
            library_roots,
            inventory: None,
            cache_path,
            imported_catalog_user_id: None,
            username: String::new(),
            password: String::new(),
            bearer_token: String::new(),
            use_sso: false,
            show_association_backoffice: false,
            association_search: String::new(),
            selected_rom_path: None,
        }
    }

    fn import_remote_cache_for_active_user(&mut self) {
        let Some(user_id) = self.active_user_id() else {
            return;
        };
        if self.imported_catalog_user_id == Some(user_id) {
            return;
        }
        self.imported_catalog_user_id = Some(user_id);
        match load_cache(&self.cache_path) {
            Ok(snapshot) if snapshot.user_id == user_id => {
                if let Err(error) = self.db.replace_catalog_snapshot(&snapshot) {
                    self.notice = Some(format!("Catalogue distant non importé : {error}"));
                }
            }
            Ok(_) => {
                self.notice = Some("Cache catalogue associé à un autre utilisateur".into());
            }
            Err(error) => {
                self.notice = Some(format!("Cache catalogue indisponible : {error}"));
            }
        }
    }

    fn refresh_cache_for_active_user(&mut self) -> Result<(), String> {
        let user_id = self
            .active_user_id()
            .ok_or_else(|| "session utilisateur absente".to_owned())?;
        SyncEngine::new(&self.db, user_id, &self.cache_path)
            .refresh_local_cache()
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    fn render_association_backoffice(&mut self, context: &egui::Context) {
        if !self.show_association_backoffice {
            return;
        }
        let mut open = true;
        egui::Window::new("Backoffice · Associations ROM")
            .open(&mut open)
            .resizable(true)
            .default_width(900.0)
            .show(context, |ui| {
                ui.label("Sélectionnez une ROM non associée, puis un jeu du même système.");
                ui.separator();
                let locations = match self.db.unlinked_rom_locations() {
                    Ok(locations) => locations,
                    Err(error) => {
                        ui.colored_label(egui::Color32::RED, format!("SQLite : {error}"));
                        return;
                    }
                };
                ui.columns(2, |columns| {
                    columns[0].heading("ROMs non associées");
                    egui::ScrollArea::vertical()
                        .max_height(420.0)
                        .show(&mut columns[0], |ui| {
                            for location in &locations {
                                let selected = self.selected_rom_path.as_deref() == Some(&location.path);
                                if ui
                                    .selectable_label(
                                        selected,
                                        format!("[{}] {}", location.system_id, location.path),
                                    )
                                    .clicked()
                                {
                                    self.selected_rom_path = Some(location.path.clone());
                                    self.association_search.clear();
                                }
                            }
                        });

                    columns[1].heading("Jeux compatibles");
                    let selected_rom = self
                        .selected_rom_path
                        .as_deref()
                        .and_then(|path| locations.iter().find(|location| location.path == path));
                    let Some(rom) = selected_rom else {
                        columns[1].label("Choisissez une ROM à gauche.");
                        return;
                    };
                    columns[1].label(format!("Système {} · {}", rom.system_id, rom.extension));
                    columns[1].add(
                        egui::TextEdit::singleline(&mut self.association_search)
                            .hint_text("Rechercher un jeu…"),
                    );
                    let games = match self.db.resolved_games(self.active_user_id().unwrap_or(1)) {
                        Ok(games) => association_candidates(games, rom.system_id, &self.association_search),
                        Err(error) => {
                            columns[1].colored_label(egui::Color32::RED, format!("SQLite : {error}"));
                            return;
                        }
                    };
                    egui::ScrollArea::vertical()
                        .max_height(340.0)
                        .show(&mut columns[1], |ui| {
                            for game in games {
                                if ui.button(format!("Associer · {}", game.title)).clicked() {
                                    match self.db.link_rom_location_to_game(&rom.path, game.game_id) {
                                        Ok(()) => match self.refresh_cache_for_active_user() {
                                            Ok(()) => {
                                                self.notice = Some(format!("ROM associée à {}", game.title));
                                                self.selected_rom_path = None;
                                                self.association_search.clear();
                                            }
                                            Err(error) => self.notice = Some(format!("Association enregistrée, cache non actualisé : {error}")),
                                        },
                                        Err(error) => self.notice = Some(format!("Association refusée : {error}")),
                                    }
                                }
                            }
                        });
                });
            });
        self.show_association_backoffice = open;
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
        let local_result = self.db.save_override(&override_value).and_then(|_| {
            SyncEngine::new(&self.db, user_id, &self.cache_path)
                .refresh_local_cache()
                .map(|_| ())
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(error.into()))
        });

        match local_result {
            Err(error) => {
                self.notice = Some(format!("Échec de sauvegarde locale : {error}"));
            }
            Ok(()) => {
                self.edit_game_id = None;
                if matches!(self.auth.state(), AuthState::Authenticated(_)) {
                    self.notice = Some(match self.auth.begin_override_sync(override_value) {
                        Ok(()) => {
                            "Surcharge sauvegardée localement · publication distante en cours"
                                .into()
                        }
                        Err(error) => format!(
                            "Surcharge conservée localement · publication non lancée : {error}"
                        ),
                    });
                } else {
                    self.notice = Some("Surcharge sauvegardée dans le cache hors ligne".into());
                }
            }
        }
    }

    fn begin_inventory_scan(&mut self) {
        let Some(user_id) = self.active_user_id() else {
            return;
        };
        if !self.can_scan_library() {
            self.notice =
                Some("Le scan nécessite un compte standard ou administrateur connecté".into());
            return;
        }
        if self.library_roots.is_empty() {
            self.notice = Some("Aucune racine de bibliothèque n’est configurée localement".into());
            return;
        }
        let mut inventory = ClientInventory::new(
            &self.database_path,
            self.library_roots.clone(),
            user_id,
            &self.cache_path,
        );
        match inventory.start() {
            Ok(()) => {
                self.inventory = Some(inventory);
                self.notice = Some("Scan de la bibliothèque en cours…".into());
            }
            Err(error) => self.notice = Some(format!("Scan non lancé : {error}")),
        }
    }

    fn poll_inventory_scan(&mut self) {
        let Some(inventory) = &mut self.inventory else {
            return;
        };
        if inventory.poll().is_none() {
            return;
        }
        self.notice = Some(match inventory.state() {
            InventoryRefreshState::Completed(report) => format!(
                "Bibliothèque actualisée : {} acceptés, {} absents, {} erreurs",
                report.accepted, report.missing, report.issues
            ),
            InventoryRefreshState::Error(error) => format!("Scan interrompu : {error}"),
            InventoryRefreshState::Idle | InventoryRefreshState::Scanning => return,
        });
    }

    fn begin_cover_picker(&mut self) {
        if self.cover_picker.is_some() {
            return;
        }
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let selection = rfd::FileDialog::new()
                .set_title("Choisir une jaquette")
                .add_filter("Images", &["png", "jpg", "jpeg", "webp"])
                .pick_file();
            let _ = sender.send(selection);
        });
        self.cover_picker = Some(receiver);
    }

    fn poll_cover_picker(&mut self) {
        let Some(receiver) = &self.cover_picker else {
            return;
        };
        match receiver.try_recv() {
            Ok(Some(path)) => {
                self.edit_cover = path.display().to_string();
                self.notice =
                    Some("Jaquette sélectionnée ; vérifiez l’aperçu puis sauvegardez".into());
                self.cover_picker = None;
            }
            Ok(None) => self.cover_picker = None,
            Err(TryRecvError::Disconnected) => {
                self.notice = Some("Le sélecteur de jaquette s’est interrompu".into());
                self.cover_picker = None;
            }
            Err(TryRecvError::Empty) => {}
        }
    }
}

impl eframe::App for MonolithApp {
    fn update(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_cover_picker();
        self.poll_inventory_scan();
        self.auth.poll();
        self.import_remote_cache_for_active_user();
        if let Some(result) = self.auth.poll_override_sync() {
            if result.is_ok() {
                self.imported_catalog_user_id = None;
            }
            self.notice = Some(match result {
                Ok(()) => "Surcharge publiée sur le backend et cache synchronisé".into(),
                Err(error) => format!(
                    "Publication distante échouée ; la version locale est conservée : {error}"
                ),
            });
        }
        if matches!(self.auth.state(), AuthState::Authenticating) || self.auth.auth_mode().is_none()
        {
            context.request_repaint_after(std::time::Duration::from_millis(100));
        }
        if matches!(self.auth.state(), AuthState::Authenticated(_)) {
            self.password.clear();
            self.bearer_token.clear();
        }
        let inventory_scanning = self
            .inventory
            .as_ref()
            .is_some_and(|inventory| inventory.state().is_scanning());
        if self.cover_picker.is_some()
            || self.auth.override_sync_in_progress()
            || inventory_scanning
        {
            context.request_repaint_after(std::time::Duration::from_millis(100));
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
                if self.can_manage_associations() && ui.button("Backoffice ROMs").clicked() {
                    self.show_association_backoffice = true;
                }
                if self.can_scan_library() {
                    let scanning = self
                        .inventory
                        .as_ref()
                        .is_some_and(|inventory| inventory.state().is_scanning());
                    if ui
                        .add_enabled(!scanning, egui::Button::new("Actualiser la bibliothèque"))
                        .clicked()
                    {
                        self.begin_inventory_scan();
                    }
                    if scanning {
                        ui.spinner();
                        ui.label("Scan…");
                    }
                }
                if self.auth.override_sync_in_progress() {
                    ui.spinner();
                    ui.label("Publication…");
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
            self.imported_catalog_user_id = None;
            self.nav.home();
            return;
        }
        self.render_association_backoffice(context);

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

    fn can_manage_associations(&self) -> bool {
        matches!(self.auth.state(), AuthState::Authenticated(session) if matches!(session.role, crate::auth::Role::Admin))
    }

    fn can_scan_library(&self) -> bool {
        matches!(self.auth.state(), AuthState::Authenticated(session) if session.role.can_write())
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
                            let mut open = false;
                            ui.group(|ui| {
                                ui.set_width(180.0);
                                if let Some(response) = render_cover(
                                    ui,
                                    game.cover_art.as_deref(),
                                    egui::vec2(168.0, 126.0),
                                ) {
                                    open |= response.interact(egui::Sense::click()).clicked();
                                }
                                open |= ui
                                    .add_sized([168.0, 38.0], egui::Button::new(&game.title))
                                    .clicked();
                                render_availability(ui, &game.launch_availability);
                            });
                            if open {
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
                render_availability(ui, &game.launch_availability);
                if self.can_manage_associations() {
                    ui.separator();
                    ui.label("ROMs associées");
                    match self.db.rom_locations() {
                        Ok(locations) => {
                            let linked = linked_locations_for_game(locations, game_id);
                            if linked.is_empty() {
                                ui.label("Aucune ROM associée.");
                            } else {
                                let mut unlink_path = None;
                                for location in linked {
                                    ui.horizontal(|ui| {
                                        ui.label(format!(
                                            "{} · {:?}",
                                            location.path, location.availability
                                        ));
                                        if ui.button("Désassocier").clicked() {
                                            unlink_path = Some(location.path.clone());
                                        }
                                    });
                                }
                                if let Some(path) = unlink_path {
                                    match self.db.unlink_rom_location(&path) {
                                        Ok(()) => match self.refresh_cache_for_active_user() {
                                            Ok(()) => {
                                                self.notice =
                                                    Some("Association ROM supprimée".into());
                                            }
                                            Err(error) => {
                                                self.notice = Some(format!(
                                                    "Association supprimée, cache non actualisé : {error}"
                                                ));
                                            }
                                        },
                                        Err(error) => {
                                            self.notice =
                                                Some(format!("Désassociation refusée : {error}"));
                                        }
                                    }
                                }
                            }
                        }
                        Err(error) => {
                            ui.colored_label(egui::Color32::RED, format!("SQLite : {error}"));
                        }
                    }
                }
                ui.separator();
                render_cover(ui, game.cover_art.as_deref(), egui::vec2(240.0, 320.0));
                ui.add_space(8.0);
                ui.label(&game.description);
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
                        if ui
                            .add_enabled(
                                self.cover_picker.is_none(),
                                egui::Button::new("Parcourir…"),
                            )
                            .clicked()
                        {
                            self.begin_cover_picker();
                        }
                        if self.cover_picker.is_some() {
                            ui.spinner();
                            ui.label("Sélecteur ouvert…");
                        }
                    });
                    ui.label("Aperçu");
                    render_cover(
                        ui,
                        non_empty(&self.edit_cover).as_deref(),
                        egui::vec2(180.0, 240.0),
                    );
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                !self.auth.override_sync_in_progress(),
                                egui::Button::new("Sauvegarder"),
                            )
                            .clicked()
                        {
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

pub fn linked_locations_for_game(
    locations: Vec<crate::models::RomLocation>,
    game_id: i64,
) -> Vec<crate::models::RomLocation> {
    locations
        .into_iter()
        .filter(|location| location.game_id == Some(game_id))
        .collect()
}

pub fn association_candidates(
    games: Vec<GameMetadata>,
    system_id: i64,
    search: &str,
) -> Vec<GameMetadata> {
    let needle = search.trim().to_lowercase();
    games
        .into_iter()
        .filter(|game| game.system_id == system_id && game.title.to_lowercase().contains(&needle))
        .collect()
}

pub fn availability_label(availability: &LaunchAvailability) -> String {
    if availability.available {
        format!(
            "Disponible · {} emplacement{}",
            availability.location_count,
            if availability.location_count > 1 {
                "s"
            } else {
                ""
            }
        )
    } else {
        "Absent de la bibliothèque".into()
    }
}

fn render_availability(ui: &mut egui::Ui, availability: &LaunchAvailability) {
    let color = if availability.available {
        egui::Color32::LIGHT_GREEN
    } else {
        egui::Color32::LIGHT_RED
    };
    ui.colored_label(color, availability_label(availability));
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

fn render_cover(
    ui: &mut egui::Ui,
    reference: Option<&str>,
    size: egui::Vec2,
) -> Option<egui::Response> {
    let Some(reference) = reference else {
        ui.label("Jaquette absente");
        return None;
    };
    match cover_uri(reference) {
        Ok(Some(uri)) => Some(ui.add(egui::Image::new(uri).fit_to_exact_size(size))),
        Ok(None) => {
            ui.label("Jaquette absente");
            None
        }
        Err(error) => {
            ui.colored_label(
                egui::Color32::LIGHT_RED,
                format!("Jaquette invalide : {error}"),
            );
            None
        }
    }
}
