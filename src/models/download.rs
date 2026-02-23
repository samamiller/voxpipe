use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::mpsc::Sender;

#[derive(Debug)]
pub enum DownloadEvent {
    Progress { downloaded: u64, total: Option<u64> },
    Done,
    Error(String),
}

pub fn download_model(
    url: &str,
    target_path: &Path,
    sender: Sender<DownloadEvent>,
) -> std::thread::JoinHandle<()> {
    let url = url.to_string();
    let target_path = target_path.to_path_buf();
    std::thread::spawn(move || {
        if target_path.exists() {
            let _ = sender.send(DownloadEvent::Error(
                "Model already exists; delete it before downloading again.".to_string(),
            ));
            return;
        }

        let Some(parent) = target_path.parent() else {
            let _ = sender.send(DownloadEvent::Error(
                "Invalid target path for model download.".to_string(),
            ));
            return;
        };

        let partial_dir = parent.join(".partial");
        if let Err(err) = fs::create_dir_all(&partial_dir) {
            let _ = sender.send(DownloadEvent::Error(format!(
                "Failed to create download directory: {err}"
            )));
            return;
        }

        let file_name = target_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("model.bin");
        let partial_path = partial_dir.join(format!("{file_name}.part"));

        let response = match ureq::get(&url).call() {
            Ok(response) => response,
            Err(err) => {
                let _ = sender.send(DownloadEvent::Error(format!(
                    "Download failed: {err}"
                )));
                return;
            }
        };

        if response.status() >= 400 {
            let _ = sender.send(DownloadEvent::Error(format!(
                "Download failed with status {}",
                response.status()
            )));
            return;
        }

        let total = response
            .header("Content-Length")
            .and_then(|value| value.parse::<u64>().ok());

        let mut reader = response.into_reader();
        let mut file = match File::create(&partial_path) {
            Ok(file) => file,
            Err(err) => {
                let _ = sender.send(DownloadEvent::Error(format!(
                    "Failed to create download file: {err}"
                )));
                return;
            }
        };

        let mut buf = [0u8; 64 * 1024];
        let mut downloaded = 0u64;
        loop {
            let read = match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(err) => {
                    let _ = sender.send(DownloadEvent::Error(format!(
                        "Download interrupted: {err}"
                    )));
                    return;
                }
            };
            if let Err(err) = file.write_all(&buf[..read]) {
                let _ = sender.send(DownloadEvent::Error(format!(
                    "Failed writing download: {err}"
                )));
                return;
            }
            downloaded += read as u64;
            let _ = sender.send(DownloadEvent::Progress { downloaded, total });
        }

        if let Err(err) = file.sync_all() {
            let _ = sender.send(DownloadEvent::Error(format!(
                "Failed to sync download: {err}"
            )));
            return;
        }

        if let Err(err) = fs::rename(&partial_path, &target_path) {
            let _ = sender.send(DownloadEvent::Error(format!(
                "Failed to finalize download: {err}"
            )));
            return;
        }

        let _ = sender.send(DownloadEvent::Done);
    })
}
