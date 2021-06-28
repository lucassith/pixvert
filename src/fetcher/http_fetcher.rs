use super::Fetchable;
use async_trait::async_trait;
use bytes::Bytes;
use std::error::Error;
use reqwest::Url;

pub struct HttpFetcher {
    reqwest: reqwest::Client
}

impl HttpFetcher {
    pub fn new() -> HttpFetcher {
        HttpFetcher{
            reqwest: reqwest::Client::new()
        }
    }
}

#[async_trait]
impl Fetchable for HttpFetcher {
    fn can_fetch(&self, link: &String) -> bool {
        Url::parse(link).is_ok()
    }

    async fn fetch(&self, link: &String) -> Result<Bytes, Box<dyn Error>> {
        let response = self.reqwest.get(link)
            .send()
            .await?
            .bytes()
            .await?;
        Result::Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;    

    #[actix_rt::test]
    async fn test_fetch_image() {
        let fetcher = HttpFetcher::new();
        let image = fetcher.fetch(&String::from("https://via.placeholder.com/150")).await;

        println!("{:#?}", &image.unwrap());
    }

    #[test]
    fn test_can_fetch() {
        let fetcher = HttpFetcher::new();
        
        assert!(fetcher.can_fetch(&"https://valid.com".to_string()));
        assert!(!fetcher.can_fetch(&"invalid".to_string()));
    }
}