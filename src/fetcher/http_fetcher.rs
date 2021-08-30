use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use actix_web::http::header;
use async_trait::async_trait;
use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;
use reqwest::Url;
use urlencoding::decode;

use crate::IMAGE_CACHE_HASH_LITERAL;
use crate::cache::{Cachable, CacheError};
use crate::fetcher::{FetchableService, FetchedObject, FetchError};
use crate::service_provider::Service;

use super::Fetchable;

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
        let client = reqwest::Client::new();
        let mut request_builder = client.get(link);
        if let Some(etag) = cached_meta.get(header::ETAG.as_str()) {
            request_builder = request_builder.header(header::IF_NONE_MATCH, etag);
        }
        if let Some(modified_date) = cached_meta.get(header::LAST_MODIFIED.as_str()) {
            request_builder = request_builder.header(header::IF_MODIFIED_SINCE, modified_date);
        }
        log::trace!("Trying to fetch resource with headers {:#?}", request_builder);
        request_builder.send().await
    }

    fn construct_hash(link: &String) -> String {
        return format!("{:x}", md5::compute(String::from("HTTP-Fetcher-Cache-") + link.as_str()));
    }

    fn decode_url(link: &String) -> String {
        String::from(&*decode(link).unwrap_or(Cow::from("")))
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

impl Service for HttpFetcher {
    fn can_be_used(&self, link: &String) -> bool {
        Url::parse(&HttpFetcher::decode_url(link)).is_ok()
    }
}

impl FetchableService for HttpFetcher {}

#[async_trait]
impl Fetchable for HttpFetcher {
    async fn fetch(&self, link: &String) -> Result<FetchedObject, FetchError> {
        let link = &HttpFetcher::decode_url(link);
        let cached_object: Result<FetchedObject, CacheError>;
        let hash = &HttpFetcher::construct_hash(link);
        {
            log::info!("Looking for hash {:#}", hash);
            cached_object = self.cache.lock().unwrap().get(&HttpFetcher::construct_hash(link));
        }

        let response: reqwest::Response = match &cached_object {
            Ok(cached_object) => {
                return Result::Ok(cached_object.clone());
                log::trace!("Found cached object: {} - mime: {}, cache_info: {:#?}.", hash, cached_object.mime, cached_object.cache_info);
                self.fetch_with_meta(link, &cached_object.cache_info).await?
            }
            Err(_) => {
                log::trace!("Object {} not found in cache.", hash);
                self.fetch_with_meta(link, &HashMap::new()).await?
            }
        };

        log::trace!("Object {} returned status: {}", hash, response.status());

        if response.status() == actix_web::http::StatusCode::NOT_MODIFIED {
            log::info!("Object {} not modified. Serving cache.", hash);
            return Result::Ok(cached_object.unwrap());
        }

        if !response.status().is_success() {
            let message = format!("Failed to fetch object. URL {}, Code: {}", link.clone(), response.status().to_string());
            log::error!("{}", message);
            return Result::Err(FetchError::FetchFailed(
                message
            ));
        }
        let mut fetched_object = FetchedObject::default();
        for header in vec![header::ETAG, header::LAST_MODIFIED] {
            match response.headers().get(&header) {
                Some(header_value) => {
                    log::info!("Cache info. Object: {}, Header: {}, Value: {:#?}", hash, header, header_value);
                    fetched_object.cache_info.insert(header.to_string(), header_value.to_str().unwrap().to_string());
                }
                _ => (),
            }
        }
        let rand_string: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .map(char::from)
            .collect();
        let cache_hash = hash.clone() + "_" + rand_string.as_str();
        log::trace!("Generated cache for image: {} - hash: {}", link, cache_hash);
        fetched_object.cache_info.insert(
            String::from(IMAGE_CACHE_HASH_LITERAL), cache_hash,
        );
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
    use std::sync::Mutex;

    use httpmock::Method::GET;
    use httpmock::MockServer;

    use crate::cache::CacheError;
    use crate::fetcher::{Fetchable, FetchedObject};

    use super::*;

    struct CachableMock {
        pub hashmap: Arc<Mutex<HashMap<String, FetchedObject>>>,
    }

    impl Cachable<FetchedObject> for CachableMock {
        fn get(&self, link: &String) -> Result<FetchedObject, CacheError> {
            if link.contains("nocache") {
                return Result::Err(CacheError::NoCacheEntry);
            }
            if link.contains("etag") {
                let mut fetched_object = FetchedObject::default();
                fetched_object.cache_info.insert(actix_web::http::header::ETAG.to_string(), "existing".to_string());
                fetched_object.bytes = bytes::Bytes::from("cached-object-etag");
                return Result::Ok(fetched_object);
            }
            return Result::Err(CacheError::NoCacheEntry);
        }

        fn set(&mut self, link: String, object: FetchedObject) -> Result<bool, CacheError> {
            self.hashmap.lock().as_deref_mut().unwrap().insert(link, object);
            return Result::Ok(true);
        }

        fn delete(&mut self, _: &String) -> bool {
            todo!()
        }

        fn count(&self) -> usize {
            return self.hashmap.lock().unwrap().len();
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
        let cache_mock = CachableMock {
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
        let cache_mock = CachableMock {
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
        let cache_mock = CachableMock {
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