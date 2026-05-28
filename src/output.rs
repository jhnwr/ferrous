use anyhow::Result;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

pub struct OutputWriter {
    file: Mutex<tokio::fs::File>,
}

impl OutputWriter {
    pub async fn new(path: &str) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await?;
        Ok(Self {
            file: Mutex::new(file),
        })
    }

    pub async fn write_item(&self, value: &serde_json::Value) -> Result<()> {
        let line = serde_json::to_string(value)?;
        let mut file = self.file.lock().await;
        file.write_all(line.as_bytes()).await?;
        file.write_all(b"\n").await?;
        Ok(())
    }
}
