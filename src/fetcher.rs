use crate::zyte::ZyteClient;

#[cfg(feature = "wreq")]
use crate::direct::DirectClient;

#[derive(Debug)]
pub enum FetchError {
    HttpError { status: u16, url: String },
    NetworkError { error: String, url: String },
    ParseError { error: String, url: String },
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchError::HttpError { status, url } => write!(f, "HTTP {status} for {url}"),
            FetchError::NetworkError { error, url } => write!(f, "network error for {url}: {error}"),
            FetchError::ParseError { error, url } => write!(f, "parse error for {url}: {error}"),
        }
    }
}

impl std::error::Error for FetchError {}

pub enum Fetcher {
    Zyte(ZyteClient),
    #[cfg(feature = "wreq")]
    Direct(DirectClient),
}

impl Fetcher {
    pub async fn fetch(&self, url: &str) -> Result<String, FetchError> {
        match self {
            Fetcher::Zyte(c) => c.fetch(url).await,
            #[cfg(feature = "wreq")]
            Fetcher::Direct(c) => c.fetch(url).await,
        }
    }
}
