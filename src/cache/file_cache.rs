use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{Error, Read, Write};
use std::path::{Path, PathBuf};

use log::debug;
use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;

use crate::cache::CacheEngine;

pub struct FileCache {
    dir: PathBuf,
}

impl FileCache {
    pub fn new(catalog: &String) -> FileCache {
        let rand_string: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(10)
            .map(char::from)
            .collect();
        let path = Path::new(catalog).join(rand_string);
        fs::create_dir_all(String::from(path.to_string_lossy())).unwrap();
        debug!("Created path {:#?}", path);
        FileCache {
            dir: path
        }
    }

    pub fn generate_file_name(name: &str) -> String {
        format!("{:x}", md5::compute(name))
    }
}

impl CacheEngine for FileCache {
    fn get(&self, name: &str) -> Option<Vec<u8>> {
        let path = self.dir.join(FileCache::generate_file_name(name));
        return match File::open(&path) {
            Ok(mut file) => {
                debug!("Found file {} under: {}", name, path.to_string_lossy());
                let mut file_content = Vec::new();
                file.read_to_end(&mut file_content).unwrap();
                Option::Some(file_content)
            }
            Err(_) => {
                Option::None
            }
        };
    }

    fn set(&self, name: &str, data: &Vec<u8>) -> Result<bool, Error> {
        let file_path = self.dir.join(FileCache::generate_file_name(name));

        let mut file = OpenOptions::new().create(true).write(true).read(true).open(
            &file_path
        )?;
        debug!("Created file at {}", file_path.to_string_lossy());
        file.write_all(data).unwrap();
        return Result::Ok(true);
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile;

    use crate::cache::CacheEngine;
    use crate::cache::file_cache::FileCache;

    #[test]
    fn file_cache_set() {
        let temp_path = tempfile::TempDir::new().unwrap().into_path();
        let cache_name = "unit-test";
        let file_cache = FileCache {
            dir: temp_path.clone(),
        };
        let data: Vec<u8> = Vec::from([0, 0, 0, 8]);
        file_cache.set(cache_name, &data).unwrap();
        let content = fs::read(temp_path.join(FileCache::generate_file_name(cache_name))).unwrap();
        assert_eq!(data, content);
        fs::remove_dir_all(temp_path).unwrap();
    }

    #[test]
    fn file_cache_get() {
        let temp_path = tempfile::TempDir::new().unwrap().into_path();
        let cache_name = "unit-test";
        let data: Vec<u8> = Vec::from([0, 1, 2, 4, 8, 16, 32]);
        let file_name = FileCache::generate_file_name(cache_name);
        fs::write(temp_path.join(file_name), &data).unwrap();

        let file_cache = FileCache {
            dir: temp_path.clone(),
        };
        let content = file_cache.get(cache_name).unwrap();
        assert_eq!(data, content);
        fs::remove_dir_all(temp_path).unwrap();
    }
}
