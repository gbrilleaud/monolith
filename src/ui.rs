use crate::auth::AuthMode;
use crate::cache::load_cache;
use crate::catalog_query::{
    filter_and_sort_catalog_rows, CatalogRow, CatalogSort, CatalogSortColumn, CatalogViewMode,
    SortDirection,
};
use crate::client_auth::{AuthState, ClientAuth};
use crate::client_inventory::{ClientInventory, InventoryRefreshState};
use crate::client_rom_download::{ClientRomDownload, RomDownloadState};
use crate::client_rom_upload::{ClientRomUpload, RomUploadState};
use crate::cover::cover_uri;
use crate::db::Database;
use crate::emulator_launcher::EmulatorLauncher;
use crate::local_rom::launch_local_rom;
use crate::models::{GameMetadata, LaunchAvailability, ScanRoot, UserOverride};
use crate::navigation::{AppView, Navigator};
use crate::retroarch_installer::{
    RetroArchInstallState, RetroArchInstaller, XENIA_CANARY_WINDOWS_X64_ARCHIVE_URL,
};
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
    emulator_launcher: EmulatorLauncher,
    nav: Navigator,
    search: String,
    catalog_view: CatalogViewMode,
    catalog_sort: CatalogSort,
    edit_game_id: Option<i64>,
    edit_description: String,
    edit_cover: String,
    cover_picker: Option<Receiver<Option<PathBuf>>>,
    notice: Option<String>,
    database_path: PathBuf,
    library_roots: Vec<ScanRoot>,
    inventory: Option<ClientInventory>,
    rom_download: ClientRomDownload,
    downloading_game_id: Option<i64>,
    rom_upload: ClientRomUpload,
    rom_upload_picker: Option<Receiver<Option<Vec<PathBuf>>>>,
    rom_upload_system_id: Option<i64>,
    cache_path: PathBuf,
    imported_catalog_user_id: Option<i64>,
    username: String,
    password: String,
    bearer_token: String,
    use_sso: bool,
    show_association_backoffice: bool,
    association_search: String,
    selected_rom_path: Option<String>,
    client_config_path: PathBuf,
    show_connection_settings: bool,
    backend_url_draft: String,
    retroarch_installer: RetroArchInstaller,
}

impl MonolithApp {
    pub fn new(
        db: Database,
        auth: ClientAuth,
        emulator_launcher: EmulatorLauncher,
        database_path: PathBuf,
        library_roots: Vec<ScanRoot>,
        cache_path: PathBuf,
        client_config_path: PathBuf,
    ) -> Self {
        let backend_url_draft = auth.backend_url().into();
        Self {
            db,
            auth,
            emulator_launcher,
            nav: Navigator::default(),
            search: String::new(),
            catalog_view: CatalogViewMode::Tiles,
            catalog_sort: CatalogSort::new(CatalogSortColumn::Title, SortDirection::Ascending),
            edit_game_id: None,
            edit_description: String::new(),
            edit_cover: String::new(),
            cover_picker: None,
            notice: None,
            database_path,
            library_roots,
            inventory: None,
            rom_download: ClientRomDownload::new(),
            downloading_game_id: None,
            rom_upload: ClientRomUpload::new(),
            rom_upload_picker: None,
            rom_upload_system_id: None,
            cache_path,
            imported_catalog_user_id: None,
            username: String::new(),
            password: String::new(),
            bearer_token: String::new(),
            use_sso: false,
            show_association_backoffice: false,
            association_search: String::new(),
            selected_rom_path: None,
            client_config_path,
            show_connection_settings: false,
            backend_url_draft,
            retroarch_installer: RetroArchInstaller::new(),
        }
    }

    fn render_connection_settings(&mut self, context: &egui::Context) {
        let mut open = self.show_connection_settings;
        let mut apply = false;
        egui::Window::new("Connexion au serveur")
            .open(&mut open)
            .resizable(false)
            .show(context, |ui| {
                ui.label("URL du backend Monolith");
                ui.add(
                    egui::TextEdit::singleline(&mut self.backend_url_draft)
                        .hint_text("http://192.168.1.39:8788")
                        .desired_width(360.0),
                );
                ui.small(
                    "Utilisez http:// ou https://. La modification déconnecte la session active.",
                );
                ui.small(format!(
                    "Configuration enregistrée localement : {}",
                    self.client_config_path.display()
                ));
                ui.add_space(8.0);
                apply = ui.button("Enregistrer et tester").clicked();
            });
        self.show_connection_settings = open;
        if apply {
            match crate::client_config::ClientConfig::new(&self.backend_url_draft).and_then(
                |config| {
                    config.save(&self.client_config_path)?;
                    self.auth.reconfigure_backend(&config.backend_url)?;
                    Ok(config)
                },
            ) {
                Ok(config) => {
                    self.backend_url_draft = config.backend_url;
                    self.notice =
                        Some("Serveur enregistré ; vérification de disponibilité en cours".into());
                    self.show_connection_settings = false;
                }
                Err(error) => {
                    self.notice = Some(format!("Configuration du serveur refusée : {error}"));
                }
            }
        }
    }

    fn install_retroarch(&mut self) {
        let data_directory = match self.database_path.parent() {
            Some(directory) => directory,
            None => {
                self.notice = Some("Dossier de données client introuvable".into());
                return;
            }
        };
        let configured_root = std::env::var_os("MONOLITH_INSTALL_ROOT")
            .map(PathBuf::from)
            .or_else(|| {
                crate::client_config::ClientConfig::load_or_default(&self.client_config_path)
                    .ok()
                    .and_then(|config| config.install_root.map(PathBuf::from))
            });
        let install_root = configured_root.unwrap_or_else(|| {
            data_directory
                .parent()
                .filter(|parent| parent.is_absolute())
                .map(PathBuf::from)
                .or_else(|| std::env::current_dir().ok())
                .unwrap_or_else(|| data_directory.to_path_buf())
        });
        match self.retroarch_installer.start(&install_root) {
            Ok(()) => {
                self.notice = Some(format!(
                    "Téléchargement de RetroArch vers {}…",
                    install_root.join("tools").join("retroarch").display()
                ));
            }
            Err(error) => self.notice = Some(format!("Installation RetroArch refusée : {error}")),
        }
    }

    fn install_pcsx2(&mut self) {
        let data_directory = match self.database_path.parent() {
            Some(directory) => directory,
            None => {
                self.notice = Some("Dossier de données client introuvable".into());
                return;
            }
        };
        let configured_root = std::env::var_os("MONOLITH_INSTALL_ROOT")
            .map(PathBuf::from)
            .or_else(|| {
                crate::client_config::ClientConfig::load_or_default(&self.client_config_path)
                    .ok()
                    .and_then(|config| config.install_root.map(PathBuf::from))
            });
        let install_root = configured_root.unwrap_or_else(|| {
            data_directory
                .parent()
                .filter(|parent| parent.is_absolute())
                .map(PathBuf::from)
                .or_else(|| std::env::current_dir().ok())
                .unwrap_or_else(|| data_directory.to_path_buf())
        });
        match self.retroarch_installer.start_pcsx2(&install_root) {
            Ok(()) => {
                self.notice = Some(format!(
                    "Téléchargement de PCSX2 vers {}…",
                    install_root.join("tools").join("pcsx2").display()
                ))
            }
            Err(error) => self.notice = Some(format!("Installation PCSX2 refusée : {error}")),
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

    fn start_rom_download(&mut self, game: &GameMetadata) -> Result<(), String> {
        let session = self
            .auth
            .authenticated_session()
            .ok_or_else(|| "connexion au backend requise".to_owned())?;
        if !session.role.can_write() {
            return Err("compte non autorisé à télécharger des ROMs".into());
        }
        let data_directory = self
            .database_path
            .parent()
            .ok_or_else(|| "dossier de données client introuvable".to_owned())?;
        self.rom_download
            .start(
                session.backend_url.clone(),
                session.access_token.clone(),
                game.game_id,
                data_directory.join("roms").join(game.system_id.to_string()),
            )
            .map_err(|error| error.to_string())?;
        self.downloading_game_id = Some(game.game_id);
        Ok(())
    }

    fn begin_rom_upload_picker(&mut self, system_id: i64) {
        if !self.can_scan_library() {
            self.notice =
                Some("L’import nécessite un compte standard ou administrateur connecté".into());
            return;
        }
        if self.rom_upload_picker.is_some()
            || matches!(self.rom_upload.state(), RomUploadState::Uploading)
        {
            return;
        }
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let selection = rfd::FileDialog::new()
                .set_title("Importer une ou plusieurs ROMs")
                .pick_files();
            let _ = sender.send(selection);
        });
        self.rom_upload_system_id = Some(system_id);
        self.rom_upload_picker = Some(receiver);
    }

    fn poll_rom_upload_picker(&mut self) {
        let Some(receiver) = &self.rom_upload_picker else {
            return;
        };
        let selection = match receiver.try_recv() {
            Ok(selection) => selection,
            Err(TryRecvError::Disconnected) => {
                self.notice = Some("Le sélecteur de ROM s’est interrompu".into());
                self.rom_upload_picker = None;
                self.rom_upload_system_id = None;
                return;
            }
            Err(TryRecvError::Empty) => return,
        };
        self.rom_upload_picker = None;
        let Some(source) = selection else {
            self.rom_upload_system_id = None;
            return;
        };
        let result = self
            .auth
            .authenticated_session()
            .ok_or_else(|| "connexion au backend requise".to_owned())
            .and_then(|session| {
                let system_id = self
                    .rom_upload_system_id
                    .take()
                    .ok_or_else(|| "console d’import absente".to_owned())?;
                self.rom_upload
                    .start(
                        session.backend_url.clone(),
                        session.access_token.clone(),
                        system_id,
                        source,
                    )
                    .map_err(|error| error.to_string())
            });
        if let Err(error) = result {
            self.notice = Some(format!("Import ROM refusé : {error}"));
        } else {
            self.notice = Some("Import ROM et vérification SHA-256 en cours…".into());
        }
    }

    fn poll_rom_upload(&mut self) {
        let Some(state) = self.rom_upload.poll().cloned() else {
            return;
        };
        match state {
            RomUploadState::Completed(upload) => {
                self.notice = Some(format!(
                    "ROM reçue dans l’inbox : {} · SHA-256 {}",
                    upload.file_name, upload.sha256
                ))
            }
            RomUploadState::Error(error) => {
                self.notice = Some(format!("Import ROM échoué : {error}"))
            }
            _ => {}
        }
    }

    fn launch_game(&mut self, game: &GameMetadata) -> Result<(), String> {
        let path = game
            .launch_availability
            .preferred_path
            .as_deref()
            .ok_or_else(|| "ROM locale introuvable".to_owned())?;
        self.emulator_launcher
            .launch(game.system_id, std::path::Path::new(path))
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    fn poll_rom_download(&mut self) {
        let Some(state) = self.rom_download.poll().cloned() else {
            return;
        };
        let game_id = self.downloading_game_id.take();
        match state {
            RomDownloadState::Completed(download) => {
                let result = game_id
                    .ok_or_else(|| anyhow::anyhow!("jeu téléchargé absent"))
                    .and_then(|game_id| {
                        self.db.record_downloaded_rom(
                            game_id,
                            &download.path,
                            download.size_bytes,
                            &download.sha256,
                        )
                    });
                match result {
                    Ok(()) => {
                        self.notice = Some(format!(
                            "ROM téléchargée et vérifiée : {}",
                            download.path.display()
                        ));
                    }
                    Err(error) => {
                        self.notice = Some(format!(
                            "ROM téléchargée mais inventaire local non mis à jour : {error}"
                        ));
                    }
                }
            }
            RomDownloadState::Error(error) => {
                self.notice = Some(format!("Téléchargement ROM échoué : {error}"));
            }
            _ => {}
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
        self.poll_rom_upload_picker();
        self.poll_rom_upload();
        self.poll_inventory_scan();
        self.poll_rom_download();
        if let Some(state) = self.retroarch_installer.poll() {
            self.notice = Some(match state {
                RetroArchInstallState::InstallerStarted(directory) => {
                    format!("Installeur RetroArch lancé pour {}", directory.display())
                }
                RetroArchInstallState::Error(error) => {
                    format!("Installation RetroArch échouée : {error}")
                }
                _ => String::new(),
            });
        }
        self.auth.poll();
        self.render_connection_settings(context);
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
            || self.rom_upload_picker.is_some()
            || matches!(self.rom_upload.state(), RomUploadState::Uploading)
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
                if ui.button("Paramètres serveur").clicked() {
                    self.backend_url_draft = self.auth.backend_url().into();
                    self.show_connection_settings = true;
                }
                if ui.button("Installer RetroArch").clicked() {
                    self.install_retroarch();
                }
                if ui.button("Installer PCSX2 2.8.0").clicked() {
                    self.install_pcsx2();
                }
                if ui.button("Télécharger Xenia Canary").clicked() {
                    ui.ctx()
                        .open_url(egui::OpenUrl::new_tab(XENIA_CANARY_WINDOWS_X64_ARCHIVE_URL));
                }
                if matches!(
                    self.retroarch_installer.state(),
                    RetroArchInstallState::Downloading
                ) {
                    ui.spinner();
                    ui.label("RetroArch…");
                }
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
            if ui.button("Paramètres serveur").clicked() {
                self.backend_url_draft = self.auth.backend_url().into();
                self.show_connection_settings = true;
            }
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
            ui.add(
                egui::TextEdit::singleline(&mut self.search)
                    .hint_text("Rechercher (* et ? acceptés)…"),
            );
            ui.selectable_value(&mut self.catalog_view, CatalogViewMode::Tiles, "Jaquettes");
            ui.selectable_value(&mut self.catalog_view, CatalogViewMode::Details, "Détails");
            let uploading = matches!(self.rom_upload.state(), RomUploadState::Uploading);
            if ui
                .add_enabled(
                    self.can_scan_library() && !uploading && self.rom_upload_picker.is_none(),
                    egui::Button::new("Importer une ROM"),
                )
                .clicked()
            {
                self.begin_rom_upload_picker(system_id);
            }
            if uploading {
                ui.spinner();
                ui.label("Import…");
            }
        });
        let Some(user_id) = self.active_user_id() else {
            return;
        };
        let games = match self.db.resolved_games(user_id) {
            Ok(games) => games,
            Err(error) => {
                ui.colored_label(egui::Color32::RED, format!("Catalogue : {error}"));
                return;
            }
        };
        let mut rows = games
            .into_iter()
            .filter(|game| game.system_id == system_id)
            .map(catalog_row_for_game)
            .collect::<Vec<_>>();
        rows.extend(
            self.db
                .unlinked_rom_locations()
                .unwrap_or_default()
                .into_iter()
                .filter(|location| location.system_id == system_id)
                .map(catalog_row_for_local_rom),
        );
        let rows = filter_and_sort_catalog_rows(&rows, &self.search, self.catalog_sort);
        match self.catalog_view {
            CatalogViewMode::Tiles => self.render_catalog_tiles(ui, &rows),
            CatalogViewMode::Details => self.render_catalog_details(ui, &rows),
        }
    }

    fn render_catalog_tiles(&mut self, ui: &mut egui::Ui, rows: &[CatalogRow]) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for row in rows {
                    ui.group(|ui| {
                        ui.set_width(180.0);
                        if let Some(response) =
                            render_cover(ui, row.cover_art.as_deref(), egui::vec2(168.0, 126.0))
                        {
                            if response.interact(egui::Sense::click()).clicked() {
                                if let Some(game_id) = row.game_id {
                                    self.nav.open_details(game_id);
                                }
                            }
                        }
                        ui.label(&row.title);
                        if let Some(game_id) = row.game_id {
                            if ui.button("Ouvrir").clicked() {
                                self.nav.open_details(game_id);
                            }
                        } else {
                            ui.label("ROM locale inconnue");
                            if ui.button("Lancer localement").clicked() {
                                self.launch_unknown_local_row(row);
                            }
                        }
                    });
                }
            });
        });
    }

    fn render_catalog_details(&mut self, ui: &mut egui::Ui, rows: &[CatalogRow]) {
        let columns = [
            ("Titre", CatalogSortColumn::Title),
            ("Fichier", CatalogSortColumn::FileName),
            ("Favori", CatalogSortColumn::Favourite),
            ("Ajout", CatalogSortColumn::AddedAt),
            ("Taille", CatalogSortColumn::SizeBytes),
            ("Extension", CatalogSortColumn::Extension),
            ("Disponible", CatalogSortColumn::Availability),
            ("Note", CatalogSortColumn::Rating),
            ("Chemin", CatalogSortColumn::Path),
        ];
        egui::ScrollArea::both().show(ui, |ui| {
            egui::Grid::new("catalog_details")
                .striped(true)
                .show(ui, |ui| {
                    for (label, column) in columns {
                        let marker = if self.catalog_sort.column == column {
                            match self.catalog_sort.direction {
                                SortDirection::Ascending => " ↑",
                                SortDirection::Descending => " ↓",
                            }
                        } else {
                            ""
                        };
                        if ui.button(format!("{label}{marker}")).clicked() {
                            self.catalog_sort.direction = if self.catalog_sort.column == column
                                && self.catalog_sort.direction == SortDirection::Ascending
                            {
                                SortDirection::Descending
                            } else {
                                SortDirection::Ascending
                            };
                            self.catalog_sort.column = column;
                        }
                    }
                    ui.label("");
                    ui.end_row();
                    for row in rows {
                        ui.label(&row.title);
                        ui.label(&row.file_name);
                        ui.label(if row.favourite { "★" } else { "" });
                        ui.label(&row.added_at);
                        ui.label(row.size_bytes.to_string());
                        ui.label(&row.extension);
                        ui.label(if row.available { "Oui" } else { "Non" });
                        ui.label(
                            row.rating
                                .map(|rating| rating.to_string())
                                .unwrap_or_default(),
                        );
                        ui.label(&row.path);
                        if let Some(game_id) = row.game_id {
                            if ui.button("Ouvrir").clicked() {
                                self.nav.open_details(game_id);
                            }
                        } else if ui.button("Lancer").clicked() {
                            self.launch_unknown_local_row(row);
                        }
                        ui.end_row();
                    }
                });
        });
    }

    fn launch_unknown_local_row(&mut self, row: &CatalogRow) {
        let location = crate::models::RomLocation {
            id: None,
            game_id: None,
            system_id: row.system_id,
            path: row.path.clone(),
            extension: row.extension.clone(),
            size_bytes: row.size_bytes,
            modified_at: None,
            sha256: None,
            availability: if row.available {
                crate::models::RomAvailability::Available
            } else {
                crate::models::RomAvailability::Missing
            },
            last_seen_at: row.added_at.clone(),
        };
        if let Err(error) = launch_local_rom(&self.emulator_launcher, &location) {
            self.notice = Some(format!("Lancement local refusé : {error}"));
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
                if game.launch_availability.available && ui.button("Lancer").clicked() {
                    if let Err(error) = self.launch_game(&game) {
                        self.notice = Some(format!("Lancement refusé : {error}"));
                    }
                }
                if !game.launch_availability.available {
                    let downloading =
                        matches!(self.rom_download.state(), RomDownloadState::Downloading);
                    if ui
                        .add_enabled(
                            self.can_edit() && !downloading,
                            egui::Button::new("Télécharger la ROM"),
                        )
                        .clicked()
                    {
                        if let Err(error) = self.start_rom_download(&game) {
                            self.notice = Some(format!("Téléchargement ROM refusé : {error}"));
                        }
                    }
                    if downloading {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Téléchargement et vérification SHA-256 en cours…");
                        });
                    }
                }
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

fn catalog_row_for_game(game: GameMetadata) -> CatalogRow {
    let path = game
        .launch_availability
        .preferred_path
        .clone()
        .unwrap_or_default();
    let file_name = std::path::Path::new(&path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&game.title)
        .to_owned();
    let extension = std::path::Path::new(&path)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    CatalogRow {
        system_id: game.system_id,
        title: game.title,
        file_name,
        path,
        extension,
        favourite: false,
        added_at: String::new(),
        size_bytes: 0,
        rating: None,
        available: game.launch_availability.available,
        cover_art: game.cover_art,
        game_id: Some(game.game_id),
    }
}

fn catalog_row_for_local_rom(location: crate::models::RomLocation) -> CatalogRow {
    let file_name = std::path::Path::new(&location.path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&location.path)
        .to_owned();
    let title = std::path::Path::new(&file_name)
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or(&file_name)
        .to_owned();
    CatalogRow {
        system_id: location.system_id,
        title,
        file_name,
        path: location.path,
        extension: location.extension,
        favourite: false,
        added_at: location.last_seen_at,
        size_bytes: location.size_bytes,
        rating: None,
        available: location.availability == crate::models::RomAvailability::Available,
        cover_art: None,
        game_id: location.game_id,
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
