use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct TaggedElement<T: Clone> {
    pub object: T,
    pub cache_data: HashMap<String, String>
}
