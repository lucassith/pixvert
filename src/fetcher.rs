use async_trait::async_trait;
use bytes::Bytes;
use std::{sync::Arc};
use std::collections::HashMap;

pub mod http_fetcher;

#[derive(Debug)]
pub enum FetchError {
    ObjectNotFound,
    FetchFailed(String),
}

#[async_trait]
pub trait Fetchable {
    fn can_fetch(&self, link: &String) -> bool;
    async fn fetch(&self, link: &String) -> Result<FetchedObject, FetchError>;
}

pub struct FetcherProvider {
    fetchers: Vec<Arc<Box<dyn Fetchable + Sync + Send>>>
}

impl FetcherProvider {
    pub fn new(fetchers: Vec<Arc<Box<dyn Fetchable + Sync + Send>>>) -> FetcherProvider {
        FetcherProvider{
            fetchers
        }
    }

    pub fn get(&self, link: &String) -> Option<Arc<Box<dyn Fetchable + Sync + Send>>> {
        for fetcher in self.fetchers.iter() {
            if fetcher.can_fetch(link) {
                return Option::Some(fetcher.clone());
            }
        }
        Option::None
    }
}


#[derive(Debug, Clone)]
pub struct FetchedObject {
    pub bytes: Bytes,
    pub mime: mime::Mime,
    pub cache_info: HashMap<String, String>,
}

impl Default for FetchedObject {
    fn default() -> Self {
        FetchedObject{
            bytes: Bytes::default(),
            mime: mime::APPLICATION_OCTET_STREAM,
            cache_info: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use crate::cache::memory_cache::MemoryCache;

    #[test]
    fn test_index_not_ok() {
        let http_fetcher: Box<dyn Fetchable + Sync + Send> = Box::new(
            http_fetcher::HttpFetcher::new(Arc::new(Mutex::new(MemoryCache::new())))
        );
        let fetchers: Vec<Arc<Box<dyn Fetchable + Sync + Send>>> = Vec::from([
            Arc::from(http_fetcher)
        ]);
        let fetcher_provider = FetcherProvider::new(
            fetchers
        );

        assert!(fetcher_provider.get(&"https://valid.com".to_string()).is_some());
        assert!(fetcher_provider.get(&"invalid".to_string()).is_none());
    }
}