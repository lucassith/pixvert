use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct TaggedElement<T> where
        for<'de> T: Deserialize<'de> + Serialize {
    pub object: T,
    pub cache_data: HashMap<String, String>
}
