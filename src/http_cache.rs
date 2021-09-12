use crate::fetcher::FetchedObject;
use std::time::Duration;

pub mod request_cache_handler;

pub enum RequestCacheResult {
    ServeCache,
    IfNotExpired(Duration),
    CheckBeforeServeCache,
    NoCache,
}

pub trait HttpCacheHandler {
    fn should_serve_cache(&self, fetched_object: &FetchedObject) -> RequestCacheResult;
}
