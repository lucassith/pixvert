use super::Fetchable;
use async_trait::async_trait;
use reqwest::{Url};
use crate::fetcher::{FetchedObject, FetchError};
use actix_web::http::header;
use crate::cache::{Cachable, CacheError};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

pub struct HttpFetcher {
    reqwest: reqwest::Client,
    cache: Arc<Mutex<dyn Cachable<FetchedObject> + Send + Sync>>,
}

impl HttpFetcher {
    pub fn new(cache: Arc<Mutex<dyn Cachable<FetchedObject> + Send + Sync>>) -> HttpFetcher {
        HttpFetcher {
            reqwest: reqwest::Client::new(),
            cache,
        }
    }

    async fn fetch_with_meta(&self, link: &String, cached_meta: &HashMap<String, String>) -> Result<reqwest::Response, reqwest::Error> {
        let mut request_builder = self.reqwest.get(link);
        for header in vec![header::ETAG, header::LAST_MODIFIED] {
            match cached_meta.get(header.as_str()) {
                Some(value) => {
                    request_builder = request_builder.header(&header, value);
                }
                _ => (),
            }
        }
        request_builder.send().await
    }

    fn construct_hash(link: &String) -> String {
        return String::from("HTTP-Fetcher-Cache-") + link.as_str();
    }
}

impl From<reqwest::Error> for FetchError {
    fn from(err: reqwest::Error) -> Self {
        FetchError::FetchFailed(
            format!(
                "Error occurred while fetching data. {:#}", err
            )
        )
    }
}

#[async_trait]
impl Fetchable for HttpFetcher {
    fn can_fetch(&self, link: &String) -> bool {
        Url::parse(link).is_ok()
    }

    async fn fetch(&self, link: &String) -> Result<FetchedObject, FetchError> {
        let cached_object: Result<FetchedObject, CacheError>;
        {
            cached_object = self.cache.lock().unwrap().get(&HttpFetcher::construct_hash(link));
        }

        let response: reqwest::Response = match &cached_object {
            Ok(cached_object) => {
                self.fetch_with_meta(link, &cached_object.cache_info).await?
            }
            Err(_) => {
                self.fetch_with_meta(link, &HashMap::new()).await?
            }
        };

        if response.status() == actix_web::http::StatusCode::NOT_MODIFIED {
            return Result::Ok(cached_object.unwrap());
        }

        if !response.status().is_success() {
            return Result::Err(FetchError::FetchFailed(
                format!("Failed to fetch object. URL {}, Code: {}", link.clone(), response.status().to_string())
            ));
        }
        let mut fetched_object = FetchedObject::default();
        for header in vec![header::ETAG, header::LAST_MODIFIED] {
            match response.headers().get(&header) {
                Some(etag) => {
                    fetched_object.cache_info.insert(header.to_string(), etag.to_str().unwrap().to_string());
                }
                _ => (),
            }
        }
        match response.headers().get(header::CONTENT_TYPE) {
            Some(content_type) => {
                fetched_object.mime = content_type.to_str().unwrap().parse().unwrap();
            }
            None => {
                fetched_object.mime = mime::APPLICATION_OCTET_STREAM;
            }
        }
        fetched_object.bytes = response.bytes().await?;
        {
            self.cache.lock().as_mut().unwrap().set(
                HttpFetcher::construct_hash(link),
                fetched_object.clone(),
            ).unwrap();
        }

        return Result::Ok(fetched_object);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::MockServer;
    use httpmock::Method::GET;
    use crate::cache::CacheError;
    use crate::fetcher::{Fetchable, FetchedObject};
    use std::sync::{Mutex};

    struct CachableMock {
        pub hashmap: Arc<Mutex<HashMap<String, FetchedObject>>>
    }

    impl Cachable<FetchedObject> for CachableMock {
        fn get(&self, link: &String) -> Result<FetchedObject, CacheError> {
            if link.contains("nocache") {
                return Result::Err(CacheError::NoCacheEntry)
            }
            if link.contains("etag") {
                let mut fetched_object = FetchedObject::default();
                fetched_object.cache_info.insert(actix_web::http::header::ETAG.to_string(), "existing".to_string());
                fetched_object.bytes = bytes::Bytes::from("cached-object-etag");
                return Result::Ok(fetched_object);
            }
            return Result::Err(CacheError::NoCacheEntry)
        }

        fn set(&mut self, link: String, object: FetchedObject) -> Result<bool, CacheError> {
            self.hashmap.lock().as_deref_mut().unwrap().insert(link, object);
            return Result::Ok(true);
        }

        fn delete(&mut self, _: &String) -> bool {
            todo!()
        }

        fn count(&self) -> usize {
            return self.hashmap.lock().unwrap().len()
        }
    }

    #[actix_rt::test]
    async fn test_fetch_image() {
        let server = MockServer::start();
        let url = server.url("/nocache.png");

        server.mock(|when, then| {
            when.method(GET)
                .path("/nocache.png");
            then.status(200)
                .header(actix_web::http::header::CONTENT_TYPE.to_string(), mime::IMAGE_JPEG.to_string())
                .body(String::from("jpg"));
        });
        let hashmap = Arc::new(Mutex::new(HashMap::new()));
        let cache_mock = CachableMock{
            hashmap: hashmap.clone()
        };
        let cache: Arc<Mutex<dyn Cachable<FetchedObject> + Sync + Send>> = Arc::new(Mutex::new(
            cache_mock
        ));
        let fetcher = HttpFetcher::new(
            cache.clone(),
        );

        assert_eq!(hashmap.clone().lock().unwrap().len(), 0);

        let image = fetcher.fetch(&url).await.unwrap();

        assert_eq!(image.bytes, bytes::Bytes::from("jpg"));
        assert_eq!(image.mime, mime::IMAGE_JPEG);
        assert_eq!(image.cache_info.len(), 0);
        assert_eq!(hashmap.clone().lock().unwrap().len(), 1);
        assert_eq!(
            hashmap.clone().lock().as_deref().unwrap().into_iter().nth(0).unwrap().0,
            &HttpFetcher::construct_hash(&url.to_string())
        );
    }

    #[actix_rt::test]
    async fn test_try_to_fetch_if_cache_found() {
        let server = MockServer::start();
        let url = server.url("/cached-etag.png");

        server.mock(|when, then| {
            when.method(GET)
                .header_exists("ETAG".to_string())
                .path("/cached-etag.png");
            then.status(304)
                .header(actix_web::http::header::CONTENT_TYPE.to_string(), mime::IMAGE_JPEG.to_string())
                .body(String::from("new body"));
        });
        let cache_mock = CachableMock{
            hashmap: Arc::new(Mutex::new(HashMap::new())),
        };
        let cache: Arc<Mutex<dyn Cachable<FetchedObject> + Sync + Send>> = Arc::new(Mutex::new(
            cache_mock
        ));
        let fetcher = HttpFetcher::new(
            cache.clone(),
        );
        let image = fetcher.fetch(&url).await.unwrap();

        assert_eq!(image.bytes, bytes::Bytes::from("cached-object-etag"));
    }

    #[actix_rt::test]
    async fn test_try_to_fetch_if_cache_found_but_newer_exists() {
        let server = MockServer::start();
        let url = server.url("/cached-etag.png");

        server.mock(|when, then| {
            when.method(GET)
                .header_exists("ETAG".to_string())
                .path("/cached-etag.png");
            then.status(200)
                .header(actix_web::http::header::CONTENT_TYPE.to_string(), mime::IMAGE_JPEG.to_string())
                .body(String::from("new body"));
        });
        let cache_mock = CachableMock{
            hashmap: Arc::new(Mutex::new(HashMap::new())),
        };
        let cache: Arc<Mutex<dyn Cachable<FetchedObject> + Sync + Send>> = Arc::new(Mutex::new(
            cache_mock
        ));
        let fetcher = HttpFetcher::new(
            cache.clone(),
        );
        let image = fetcher.fetch(&url).await.unwrap();

        assert_eq!(image.bytes, bytes::Bytes::from("new body"));
    }
}