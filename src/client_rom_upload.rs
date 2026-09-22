use crate::{
    backend_client::{BackendClient, RomUpload},
    rom_bundle::{build_rom_bundle, BundleKind},
};
use anyhow::Result;
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum RomUploadState {
    #[default]
    Idle,
    Uploading,
    Completed(RomUpload),
    Error(String),
}

pub struct ClientRomUpload {
    receiver: Option<Receiver<std::result::Result<RomUpload, String>>>,
    state: RomUploadState,
}

impl Default for ClientRomUpload {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientRomUpload {
    pub fn new() -> Self {
        Self {
            receiver: None,
            state: RomUploadState::Idle,
        }
    }

    pub fn state(&self) -> &RomUploadState {
        &self.state
    }

    pub fn start(
        &mut self,
        backend_url: String,
        bearer_token: String,
        system_id: i64,
        sources: Vec<PathBuf>,
    ) -> Result<()> {
        if sources.is_empty() {
            anyhow::bail!("aucune ROM sélectionnée");
        }
        if self.receiver.is_some() {
            anyhow::bail!("import ROM déjà en cours");
        }
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let result = upload(&backend_url, &bearer_token, system_id, &sources)
                .map_err(|error| format!("{error:#}"));
            let _ = sender.send(result);
        });
        self.receiver = Some(receiver);
        self.state = RomUploadState::Uploading;
        Ok(())
    }

    pub fn poll(&mut self) -> Option<&RomUploadState> {
        let receiver = self.receiver.as_ref()?;
        match receiver.try_recv() {
            Ok(Ok(upload)) => {
                self.receiver = None;
                self.state = RomUploadState::Completed(upload);
                Some(&self.state)
            }
            Ok(Err(error)) => {
                self.receiver = None;
                self.state = RomUploadState::Error(error);
                Some(&self.state)
            }
            Err(TryRecvError::Disconnected) => {
                self.receiver = None;
                self.state = RomUploadState::Error("import ROM interrompu".into());
                Some(&self.state)
            }
            Err(TryRecvError::Empty) => None,
        }
    }
}

fn upload(
    backend_url: &str,
    bearer_token: &str,
    system_id: i64,
    sources: &[PathBuf],
) -> Result<RomUpload> {
    let bundle = build_rom_bundle(sources)?;
    let runtime = tokio::runtime::Runtime::new()?;
    let result = runtime.block_on(async {
        let client = BackendClient::new(backend_url)?;
        match bundle.kind {
            BundleKind::SingleFile => {
                client
                    .upload_rom(bearer_token, system_id, &bundle.path)
                    .await
            }
            BundleKind::ZipArchive => {
                client
                    .upload_rom_bundle(bearer_token, system_id, &bundle)
                    .await
            }
        }
    });
    bundle.cleanup()?;
    result
}
