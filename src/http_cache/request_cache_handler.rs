use crate::http_cache::{HttpCacheHandler, RequestCacheResult};
use crate::fetcher::FetchedObject;
use actix_web::http::header;
use cache_control::{CacheControl};

pub struct RequestCacheHandler {
}

impl HttpCacheHandler for RequestCacheHandler {
    fn should_serve_cache(&self, fetched_object: &FetchedObject) -> RequestCacheResult {
        if let Some(cache_header) = fetched_object.cache_info.get(header::CACHE_CONTROL.as_str()) {
            let control = CacheControl::from_value(cache_header).unwrap_or(CacheControl::default());
            if control.no_store {
                return RequestCacheResult::NoCache;
            }
            if let Some(duration) = control.max_age {
                return RequestCacheResult::IfNotExpired(duration.to_std().unwrap_or_default())
            }
        }
        return RequestCacheResult::ServeCache;
    }
}
