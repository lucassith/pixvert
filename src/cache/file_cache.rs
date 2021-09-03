use std::collections::HashMap;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, Cursor, LineWriter, Read, Write};
use std::path::{Path, PathBuf};

use bytes::Bytes;
use image::{DynamicImage, EncodableLayout, GenericImageView, RgbaImage};
use log::{debug};
use md5;
use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;
use tempdir::TempDir;

use crate::cache::CacheError;
use crate::fetcher::FetchedObject;
use crate::image::{DecodedImage, EncodedImage};

use super::Cachable;

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
        let result = fs::create_dir_all(String::from(path.to_string_lossy())).unwrap();
        debug!("Created path {:#?}", path);
        FileCache {
            catalog: path
        }
    }
}

impl FileCache {
    pub fn generate_file_name(name: &String) -> String {
        format!("{:x}", md5::compute(name))
    }

    pub fn write_string(&self, mut writer: Box<&mut dyn Write>, value: &String) {
        writer.write(value.as_bytes());
        writer.write_all(b"\n").unwrap();
    }

    pub fn write_hashmap(&self, mut writer: Box<&mut dyn Write>, hashmap: &HashMap<String, String>) {
        writer.write(hashmap.into_iter().fold(String::new(), |cur, next| -> String {
            if cur.is_empty() {
                return [next.0.clone(), HASHMAP_VALUE_DIVIDER.to_string(), next.1.clone()].concat();
            }
            return [cur, HASHMAP_DIVIDER.to_string(), next.0.clone(), HASHMAP_VALUE_DIVIDER.to_string(), next.1.clone()].concat();
        }).as_bytes());
        writer.write_all(b"\n").unwrap();
    }

    pub fn write_bytes(&self, mut writer: Box<&mut dyn Write>, bytes: &Bytes) {
        writer.write_all(bytes.as_bytes());
        writer.flush().unwrap();
    }

    pub fn read_string(&self, mut reader: Box<&mut dyn BufRead>) -> String {
        let mut mime_bytes = Vec::new();
        reader.read_until(b'\n', &mut mime_bytes);
        String::from(std::str::from_utf8(mime_bytes.as_slice()).unwrap().trim_end())
    }

    pub fn read_hashmap(&self, mut reader: Box<&mut dyn BufRead>) -> HashMap<String, String> {
        let mut hashmap_bytes = Vec::new();
        reader.read_until(b'\n', &mut hashmap_bytes);
        let hashmap_string = std::str::from_utf8(hashmap_bytes.as_slice()).unwrap().trim_end();
        let mut cache_info: HashMap<String, String> = HashMap::new();
        for line in hashmap_string.split(HASHMAP_DIVIDER) {
            if (!line.is_empty()) {
                let mut splitter = line.splitn(2, HASHMAP_VALUE_DIVIDER);
                cache_info.insert(String::from(splitter.next().unwrap()), String::from(splitter.next().unwrap()));
            }
        }
        return cache_info;
    }

    pub fn read_bytes(&self, mut reader: Box<&mut dyn BufRead>) -> Bytes {
        let mut payload_bytes = Vec::new();
        reader.read_to_end(&mut payload_bytes);
        Bytes::from(payload_bytes)
    }

    fn delete(&mut self, key: &String) -> bool {
        fs::remove_file(FileCache::generate_file_name(&key)).is_ok()
    }

    fn count(&self) -> usize {
        let dir = fs::read_dir(&self.catalog).unwrap();
        dir.count()
    }
}

impl Cachable<FetchedObject> for FileCache {
    fn get(&self, key: &String) -> Result<FetchedObject, super::CacheError> {
        let file = File::open(self.catalog.join(FileCache::generate_file_name(key)));
        return match file {
            Ok(mut file) => {
                let mut file_content = Vec::new();
                file.read_to_end(&mut file_content).unwrap();
                let mut cursor = Cursor::new(file_content);

                let mime = self.read_string(Box::from(&mut cursor as &mut dyn BufRead));
                let cache_info = self.read_hashmap(Box::from(&mut cursor as &mut dyn BufRead));
                let bytes = self.read_bytes(Box::from(&mut cursor as &mut dyn BufRead));

                return Result::Ok(FetchedObject {
                    mime,
                    cache_info,
                    bytes,
                });
            }
            Err(e) => {
                Result::Err(CacheError::NoCacheEntry)
            }
        };
    }

    fn set(&mut self, key: String, obj: FetchedObject) -> Result<bool, super::CacheError> {
        let file_path = self.catalog.join(FileCache::generate_file_name(&key));
        debug!("Trying to create/open {:#?}", file_path);
        let file = OpenOptions::new().create(true).create_new(true).write(true).read(true).open(
            file_path
        ).unwrap();
        let mut file = LineWriter::new(file);
        self.write_string(Box::from(&mut file as &mut dyn Write), &obj.mime);
        self.write_hashmap(Box::from(&mut file as &mut dyn Write), &obj.cache_info);
        self.write_bytes(Box::from(&mut file as &mut dyn Write), &obj.bytes);
        return Ok(true);
    }

    fn delete(&mut self, key: &String) -> bool {
        self.delete(key)
    }

    fn count(&self) -> usize {
        self.count()
    }
}

impl Cachable<DecodedImage> for FileCache {
    fn get(&self, key: &String) -> Result<DecodedImage, super::CacheError> {
        let file = File::open(self.catalog.join(FileCache::generate_file_name(key)));
        return match file {
            Ok(mut file) => {
                let mut file_content = Vec::new();
                file.read_to_end(&mut file_content).unwrap();
                let mut cursor = Cursor::new(file_content);

                let from = self.read_string(Box::from(&mut cursor as &mut dyn BufRead));
                let width = self.read_string(Box::from(&mut cursor as &mut dyn BufRead)).parse::<u32>()?;
                let height = self.read_string(Box::from(&mut cursor as &mut dyn BufRead)).parse::<u32>()?;
                let cache_info = self.read_hashmap(Box::from(&mut cursor as &mut dyn BufRead));
                let bytes = self.read_bytes(Box::from(&mut cursor as &mut dyn BufRead));
                let rgba_image = RgbaImage::from_raw(width, height, bytes.to_vec());
                if rgba_image.is_none() {
                    return Result::Err(CacheError::InvalidCacheEntry);
                }
                let image = DynamicImage::ImageRgba8(rgba_image.unwrap());

                return Result::Ok(DecodedImage {
                    cache_info,
                    from,
                    image,
                });
            }
            Err(e) => {
                Result::Err(CacheError::NoCacheEntry)
            }
        };
    }

    fn set(&mut self, key: String, obj: DecodedImage) -> Result<bool, super::CacheError> {
        let file_path = self.catalog.join(FileCache::generate_file_name(&key));
        debug!("Trying to create/open {:#?}", file_path);
        let file = OpenOptions::new().create(true).create_new(true).write(true).read(true).open(
            file_path
        ).unwrap();
        let mut file = LineWriter::new(file);
        self.write_string(Box::from(&mut file as &mut dyn Write), &obj.from);
        self.write_string(Box::from(&mut file as &mut dyn Write), &obj.image.width().to_string());
        self.write_string(Box::from(&mut file as &mut dyn Write), &obj.image.height().to_string());
        self.write_hashmap(Box::from(&mut file as &mut dyn Write), &obj.cache_info);
        let image_bytes = obj.image.to_rgba8().to_vec();
        self.write_bytes(Box::from(&mut file as &mut dyn Write), &Bytes::from(image_bytes));
        return Ok(true);
    }

    fn delete(&mut self, key: &String) -> bool {
        self.delete(key)
    }

    fn count(&self) -> usize {
        self.count()
    }
}

impl Cachable<EncodedImage> for FileCache {
    fn get(&self, key: &String) -> Result<EncodedImage, super::CacheError> {
        let file = File::open(self.catalog.join(FileCache::generate_file_name(key)));
        return match file {
            Ok(mut file) => {
                let mut file_content = Vec::new();
                file.read_to_end(&mut file_content).unwrap();
                let mut cursor = Cursor::new(file_content);

                let from = self.read_string(Box::from(&mut cursor as &mut dyn BufRead));
                let output_mime = self.read_string(Box::from(&mut cursor as &mut dyn BufRead));
                let cache_info = self.read_hashmap(Box::from(&mut cursor as &mut dyn BufRead));
                let image = self.read_bytes(Box::from(&mut cursor as &mut dyn BufRead));

                return Result::Ok(EncodedImage {
                    from,
                    output_mime,
                    cache_info,
                    image,
                });
            }
            Err(e) => {
                Result::Err(CacheError::NoCacheEntry)
            }
        };
    }

    fn set(&mut self, key: String, obj: EncodedImage) -> Result<bool, super::CacheError> {
        let file_path = self.catalog.join(FileCache::generate_file_name(&key));
        debug!("Trying to create/open {:#?}", file_path);
        let file = OpenOptions::new().create(true).write(true).read(true).open(
            file_path
        ).unwrap();
        let mut file = LineWriter::new(file);
        self.write_string(Box::from(&mut file as &mut dyn Write), &obj.from);
        self.write_string(Box::from(&mut file as &mut dyn Write), &obj.output_mime);
        self.write_hashmap(Box::from(&mut file as &mut dyn Write), &obj.cache_info);
        self.write_bytes(Box::from(&mut file as &mut dyn Write), &obj.image);
        return Ok(true);
    }

    fn delete(&mut self, key: &String) -> bool {
        self.delete(key)
    }

    fn count(&self) -> usize {
        self.count()
    }
}


#[cfg(test)]
mod tests {
    use image::ImageFormat;
    use image::io::Reader;

    use crate::fetcher::FetchedObject;

    use super::*;

    #[test]
    fn test_set_get_fetched_object() {
        let mut file_cache = FileCache::new(
            &String::from("/tmp/rust-unit-test")
        );
        let key = String::from("lorem.png");
        let mut hashmap: HashMap<String, String> = HashMap::new();
        hashmap.insert(String::from("lorem"), String::from("ipsum"));
        hashmap.insert(String::from("Content-Type"), String::from("application/json"));
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("__test__/PNG.png");
        let item_to_store = FetchedObject {
            bytes: Bytes::from(fs::read(d).unwrap()),
            cache_info: hashmap,
            mime: String::from("x"),
        };
        let output = file_cache.set(
            key.clone(),
            item_to_store.clone(),
        );
        assert_eq!(output.unwrap_or(false), true);
        let item_received: FetchedObject = file_cache.get(&key).unwrap();
        assert_eq!(item_to_store.mime, item_received.mime);
        assert_eq!(item_to_store.cache_info, item_received.cache_info);
        assert_eq!(item_to_store.bytes, item_received.bytes);
    }

    #[test]
    fn test_set_get_empty_cache_info_fetched_object() {
        let mut file_cache = FileCache::new(
            &String::from("/tmp/rust-unit-test")
        );
        let key = String::from("lorem.png");
        let mut hashmap: HashMap<String, String> = HashMap::new();
        let item_to_store = FetchedObject {
            bytes: Bytes::from(String::from("test123")),
            cache_info: hashmap,
            mime: String::from("x"),
        };
        let output = file_cache.set(
            key.clone(),
            item_to_store.clone(),
        );
        assert_eq!(output.unwrap_or(false), true);
        let item_received: FetchedObject = file_cache.get(&key).unwrap();
        assert_eq!(item_to_store.mime, item_received.mime);
        assert_eq!(item_to_store.cache_info, item_received.cache_info);
        assert_eq!(item_to_store.bytes, item_received.bytes);
    }

    #[test]
    fn test_set_get_decoded_image() {
        let mut file_cache = FileCache::new(
            &String::from("/tmp/rust-unit-test")
        );
        let key = String::from("lorem.png");
        let mut hashmap: HashMap<String, String> = HashMap::new();
        hashmap.insert(String::from("lorem"), String::from("ipsum"));
        hashmap.insert(String::from("Content-Type"), String::from("application/json"));
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("__test__/PNG.png");
        let mut reader = Reader::new(Cursor::new(
            fs::read(d).unwrap()
        ));
        reader.set_format(ImageFormat::Png);
        let item_to_store = DecodedImage {
            image: reader.decode().unwrap(),
            cache_info: hashmap,
            from: String::from("x"),
        };
        let output = file_cache.set(
            key.clone(),
            item_to_store.clone(),
        );
        assert_eq!(output.unwrap_or(false), true);
        let item_received: DecodedImage = file_cache.get(&key).unwrap();
        assert_eq!(item_to_store.from, item_received.from);
        assert_eq!(item_to_store.cache_info, item_received.cache_info);
        assert_eq!(item_to_store.image.to_bytes(), item_received.image.to_bytes());
    }
}

