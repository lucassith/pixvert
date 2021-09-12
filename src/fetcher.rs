use std::sync::Arc;
use std::collections::HashMap;

use async_trait::async_trait;
use bytes::Bytes;


use crate::service_provider::Service;

pub mod http_fetcher;

static FETCHED_TIME_CACHE_MAP_NAME: &str = "FETCHED_TIME_CACHE_MAP_NAME";

#[derive(Debug)]
pub enum FetchError {
    ObjectNotFound,
    FetchFailed(String),
}

pub trait FetchableService: Fetchable + Service {}

#[async_trait]
pub trait Fetchable {
    async fn fetch(&self, link: &String) -> Result<FetchedObject, FetchError>;
}

pub struct FetcherProvider {
    fetchers: Vec<Arc<Box<dyn Fetchable + Sync + Send>>>,
}


#[derive(Debug, Clone)]
pub struct FetchedObject {
    pub bytes: Bytes,
    pub mime: String,
    pub cache_info: HashMap<String, String>,
}

impl Default for FetchedObject {
    fn default() -> Self {
        FetchedObject {
            bytes: Bytes::default(),
            mime: mime::APPLICATION_OCTET_STREAM.to_string(),
            cache_info: HashMap::new(),
        }
    }
}
