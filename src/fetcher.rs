use async_trait::async_trait;
use bytes::Bytes;
use std::{borrow::Borrow, error::Error, sync::Arc};

pub mod http_fetcher;

#[async_trait]
pub trait Fetchable {
    fn can_fetch(&self, link: &String) -> bool;
    async fn fetch(&self, link: &String) -> Result<Bytes, Box<dyn Error>>;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_not_ok() {
        let http_fetcher: Box<dyn Fetchable + Sync + Send> = Box::new(http_fetcher::HttpFetcher::new());
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