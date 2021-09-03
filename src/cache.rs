use std::num::ParseIntError;

pub mod memory_cache;
pub mod file_cache;

#[derive(Debug)]
pub enum CacheError {
    NoCacheEntry,
    InvalidCacheEntry,
}

impl From<ParseIntError> for CacheError {
    fn from(_: ParseIntError) -> Self {
        Self::InvalidCacheEntry
    }
}

pub trait Cachable<T: Clone> {
    fn get(&self, link: &String) -> Result<T, CacheError>;
    fn set(&mut self, link: String, object: T) -> Result<bool, CacheError>;
    fn delete(&mut self, link: &String) -> bool;
    fn count(&self) -> usize;
}

