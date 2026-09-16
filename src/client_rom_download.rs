use crate::backend_client::{BackendClient, RomDownload};
use anyhow::Result;
use std::{
    path::Path,
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum RomDownloadState {
    #[default]
    Idle,
    Downloading,
    Completed(RomDownload),
    Error(String),
}

pub struct ClientRomDownload {
    receiver: Option<Receiver<std::result::Result<RomDownload, String>>>,
    state: RomDownloadState,
}

impl Default for ClientRomDownload {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientRomDownload {
    pub fn new() -> Self {
        Self {
            receiver: None,
            state: RomDownloadState::Idle,
        }
    }

    pub fn state(&self) -> &RomDownloadState {
        &self.state
    }

    pub fn start(
        &mut self,
        backend_url: String,
        bearer_token: String,
        game_id: i64,
        destination_directory: impl AsRef<Path>,
    ) -> Result<()> {
        if self.receiver.is_some() {
            anyhow::bail!("téléchargement ROM déjà en cours");
        }
        let destination_directory = destination_directory.as_ref().to_path_buf();
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let result = download(&backend_url, &bearer_token, game_id, &destination_directory)
                .map_err(|error| format!("{error:#}"));
            let _ = sender.send(result);
        });
        self.receiver = Some(receiver);
        self.state = RomDownloadState::Downloading;
        Ok(())
    }

    pub fn poll(&mut self) -> Option<&RomDownloadState> {
        let receiver = self.receiver.as_ref()?;
        match receiver.try_recv() {
            Ok(Ok(download)) => {
                self.receiver = None;
                self.state = RomDownloadState::Completed(download);
                Some(&self.state)
            }
            Ok(Err(error)) => {
                self.receiver = None;
                self.state = RomDownloadState::Error(error);
                Some(&self.state)
            }
            Err(TryRecvError::Disconnected) => {
                self.receiver = None;
                self.state = RomDownloadState::Error("téléchargement ROM interrompu".into());
                Some(&self.state)
            }
            Err(TryRecvError::Empty) => None,
        }
    }
}

fn download(
    backend_url: &str,
    bearer_token: &str,
    game_id: i64,
    destination_directory: &Path,
) -> Result<RomDownload> {
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async {
        BackendClient::new(backend_url)?
            .download_game_rom(bearer_token, game_id, destination_directory)
            .await
    })
}
