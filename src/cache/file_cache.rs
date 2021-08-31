use md5;
use std::path::{Path, PathBuf};
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use log::info;
use std::fs;
use bytes::Bytes;

use crate::fetcher::FetchedObject;

use super::Cachable;
use crate::cache::CacheError;
use image::EncodableLayout;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufRead, LineWriter, Write};

const HASHMAP_DIVIDER: &str = "␞";
const HASHMAP_VALUE_DIVIDER: &str = "␟";

pub struct FileCache {
    catalog: PathBuf,
}

impl FileCache {
    pub fn new(catalog: &String) -> FileCache {
        let rand_string: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(10)
            .map(char::from)
            .collect();
        let path = Path::new(catalog).join(rand_string);
        info!("Created path {:#?}", path);
        FileCache {
            catalog: path
        }
    }
}

impl FileCache {
    pub fn generate_file_name(name: &String) -> String {
        format!("{:x}", md5::compute(name))
    }
}

impl Cachable<FetchedObject> for FileCache {
    fn get(&self, key: &String) -> Result<FetchedObject, super::CacheError> {
        let file = File::open(self.catalog.join(FileCache::generate_file_name(key)));
        return match file {
            Ok(file) => {
                let reader = BufReader::new(file);
                let mut lines = reader.lines();
                let mime = lines.next().unwrap().unwrap();
                let hashmap_string = lines.next().unwrap().unwrap();
                let mut cache_info: HashMap<String,String> = HashMap::new();
                for line in hashmap_string.split(HASHMAP_DIVIDER) {
                    let mut splitter = line.splitn(2, HASHMAP_VALUE_DIVIDER);
                    cache_info.insert(String::from(splitter.next().unwrap()), String::from(splitter.next().unwrap()));
                }
                let bytes: Bytes = Bytes::from(lines.fold(String::new(), |a, b| {
                    [a, b.unwrap()].concat()
                }));
                return Result::Ok(FetchedObject {
                    mime,
                    cache_info,
                    bytes,
                });
            },
            Err(e) => {
                Result::Err(CacheError::NoCacheEntry)
            }
        }
    }

    fn set(&mut self, key: String, obj: FetchedObject) -> Result<bool, super::CacheError> {
        let file = OpenOptions::new().create(true).write(true).open(
            self.catalog.join(FileCache::generate_file_name(&key))
        ).unwrap();
        let mut file = LineWriter::new(file);
        file.write(obj.mime.as_bytes());
        file.write(obj.cache_info.into_iter().fold(String::new(), |cur, next| -> String {
            if cur.is_empty() {
                return [next.0, HASHMAP_VALUE_DIVIDER.to_string(), next.1].concat();
            }
            return [cur, HASHMAP_DIVIDER.to_string(), next.0, HASHMAP_VALUE_DIVIDER.to_string(), next.1].concat();
        }).as_bytes());
        file.write(obj.bytes.as_bytes());
        return Ok(true);
    }

    fn delete(&mut self, key: &String) -> bool {
        fs::remove_file(FileCache::generate_file_name(&key)).is_ok()
    }

    fn count(&self) -> usize {
        let dir = fs::read_dir(&self.catalog).unwrap();
        dir.count()
    }
}
