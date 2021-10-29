mod file_cache;

use std::io::Error;


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
