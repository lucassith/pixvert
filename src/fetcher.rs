use std::ops::Add;

use actix_web::http::header;
use chrono;
use chrono::{DateTime, Duration, NaiveDateTime, TimeZone, Utc};
use log::debug;

use crate::cache::CacheEngine;
use crate::image::Image;
use crate::tagged_element::TaggedElement;

pub(super) const REQUEST_TIME_KEY: &str = "REQUEST_RECEIVED_AT";

pub trait Fetcher<T> {
    fn fetch(resource: String) -> T;
}

pub struct ReqwestImageFetcher {
    cache: dyn CacheEngine,
}

#[derive(Eq, PartialEq, Debug)]
pub enum CanServeCache {
    Yes,
    MustReinvalidateETag(String),
    MustReinvalidateByRequestTime(chrono::DateTime<Utc>),
    No,
}

impl ReqwestImageFetcher {
    pub fn can_serve_cache(resource: TaggedElement<Image>) -> CanServeCache {
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
            let expires_at: DateTime<Utc> = Utc.from_local_datetime(&NaiveDateTime::parse_from_str(expires_header, "%a, %d %b %Y %H:%M:%S GMT")
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
}

pub enum FetchError {
    NotFound,
    NotAvailable,
}

impl Fetcher<Image> for ReqwestImageFetcher {
    fn fetch(_: String) -> Image {
        unimplemented!();
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::ops::{Add, Sub};

    use actix_web::http::header;
    use chrono::{Duration, Utc};
    use log4rs::append::console::ConsoleAppender;
    use log4rs::Config;
    use log4rs::config::{Appender, Root};
    use log::LevelFilter;

    use crate::fetcher::{CanServeCache, REQUEST_TIME_KEY, ReqwestImageFetcher};
    use crate::image::Image;
    use crate::tagged_element::TaggedElement;

    fn init() {
        let stdout = ConsoleAppender::builder().build();
        let config = Config::builder()
            .appender(Appender::builder().build("stdout", Box::new(stdout)))
            .build(Root::builder().appender("stdout").build(LevelFilter::Trace))
            .unwrap();
        match log4rs::init_config(config) {
            Err(e) => {},
            _ => {},
        }
    }

    #[test]
    fn test_no_cache_if_no_headers() {
        let can_serve = ReqwestImageFetcher::can_serve_cache(TaggedElement {
            object: Image::default(),
            cache_data: HashMap::default(),
        });
        assert_eq!(can_serve, CanServeCache::No);
    }

    #[test]
    fn test_can_serve_cache_immutable() {
        let mut cache_data = HashMap::default();
        cache_data.insert(header::CACHE_CONTROL.to_string(), String::from("immutable"));
        let can_serve = ReqwestImageFetcher::can_serve_cache(TaggedElement {
            object: Image::default(),
            cache_data,
        });
        assert_eq!(can_serve, CanServeCache::Yes);
    }

    #[test]
    fn test_no_cache_if_no_store() {
        let mut cache_data = HashMap::default();
        cache_data.insert(header::CACHE_CONTROL.to_string(), String::from("no-store"));
        let can_serve = ReqwestImageFetcher::can_serve_cache(TaggedElement {
            object: Image::default(),
            cache_data,
        });
        assert_eq!(can_serve, CanServeCache::No);
    }

    #[test]
    fn test_cache_depending_on_max_age_not_expired_59_seconds() {
        let mut cache_data = HashMap::default();
        cache_data.insert(header::CACHE_CONTROL.to_string(), String::from("max-age=60"));
        cache_data.insert(REQUEST_TIME_KEY.to_string(), Utc::now().sub(Duration::seconds(59)).to_rfc3339());
        let can_serve = ReqwestImageFetcher::can_serve_cache(TaggedElement {
            object: Image::default(),
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
        let can_serve = ReqwestImageFetcher::can_serve_cache(TaggedElement {
            object: Image::default(),
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
        let can_serve = ReqwestImageFetcher::can_serve_cache(TaggedElement {
            object: Image::default(),
            cache_data,
        });
        assert_eq!(can_serve, CanServeCache::MustReinvalidateETag(etag.to_string()));
    }

    #[test]
    fn test_cache_depending_on_expires_valid_date() {
        init();
        let mut cache_data = HashMap::default();
        let request_time = Utc::now().add(Duration::seconds(10));
        cache_data.insert(header::EXPIRES.to_string(), request_time.format("%a, %d %b %Y %H:%M:%S GMT").to_string());
        let can_serve = ReqwestImageFetcher::can_serve_cache(TaggedElement {
            object: Image::default(),
            cache_data,
        });
        assert_eq!(can_serve, CanServeCache::Yes);
    }

    #[test]
    fn test_cache_depending_on_expires_expired_date() {
        init();
        let mut cache_data = HashMap::default();
        let request_time = Utc::now().sub(Duration::seconds(10));
        cache_data.insert(header::EXPIRES.to_string(), request_time.format("%a, %d %b %Y %H:%M:%S GMT").to_string());
        let can_serve = ReqwestImageFetcher::can_serve_cache(TaggedElement {
            object: Image::default(),
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
        let can_serve = ReqwestImageFetcher::can_serve_cache(TaggedElement {
            object: Image::default(),
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
        let can_serve = ReqwestImageFetcher::can_serve_cache(TaggedElement {
            object: Image::default(),
            cache_data,
        });
        assert_eq!(can_serve, CanServeCache::MustReinvalidateByRequestTime(request_date));
    }
}
