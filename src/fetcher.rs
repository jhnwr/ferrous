use crate::zyte::ZyteClient;
use anyhow::Result;

#[cfg(feature = "wreq")]
use crate::direct::DirectClient;

pub enum Fetcher {
    Zyte(ZyteClient),
    #[cfg(feature = "wreq")]
    Direct(DirectClient),
}

impl Fetcher {
    pub async fn fetch(&self, url: &str) -> Result<String> {
        match self {
            Fetcher::Zyte(c) => c.fetch(url).await,
            #[cfg(feature = "wreq")]
            Fetcher::Direct(c) => c.fetch(url).await,
        }
    }
}
