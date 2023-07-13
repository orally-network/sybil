use crate::log;

use super::HttpRequest;

pub async fn logger(req: HttpRequest) -> HttpRequest {
    log!("HTTP update request at: {}", req.url);
    req
}
