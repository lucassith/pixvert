mod file_cache;

use std::collections::HashMap;
use std::io::Error;
use std::sync::Mutex;


pub trait CacheEngine {
    fn get(&self, name: &str) -> Option<Vec<u8>>;
    fn set(&self, name: &str, data: &Vec<u8>) -> Result<bool, Error>;
}

pub struct NoCacheEngine {}

impl CacheEngine for NoCacheEngine {
    fn get(&self, _: &str) -> Option<Vec<u8>> {
        Option::None
    }
    fn set(&self, _: &str, _: &Vec<u8>) -> Result<bool, Error> {
        Result::Ok(true)
    }
}

pub struct HashMapCacheEngine {
    hashmap: Mutex<HashMap<String, Vec<u8>>>
}

impl HashMapCacheEngine {
    pub fn new() -> Self {
        HashMapCacheEngine{
            hashmap: Mutex::from(HashMap::default())
        }
    }
}

impl Default for HashMapCacheEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl CacheEngine for HashMapCacheEngine {
    fn get(&self, name: &str) -> Option<Vec<u8>> {
        return match self.hashmap.lock().unwrap().get(name) {
            Some(value) => Some(value.clone()),
            None => None
        }
    }

    fn set(&self, name: &str, data: &Vec<u8>) -> Result<bool, Error> {
        self.hashmap.lock().unwrap().insert(name.to_string(), data.clone());
        return Ok(true);
    }
}
