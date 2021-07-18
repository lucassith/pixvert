use std::sync::Arc;

pub trait Service {
    fn can_be_used(&self, resource: &String) -> bool;
}

pub struct ServiceProvider<T: ?Sized> where
    T: Service + Sync + Send {
    services: Vec<Arc<Box<T>>>
}

impl<T: ?Sized> ServiceProvider<T> where
    T: Service + Sync + Send {
    pub fn new(services: Vec<Arc<Box<T>>>) -> ServiceProvider<T> {
        ServiceProvider{
            services
        }
    }

    pub fn get(&self, link: &String) -> Option<Arc<Box<T>>> {
        for service in self.services.iter() {
            if service.can_be_used(link) {
                return Option::Some(service.clone());
            }
        }
        Option::None
    }
}

