use std::sync::Arc;

pub mod memory_cache;

#[derive(Debug)]
pub enum CacheError {
    NoCacheEntry,
}

pub trait Cachable<T: Clone> {
    fn get(&self, link: &String) -> Result<T, CacheError>;
    fn set(&mut self, link: String, object: T) -> Result<bool, CacheError>;
    fn delete(&mut self, link: &String) -> bool;
    fn count(&self) -> usize;
}

