use crate::log;

use super::{HttpRequest, HttpResponse};

pub async fn logger(req: HttpRequest) -> HttpRequest {
    log!("HTTP update request at: {}", req.url);
    req
}

pub async fn cors(mut res: HttpResponse) -> HttpResponse {
    res.headers
        .push(("Access-Control-Allow-Origin".into(), "*".into()));
    res.headers
        .push(("Access-Control-Allow-Methods".into(), "GET".into()));
    res.headers
        .push(("Access-Control-Allow-Headers".into(), "Content-Type".into()));
    res.headers
        .push(("Access-Control-Max-Age".into(), "86400".into()));
    res
}
