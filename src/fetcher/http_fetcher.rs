use super::Fetchable;
use async_trait::async_trait;
use reqwest::{Url};
use crate::fetcher::{FetchedObject, FetchError};
use actix_web::http::header;
use crate::cache::{Cachable, CacheError};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::borrow::BorrowMut;

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
            );
        }

        return Result::Ok(fetched_object);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::memory_cache::MemoryCache;
    use httpmock::MockServer;
    use crate::cache::CacheError;
    use crate::fetcher::{Fetchable, FetchedObject};
    use std::sync::{Mutex, Arc};

    struct CachableMock {

    }

    impl<T: Clone> Cachable<T> for CachableMock {
        fn get(&self, link: &String) -> Result<T, CacheError> {
            todo!()
        }

        fn set(&mut self, link: String, object: T) -> Result<bool, CacheError> {
            todo!()
        }

        fn delete(&mut self, link: &String) -> bool {
            todo!()
        }

        fn count(&self) -> usize {
            todo!()
        }
    }

    #[actix_rt::test]
    async fn test_fetch_image() {

        let server = MockServer::start();
        let url = server.url("/image.png");

        let image_mock = server.mock(|when, then| {
            when.path("/image.png");
            then.status(200)
                .header(actix_web::http::header::CONTENT_TYPE.to_string(), mime::IMAGE_JPEG.to_string())
                .body(String::from("jpg"));
        });
        let cache_mock = CachableMock{};
        let cache: Arc<Mutex<dyn Cachable<FetchedObject> + Sync + Send>> = Arc::new(Mutex::new(
            cache_mock
        ));
        let fetcher = HttpFetcher::new(
            cache.clone(),
        );
        //let image = fetcher.fetch(&url).await;

        //println!("{:#?}", &image.unwrap());
    }
}