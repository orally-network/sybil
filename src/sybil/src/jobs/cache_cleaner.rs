use thiserror::Error;

use crate::{log, HTTP_CACHE, SIGNATURES_CACHE};

#[derive(Error, Debug)]
pub enum CacheCleanerError {}

pub fn execute() {
    ic_cdk::spawn(async {
        if let Err(e) = _execute().await {
            log!("[CACHE CLEANER] error while executing cache cleaner job: {e:?}");
        }
    })
}

#[inline(always)]
async fn _execute() -> Result<(), CacheCleanerError> {
    log!("[CACHE CLEANER] cache cleaner job started");
    HTTP_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.clean();
    });
    SIGNATURES_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.clean();
    });
    log!("[CACHE CLEANER] cache cleaner job stopped");
    Ok(())
}
