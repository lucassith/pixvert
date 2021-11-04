use std::collections::HashMap;
use std::ops::Add;
use std::sync::Mutex;
use actix_web::http;

use actix_web::http::{header, HeaderValue};
use chrono;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Duration, NaiveDateTime, TimeZone, Utc};
use log::{error, debug};
use reqwest::{RequestBuilder, StatusCode};
use async_trait::async_trait;
use uuid::Uuid;

use crate::cache::CacheEngine;
use crate::config::Config;
use crate::tagged_element::TaggedElement;

pub(super) const REQUEST_TIME_KEY: &str = "REQUEST_RECEIVED_AT";
pub(super) const CHRONO_HTTP_DATE_FORMAT: &str = "%a, %d %b %Y %H:%M:%S GMT";
pub const HTTP_ADDITIONAL_DATA_HEADERS_KEY: &str = "http_headers";

pub fn generate_resource_tag(tag: &String) -> String {
    return format!("{:x}", md5::compute(tag));
}

#[async_trait]
pub trait Fetcher<T> {
    async fn fetch(&self, resource: &String) -> Result<T, FetchError>;
}

pub struct ReqwestImageFetcher<'a> {
    pub cache: &'a Mutex<Box<dyn CacheEngine + Send>>,
    pub config: Config,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Resource {
    pub content_type: String,
    pub additional_data: HashMap<String, HashMap<String, String>>,
    pub id: String,
    pub content: Vec<u8>,
}

impl Default for Resource {
    fn default() -> Self {
        Self{ content: Vec::default(), additional_data: HashMap::default(), id: Uuid::new_v4().to_string(), content_type: String::from("") }
    }
}

#[derive(Eq, PartialEq, Debug)]
pub enum CanServeCache {
    Yes,
    MustReinvalidateETag(String),
    MustReinvalidateByRequestTime(chrono::DateTime<Utc>),
    No,
}

impl ReqwestImageFetcher<'_> {
    pub fn can_serve_cache(resource: &TaggedElement<Resource>) -> CanServeCache {
        if let Some(cache_control_header) = resource.cache_data.get(header::CACHE_CONTROL.as_str()) {
            let cc = cache_control::CacheControl::from_value(cache_control_header).unwrap();
            if cc.immutable { return CanServeCache::Yes; }
            if cc.no_store { return CanServeCache::No; }
            if let (Some(request_time), Some(duration)) = (resource.cache_data.get(REQUEST_TIME_KEY), cc.max_age) {
                let request_time: DateTime<Utc> = request_time.parse().unwrap();
                let expires_at = request_time.add(Duration::from_std(duration).unwrap());
                let now: DateTime<Utc> = Utc::now();
                debug!("Current time is {} - expires at {}", now.to_rfc3339(), expires_at.to_rfc3339());
                if now > expires_at {
                    return match resource.cache_data.get(header::ETAG.as_str()) {
                        Some(etag) => {
                            CanServeCache::MustReinvalidateETag(etag.clone())
                        }
                        None => {
                            CanServeCache::MustReinvalidateByRequestTime(request_time)
                        }
                    };
                } else {
                    return CanServeCache::Yes;
                }
            }
        }
        if let Some(expires_header) = resource.cache_data.get(header::EXPIRES.as_str()) {
            let expires_at: DateTime<Utc> = Utc.from_local_datetime(&NaiveDateTime::parse_from_str(expires_header, CHRONO_HTTP_DATE_FORMAT)
                .unwrap()).unwrap();
            let now = Utc::now();

            debug!("Current time is {} - expires at {}", now.to_rfc3339(), expires_at.to_rfc3339());

            if now > expires_at { return CanServeCache::No; }
            return CanServeCache::Yes;
        }
        if let Some(etag) = resource.cache_data.get(header::ETAG.as_str()) {
            return CanServeCache::MustReinvalidateETag(etag.clone());
        }
        if let Some(request_time) = resource.cache_data.get(REQUEST_TIME_KEY) {
            return CanServeCache::MustReinvalidateByRequestTime(request_time.parse().unwrap())
        }
        return CanServeCache::No;
    }

    fn insert_request_cache_data(cache_data: &mut HashMap<String, String>, header_name: String, header_value: Option<&HeaderValue>) {
        if let Some(header_value) = header_value {
            if let Ok(header_value) = header_value.to_str() {
                cache_data.insert(header_name, header_value.to_string());
            }
        }
    }

    fn get_cache_control(&self, resource: &String, header: Option<&HeaderValue>) -> String {
        for overriden_cache in &self.config.overridden_cache {
            if resource.contains(&overriden_cache.domain) {
                return overriden_cache.domain.clone()
            }
        }
        if let Some(header_value) = header {
            return match header_value.to_str() {
                Ok(cache_control) => {
                    cache_control.to_string()
                }
                Err(e) => {
                    error!("{}", e);
                    String::from("")
                }
            }
        }
        String::from("")
    }
}

#[derive(Debug)]
pub enum FetchError {
    NotFound,
    NotAvailable,
    InvalidResourceTag(String),
    InvalidFormat,
    Unknown(String),
}

impl From<reqwest::Error> for FetchError {
    fn from(err: reqwest::Error) -> FetchError {
        return FetchError::Unknown(String::from(format!("Unknown Reqwest error. {}", err)));
    }
}

#[async_trait]
impl Fetcher<Resource> for ReqwestImageFetcher<'_> {
    async fn fetch(&self, resource: &String) -> Result<Resource, FetchError> {
        if let Err(parse_error) = reqwest::Url::parse(resource.as_str()) {
            return Err(FetchError::InvalidResourceTag(parse_error.to_string()))
        }
        let resource_tag = generate_resource_tag(&resource);
        let cache_element: Option<TaggedElement<Resource>>;
        {
            cache_element = match self.cache.lock().unwrap().get(resource_tag.as_str()) {
                Some(data) => {
                    Some(bincode::deserialize(data.as_slice()).unwrap())
                },
                None => {
                    None
                }
            }
        }
        let request_builder: RequestBuilder;
        if let Some(tagged_image) = &cache_element {
            request_builder = match Self::can_serve_cache(&tagged_image) {
                CanServeCache::Yes => return Ok(tagged_image.object.clone()),
                CanServeCache::MustReinvalidateETag(etag) => reqwest::Client::new().get(resource).header(
                    http::header::IF_NONE_MATCH.as_str(),
                    etag.as_str()
                ),
                CanServeCache::MustReinvalidateByRequestTime(time) => reqwest::Client::new().get(resource).header(
                    http::header::IF_MODIFIED_SINCE.as_str(),
                    time.format(CHRONO_HTTP_DATE_FORMAT).to_string().as_str()
                ),
                CanServeCache::No => reqwest::Client::new().get(resource),
            };
        } else {
            request_builder = reqwest::Client::new().get(resource);
        }
        let response_time: String = Utc::now().to_rfc3339();
        let response = request_builder.send().await?;
        match response.status() {
            code if code.is_client_error() => return Err(FetchError::NotFound),
            code if code.is_server_error() => return Err(FetchError::NotAvailable),
            code if code == StatusCode::OK => {
                let mut cache_data: HashMap<String, String> = HashMap::new();
                let content_type = match response.headers().get(http::header::CONTENT_TYPE.as_str()) {
                    Some(content_type) => String::from(content_type.to_str().unwrap_or(mime::OCTET_STREAM.as_str())),
                    None => mime::OCTET_STREAM.to_string(),
                };
                let cache_control = self.get_cache_control(resource, response.headers().get(http::header::CACHE_CONTROL.as_str()));
                Self::insert_request_cache_data(&mut cache_data, REQUEST_TIME_KEY.to_string(), Some(&HeaderValue::from_str(response_time.as_str()).unwrap()));
                Self::insert_request_cache_data(&mut cache_data, http::header::ETAG.to_string(), response.headers().get(http::header::ETAG));
                Self::insert_request_cache_data(&mut cache_data, http::header::EXPIRES.to_string(), response.headers().get(http::header::EXPIRES));
                Self::insert_request_cache_data(&mut cache_data, http::header::CACHE_CONTROL.to_string(), Some(&HeaderValue::from_str(cache_control.as_str()).unwrap()));
                let resource = TaggedElement {
                    object: Resource {
                        content_type,
                        content: response.bytes().await.unwrap().to_vec(),
                        id: Uuid::new_v4().to_string(),
                        additional_data: HashMap::from([(
                                String::from(HTTP_ADDITIONAL_DATA_HEADERS_KEY),
                                HashMap::from([
                                    (String::from(header::CACHE_CONTROL.to_string()), String::from(cache_control)),
                                    (String::from(header::EXPIRES.to_string()), cache_data.get(http::header::EXPIRES.as_str()).unwrap_or(&String::from("")).to_string())
                                ]),
                            )],
                        )
                    },
                    cache_data
                };
                {
                    self.cache.lock().unwrap().set(
                        &resource_tag,
                        &bincode::serialize(&resource.clone()).unwrap(),
                    ).unwrap();
                }
                Ok(resource.object)
            }
            code if code == StatusCode::NOT_MODIFIED => {
                match &cache_element {
                    Some(cache_resource) => {
                        Ok((*cache_resource).clone().object)
                    }
                    None => return Err(FetchError::Unknown("Server returned 'not modified' but the cache value doesn't exist.".to_string()))
                }
            },
            _ => {
                todo!();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::ops::{Add, Sub};
    use std::sync::{Mutex};

    use actix_web::http::header;
    use chrono::{Duration, Utc};
    use httpmock::{MockServer, prelude::*};
    use log4rs::append::console::ConsoleAppender;
    use log4rs::Config;
    use log4rs::config::{Appender, Root};
    use crate::config::Config as ApplicationConfig;
    use log::LevelFilter;
    use crate::cache::{CacheEngine, HashMapCacheEngine, NoCacheEngine};

    use crate::fetcher::{CanServeCache, CHRONO_HTTP_DATE_FORMAT, Fetcher, generate_resource_tag, REQUEST_TIME_KEY, ReqwestImageFetcher, Resource};
    use crate::tagged_element::TaggedElement;

    fn init() {
        let stdout = ConsoleAppender::builder().build();
        let config = Config::builder()
            .appender(Appender::builder().build("stdout", Box::new(stdout)))
            .build(Root::builder().appender("stdout").build(LevelFilter::Trace))
            .unwrap();
        match log4rs::init_config(config) {
            Err(_) => {},
            _ => {},
        }
    }

    #[actix_rt::test]
    async fn test_get_new_resource() {
        let cache = Mutex::from(Box::from(NoCacheEngine{}) as Box<dyn CacheEngine + Send>);
        let fetcher = ReqwestImageFetcher{ cache: &cache, config: ApplicationConfig::default() };
        let server = MockServer::start();
        let mock_body: Vec<u8> = Vec::from([0,1,2,3,4,5]);

        server.mock(|when, then| {
            when.method(GET)
                .path("/image.png");
            then.status(200)
                .header("content-type", "application/png")
                .body(mock_body.clone());
        });

        let resource = fetcher.fetch(&format!("http://{}:{}/image.png", server.host(), server.port())).await.ok().unwrap();
        assert_eq!(resource.content, mock_body)
    }

    #[actix_rt::test]
    async fn test_should_set_cache_value() {
        let hashmap = HashMapCacheEngine::default();
        let cache = Mutex::from(Box::from(hashmap) as Box<dyn CacheEngine + Send>);
        let fetcher = ReqwestImageFetcher{ cache: &cache, config: ApplicationConfig::default() };
        let server = MockServer::start();
        let mock_body: Vec<u8> = Vec::from([0,1,2,3,4,5]);

        server.mock(|when, then| {
            when.method(GET)
                .path("/image.png");
            then.status(200)
                .header("content-type", "application/png")
                .body(mock_body.clone());
        });
        let resource_url = format!("http://{}:{}/image.png", server.host(), server.port());
        fetcher.fetch(&resource_url).await.ok().unwrap();
        let cache_value = cache.lock().unwrap().get(generate_resource_tag(&resource_url).as_str());
        let tagged_resource: TaggedElement<Resource> = bincode::deserialize(&cache_value.unwrap()).unwrap();
        assert_eq!(tagged_resource.object.content, mock_body)
    }

    #[test]
    fn test_no_cache_if_no_headers() {
        let can_serve = ReqwestImageFetcher::can_serve_cache(&TaggedElement {
            object: Resource::default(),
            cache_data: HashMap::default(),
        });
        assert_eq!(can_serve, CanServeCache::No);
    }

    #[test]
    fn test_can_serve_cache_immutable() {
        let mut cache_data = HashMap::default();
        cache_data.insert(header::CACHE_CONTROL.to_string(), String::from("immutable"));
        let can_serve = ReqwestImageFetcher::can_serve_cache(&TaggedElement {
            object: Resource::default(),
            cache_data,
        });
        assert_eq!(can_serve, CanServeCache::Yes);
    }

    #[test]
    fn test_no_cache_if_no_store() {
        let mut cache_data = HashMap::default();
        cache_data.insert(header::CACHE_CONTROL.to_string(), String::from("no-store"));
        let can_serve = ReqwestImageFetcher::can_serve_cache(&TaggedElement {
            object: Resource::default(),
            cache_data,
        });
        assert_eq!(can_serve, CanServeCache::No);
    }

    #[test]
    fn test_cache_depending_on_max_age_not_expired_59_seconds() {
        let mut cache_data = HashMap::default();
        cache_data.insert(header::CACHE_CONTROL.to_string(), String::from("max-age=60"));
        cache_data.insert(REQUEST_TIME_KEY.to_string(), Utc::now().sub(Duration::seconds(59)).to_rfc3339());
        let can_serve = ReqwestImageFetcher::can_serve_cache(&TaggedElement {
            object: Resource::default(),
            cache_data,
        });
        assert_eq!(can_serve, CanServeCache::Yes);
    }

    #[test]
    fn test_cache_depending_on_max_age_must_reinvalidate_after_60_seconds() {
        init();
        let mut cache_data = HashMap::default();
        let request_time = Utc::now().sub(Duration::seconds(60));
        cache_data.insert(header::CACHE_CONTROL.to_string(), String::from("max-age=60"));
        cache_data.insert(REQUEST_TIME_KEY.to_string(), request_time.to_rfc3339());
        let can_serve = ReqwestImageFetcher::can_serve_cache(&TaggedElement {
            object: Resource::default(),
            cache_data,
        });
        assert_eq!(can_serve, CanServeCache::MustReinvalidateByRequestTime(request_time.clone()));
    }

    #[test]
    fn test_cache_depending_on_max_age_must_reinvalidate_etag_after_60_seconds() {
        init();
        let mut cache_data = HashMap::default();
        let request_time = Utc::now().sub(Duration::seconds(60));
        let etag = "W/11";
        cache_data.insert(header::CACHE_CONTROL.to_string(), String::from("max-age=60"));
        cache_data.insert(header::ETAG.to_string(), etag.to_string());
        cache_data.insert(REQUEST_TIME_KEY.to_string(), request_time.to_rfc3339());
        let can_serve = ReqwestImageFetcher::can_serve_cache(&TaggedElement {
            object: Resource::default(),
            cache_data,
        });
        assert_eq!(can_serve, CanServeCache::MustReinvalidateETag(etag.to_string()));
    }

    #[test]
    fn test_cache_depending_on_expires_valid_date() {
        init();
        let mut cache_data = HashMap::default();
        let request_time = Utc::now().add(Duration::seconds(10));
        cache_data.insert(header::EXPIRES.to_string(), request_time.format(CHRONO_HTTP_DATE_FORMAT).to_string());
        let can_serve = ReqwestImageFetcher::can_serve_cache(&TaggedElement {
            object: Resource::default(),
            cache_data,
        });
        assert_eq!(can_serve, CanServeCache::Yes);
    }

    #[test]
    fn test_cache_depending_on_expires_expired_date() {
        init();
        let mut cache_data = HashMap::default();
        let request_time = Utc::now().sub(Duration::seconds(10));
        cache_data.insert(header::EXPIRES.to_string(), request_time.format(CHRONO_HTTP_DATE_FORMAT).to_string());
        let can_serve = ReqwestImageFetcher::can_serve_cache(&TaggedElement {
            object: Resource::default(),
            cache_data,
        });
        assert_eq!(can_serve, CanServeCache::No);
    }

    #[test]
    fn test_cache_etag_reinvalidation_if_cache_headers_not_exist() {
        init();
        let mut cache_data = HashMap::default();
        let etag = "W/38271";
        cache_data.insert(header::ETAG.to_string(), etag.to_string());
        let can_serve = ReqwestImageFetcher::can_serve_cache(&TaggedElement {
            object: Resource::default(),
            cache_data,
        });
        assert_eq!(can_serve, CanServeCache::MustReinvalidateETag(etag.to_string()));
    }

    #[test]
    fn test_cache_modified_date_reinvalidation_if_cache_headers_not_exist() {
        init();
        let mut cache_data = HashMap::default();
        let request_date = Utc::now().sub(Duration::seconds(59));
        cache_data.insert(REQUEST_TIME_KEY.to_string(), request_date.to_rfc3339());
        let can_serve = ReqwestImageFetcher::can_serve_cache(&TaggedElement {
            object: Resource::default(),
            cache_data,
        });
        assert_eq!(can_serve, CanServeCache::MustReinvalidateByRequestTime(request_date));
    }
}
