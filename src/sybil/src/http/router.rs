use std::collections::HashMap;

use lazy_static::lazy_static;
use regex::Regex;
use thiserror::Error;

lazy_static! {
    pub static ref ROUTER_KEY_REGEX: Regex =
        Regex::new(r"^([\w/]+:)?[\w/]+(:[\w/]+)?$").expect("invalid regex");
}

#[derive(Error, Debug)]
pub enum RouterError {
    #[error("Invalid key: {0}")]
    InvalidKey(String),
}

#[derive(Debug)]
pub struct Router<H> {
    pub routes: HashMap<String, H>,
}

#[derive(Debug)]
pub struct RouterMatch<'a, H> {
    pub params: String,
    pub value: &'a H,
}

impl<H> Router<H> {
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: &str, value: H) -> Result<(), RouterError> {
        if !ROUTER_KEY_REGEX.is_match(key) {
            return Err(RouterError::InvalidKey(key.to_string()));
        }

        let separated_key = key.split(':').collect::<Vec<&str>>();

        let route = separated_key.first().expect("invalid key").to_string();

        self.routes.insert(route, value);

        Ok(())
    }

    pub fn at(&self, key: &str) -> Option<RouterMatch<H>> {
        let splitted_key = key.split('?').collect::<Vec<&str>>();

        let params = splitted_key.last().expect("invalid key");

        let key = splitted_key.first().expect("invalid key").to_string();

        let value = self.routes.get(&key)?;

        Some(RouterMatch {
            params: params.to_string(),
            value,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router() {
        let mut router = Router::<String> {
            routes: HashMap::new(),
        };

        router.insert("/test:params", "test:1".to_string()).unwrap();

        println!("{:?}", router);

        let route = router.at("/test?param=a").unwrap();
        println!("{:?}", route);
    }
}
