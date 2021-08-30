use super::{Cachable};
use md5;
use std::path::Path;
use crate::fetcher::FetchedObject;

pub struct FileCache {
    catalog: String
}

impl FileCache {
    pub fn new(catalog: String) -> FileCache {
        FileCache{
            catalog
        }
    }
}

impl FileCache {
    pub fn generate_file_name(name: &String) -> String {
        format!("{:x}", md5::compute(name))
    }
}

impl Cachable<FetchedObject> for FileCache {
    fn get(&self, link: &String) -> Result<FetchedObject, super::CacheError> {
        return unimplemented!();
        let file_name = FileCache::generate_file_name(link);
        let file_path = Path::new(&self.catalog).join(file_name);
        log::debug!("Checking file under {}", file_path.to_string_lossy());
    }

    fn set(&mut self, link: String, object: FetchedObject) -> Result<bool, super::CacheError> {
        unimplemented!()
    }

    fn delete(&mut self, link: &String) -> bool {
        unimplemented!()
    }

    fn count(&self) -> usize {
        unimplemented!()
    }
}
