use reqwest::Url;

pub trait Fetch {
    fn fetch_image() -> bool;
}

impl Fetch for Url {
    fn fetch_image() -> bool {
        todo!()
    }
}
