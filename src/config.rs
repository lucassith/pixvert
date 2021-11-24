use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OverriddenCache {
    pub domain: String,
    pub cache_control: String,
}

#[derive(Serialize, Debug, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub allow_from: Vec<String>,
    pub overridden_cache: Vec<OverriddenCache>,
}

impl Default for Config {
    fn default() -> Self {
        Config{
            allow_from: vec![String::from("localhost")],
            overridden_cache: Vec::from(
                vec![
                    OverriddenCache{
                        domain: String::from("localhost"),
                        cache_control: String::from("immutable"),
                    }
                ]
            ),
        }
    }
}
