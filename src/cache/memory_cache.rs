use std::{collections::HashMap, sync::Arc};

use super::{Cachable, CacheError};

pub struct MemoryCache<T: Clone> {
    objects: HashMap<String, T>
}

impl<T: Clone> MemoryCache<T> {
    pub fn new() -> MemoryCache<T> {
        MemoryCache{
            objects: HashMap::new()
        }
    }
}

impl<T: Clone> Cachable<T> for MemoryCache<T> {
    fn get(&self, link: &String) -> Result<T, super::CacheError> {
        let object = self.objects.get(link).clone();
        return match object {
            Some(object) => {
                Result::Ok(object.clone())
            },
            None => {
                Result::Err(CacheError::NoCacheEntry)
            }
        }
    }

    fn set(&mut self, link: String, object: T) -> Result<bool, super::CacheError> {
        self.objects.insert(link, object);
        return Result::Ok(true);
    }

    fn delete(&mut self, link: &String) -> bool {
        return self.objects.remove(link).is_some();
    }

    fn count(&self) -> usize {
        self.objects.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_set() {
        let mut memory_cache: MemoryCache<String> = MemoryCache::new();
        let identifier = String::from("unit test identifier");
        let object = String::from("unit test object");
        assert!(memory_cache.set(identifier.clone(), object.clone()).is_ok());
        let found_object = memory_cache.get(&identifier);
        match found_object {
            Ok(found_object) => {
                assert_eq!(*found_object, object);
            }
            Err(_) => {
                assert!(false)
            }
        }
    }

    #[test]
    fn test_set_override() {
        let mut memory_cache: MemoryCache<String> = MemoryCache::new();
        let identifier = String::from("unit test identifier");
        let object = String::from("unit test object");
        let object_overriden = String::from("unit test object iv");
        assert!(memory_cache.set(identifier.clone(), object.clone()).is_ok());
        assert!(memory_cache.set(identifier.clone(), object_overriden.clone()).is_ok());
        let found_object = memory_cache.get(&identifier);
        match found_object {
            Ok(found_object) => {
                assert_eq!(*found_object, object_overriden);
            }
            Err(_) => {
                assert!(false)
            }
        }
    }

    #[test]
    fn test_delete() {
        let mut memory_cache: MemoryCache<String> = MemoryCache::new();
        let identifier = String::from("unit test identifier");
        let object = String::from("unit test object");
        assert!(memory_cache.set(identifier.clone(), object.clone()).is_ok());
        assert!(memory_cache.get(&identifier).is_ok());
        assert!(memory_cache.delete(&identifier));
        assert!(memory_cache.get(&identifier).is_err());
        assert!(!memory_cache.delete(&identifier));
    }
}