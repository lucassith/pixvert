use serde::{Serialize, Deserialize};

#[derive(Serialize, Debug, Deserialize, PartialEq)]
pub struct OverriddenCache {
    domain: String,
    cache: String,
}

#[derive(Serialize, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Config {
    allow_from: Vec<String>,
    overridden_cache: Vec<OverriddenCache>,
}

impl Default for Config {
    fn default() -> Self {
        Config{
            allow_from: vec![String::from("localhost")],
            overridden_cache: Vec::from(
                vec![
                    OverriddenCache{
                        domain: String::from("localhost"),
                        cache: String::from("immutable"),
                    }
                ]
            ),
        }
    }
}
