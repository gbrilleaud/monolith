use crate::{
    auth::AuthMode,
    backend_client::BackendClient,
    models::UserOverride,
    session_store::{ClientSession, SessionStore},
};
use anyhow::{bail, Context, Result};
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthState {
    SignedOut,
    Authenticating,
    Authenticated(ClientSession),
    Offline { user_id: i64 },
    Error(String),
}

type AuthResult = std::result::Result<ClientSession, String>;
pub type OverrideSyncResult = std::result::Result<(), String>;

pub struct ClientAuth {
    backend_url: String,
    store: SessionStore,
    cache_path: PathBuf,
    state: AuthState,
    receiver: Option<Receiver<AuthResult>>,
    override_receiver: Option<Receiver<OverrideSyncResult>>,
    policy_receiver: Option<Receiver<std::result::Result<AuthMode, String>>>,
    auth_mode: Option<AuthMode>,
    policy_error: Option<String>,
}

impl ClientAuth {
    pub fn new(
        backend_url: impl Into<String>,
        store: SessionStore,
        cache_path: PathBuf,
        now: i64,
    ) -> Result<Self> {
        let state = store
            .load_valid_at(now)?
            .map(AuthState::Authenticated)
            .unwrap_or(AuthState::SignedOut);
        Ok(Self {
            backend_url: backend_url.into(),
            store,
            cache_path,
            state,
            receiver: None,
            override_receiver: None,
            policy_receiver: None,
            auth_mode: None,
            policy_error: None,
        })
    }

    pub fn state(&self) -> &AuthState {
        &self.state
    }

    pub fn authenticated_session(&self) -> Option<&ClientSession> {
        match &self.state {
            AuthState::Authenticated(session) => Some(session),
            _ => None,
        }
    }

    pub fn auth_mode(&self) -> Option<AuthMode> {
        self.auth_mode
    }

    pub fn policy_error(&self) -> Option<&str> {
        self.policy_error.as_deref()
    }

    pub fn probe_policy(&mut self) -> Result<()> {
        if self.policy_receiver.is_some() {
            bail!("détection déjà en cours");
        }
        let backend_url = self.backend_url.clone();
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let result = (|| -> Result<AuthMode> {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .context("initialisation réseau")?;
                runtime.block_on(async move {
                    Ok(BackendClient::new(backend_url)?.health().await?.auth_mode)
                })
            })()
            .map_err(|error| format!("{error:#}"));
            let _ = sender.send(result);
        });
        self.policy_receiver = Some(receiver);
        self.policy_error = None;
        Ok(())
    }

    pub fn begin_local_login(&mut self, username: &str, password: &str) -> Result<()> {
        let username = username.trim();
        if username.is_empty() || password.is_empty() {
            bail!("identifiant et mot de passe requis");
        }
        let backend_url = self.backend_url.clone();
        let cache_path = self.cache_path.clone();
        let store = self.store.clone();
        let username = username.to_owned();
        let password = password.to_owned();
        self.start(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .context("initialisation réseau")?;
            runtime.block_on(async move {
                let client = BackendClient::new(&backend_url)?;
                let login = client.login_local(&username, &password).await?;
                client
                    .refresh_offline_cache(&login.access_token, &cache_path)
                    .await?;
                let session = ClientSession {
                    backend_url,
                    access_token: login.access_token,
                    user_id: login.user_id,
                    username: login.username,
                    role: login.role,
                    expires_at: login.expires_at,
                };
                store.save(&session)?;
                Ok(session)
            })
        })
    }

    pub fn begin_bearer_login(&mut self, token: &str) -> Result<()> {
        let token = token.trim();
        if token.is_empty() {
            bail!("jeton SSO requis");
        }
        let backend_url = self.backend_url.clone();
        let cache_path = self.cache_path.clone();
        let store = self.store.clone();
        let token = token.to_owned();
        self.start(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .context("initialisation réseau")?;
            runtime.block_on(async move {
                let client = BackendClient::new(&backend_url)?;
                let identity = client.identity(&token).await?;
                client.refresh_offline_cache(&token, &cache_path).await?;
                let session = ClientSession {
                    backend_url,
                    access_token: token,
                    user_id: identity.user_id,
                    username: identity.username,
                    role: identity.role,
                    expires_at: identity
                        .expires_at
                        .context("jeton sans date d’expiration")?,
                };
                store.save(&session)?;
                Ok(session)
            })
        })
    }

    pub fn logout(&mut self) -> Result<()> {
        self.store.clear()?;
        self.receiver = None;
        self.override_receiver = None;
        self.state = AuthState::SignedOut;
        Ok(())
    }

    pub fn begin_override_sync(&mut self, value: UserOverride) -> Result<()> {
        if self.override_receiver.is_some() {
            bail!("publication d’une surcharge déjà en cours");
        }
        let AuthState::Authenticated(session) = &self.state else {
            bail!("publication distante impossible hors connexion");
        };
        if !session.role.can_write() {
            bail!("compte en lecture seule");
        }
        if value.user_id != session.user_id {
            bail!("la surcharge ne correspond pas à la session active");
        }

        let backend_url = session.backend_url.clone();
        let token = session.access_token.clone();
        let cache_path = self.cache_path.clone();
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let result = (|| -> Result<()> {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .context("initialisation réseau")?;
                runtime.block_on(async move {
                    let client = BackendClient::new(backend_url)?;
                    client.save_override(&token, &value).await?;
                    client.refresh_offline_cache(&token, &cache_path).await?;
                    Ok(())
                })
            })()
            .map_err(|error| format!("{error:#}"));
            let _ = sender.send(result);
        });
        self.override_receiver = Some(receiver);
        Ok(())
    }

    pub fn override_sync_in_progress(&self) -> bool {
        self.override_receiver.is_some()
    }

    pub fn poll_override_sync(&mut self) -> Option<OverrideSyncResult> {
        let receiver = self.override_receiver.as_ref()?;
        match receiver.try_recv() {
            Ok(result) => {
                self.override_receiver = None;
                Some(result)
            }
            Err(TryRecvError::Disconnected) => {
                self.override_receiver = None;
                Some(Err("publication distante interrompue".into()))
            }
            Err(TryRecvError::Empty) => None,
        }
    }

    pub fn continue_offline(&mut self, user_id: i64) {
        self.receiver = None;
        self.state = AuthState::Offline { user_id };
    }

    pub fn reset_error(&mut self) {
        if matches!(self.state, AuthState::Error(_)) {
            self.state = AuthState::SignedOut;
        }
    }

    pub fn poll(&mut self) {
        self.poll_policy();
        let Some(receiver) = &self.receiver else {
            return;
        };
        match receiver.try_recv() {
            Ok(Ok(session)) => {
                self.state = AuthState::Authenticated(session);
                self.receiver = None;
            }
            Ok(Err(error)) => {
                self.state = AuthState::Error(error);
                self.receiver = None;
            }
            Err(TryRecvError::Disconnected) => {
                self.state = AuthState::Error("tâche d’authentification interrompue".into());
                self.receiver = None;
            }
            Err(TryRecvError::Empty) => {}
        }
    }

    fn poll_policy(&mut self) {
        let Some(receiver) = &self.policy_receiver else {
            return;
        };
        match receiver.try_recv() {
            Ok(Ok(mode)) => {
                self.auth_mode = Some(mode);
                self.policy_error = None;
                self.policy_receiver = None;
            }
            Ok(Err(error)) => {
                self.policy_error = Some(error);
                self.policy_receiver = None;
            }
            Err(TryRecvError::Disconnected) => {
                self.policy_error = Some("détection interrompue".into());
                self.policy_receiver = None;
            }
            Err(TryRecvError::Empty) => {}
        }
    }

    fn start<F>(&mut self, operation: F) -> Result<()>
    where
        F: FnOnce() -> Result<ClientSession> + Send + 'static,
    {
        if self.receiver.is_some() {
            bail!("authentification déjà en cours");
        }
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let result = operation().map_err(|error| format!("{error:#}"));
            let _ = sender.send(result);
        });
        self.receiver = Some(receiver);
        self.state = AuthState::Authenticating;
        Ok(())
    }
}
