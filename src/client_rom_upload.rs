use crate::backend_client::{BackendClient, RomUpload};
use anyhow::Result;
use std::{
    path::Path,
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
        source: impl AsRef<Path>,
    ) -> Result<()> {
        if self.receiver.is_some() {
            anyhow::bail!("import ROM déjà en cours");
        }
        let source = source.as_ref().to_path_buf();
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let result = upload(&backend_url, &bearer_token, system_id, &source)
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
    source: &Path,
) -> Result<RomUpload> {
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async {
        BackendClient::new(backend_url)?
            .upload_rom(bearer_token, system_id, source)
            .await
    })
}
