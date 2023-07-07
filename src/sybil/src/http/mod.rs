mod handlers;
mod middlewares;
mod response;

use std::future::Future;
use std::pin::Pin;
use std::sync::OnceLock;

use ic_cdk::{query, update};

use matchit::Router;

use crate::types::http::{HttpRequest, HttpResponse};

type Handler =
    Box<dyn Fn(HttpRequest) -> Pin<Box<dyn Future<Output = HttpResponse>>> + Sync + Send>;
type PreMiddleware =
    Box<dyn Fn(HttpRequest) -> Pin<Box<dyn Future<Output = HttpRequest>>> + Sync + Send>;
type PostMiddleware =
    Box<dyn Fn(HttpResponse) -> Pin<Box<dyn Future<Output = HttpResponse>>> + Sync + Send>;

pub struct HTTPService {
    query_router: Router<Handler>,
    update_router: Router<Handler>,
    pre_query_middlewares: Vec<PreMiddleware>,
    post_query_middlewares: Vec<PostMiddleware>,
    pre_update_middlewares: Vec<PreMiddleware>,
    post_update_middlewares: Vec<PostMiddleware>,
}

pub static HTTP_SERVICE: OnceLock<HTTPService> = OnceLock::new();

pub fn init_http_service() -> HTTPService {
    HTTPService {
        query_router: init_query_router(),
        update_router: init_update_router(),
        pre_query_middlewares: init_pre_query_middlewares(),
        post_query_middlewares: init_post_query_middlewares(),
        pre_update_middlewares: init_pre_update_middlewares(),
        post_update_middlewares: init_post_update_middlewares(),
    }
}

pub fn init_query_router() -> Router<Handler> {
    Router::new()
}

pub fn init_pre_query_middlewares() -> Vec<PreMiddleware> {
    vec![]
}

pub fn init_post_query_middlewares() -> Vec<PostMiddleware> {
    vec![]
}

pub fn init_pre_update_middlewares() -> Vec<PreMiddleware> {
    vec![Box::new(|request| Box::pin(middlewares::logger(request)))]
}

pub fn init_post_update_middlewares() -> Vec<PostMiddleware> {
    vec![]
}

pub fn init_update_router() -> Router<Handler> {
    let mut router: Router<Handler> = Router::new();

    router
        .insert(
            "/get_asset_data/:query",
            Box::new(|request| Box::pin(handlers::get_asset_data_request(request))),
        )
        .expect("Failed to insert handler");

    router
}

pub async fn run_pre_query_middlewares(req: HttpRequest) -> HttpRequest {
    let http_service = HTTP_SERVICE.get().expect("HTTP service not initialized");

    let mut req = req;

    for middleware in &http_service.pre_query_middlewares {
        req = middleware(req).await;
    }

    req
}

pub async fn run_post_query_middlewares(res: HttpResponse) -> HttpResponse {
    let http_service = HTTP_SERVICE.get().expect("HTTP service not initialized");

    let mut res = res;

    for middleware in &http_service.post_query_middlewares {
        res = middleware(res).await;
    }

    res
}

pub async fn run_pre_update_middlewares(req: HttpRequest) -> HttpRequest {
    let http_service = HTTP_SERVICE.get().expect("HTTP service not initialized");

    let mut req = req;

    for middleware in &http_service.pre_update_middlewares {
        req = middleware(req).await;
    }

    req
}

pub async fn run_post_update_middlewares(res: HttpResponse) -> HttpResponse {
    let http_service = HTTP_SERVICE.get().expect("HTTP service not initialized");

    let mut res = res;

    for middleware in &http_service.post_update_middlewares {
        res = middleware(res).await;
    }

    res
}

#[query]
pub async fn http_request(req: HttpRequest) -> HttpResponse {
    let req = run_pre_query_middlewares(req).await;

    let service = HTTP_SERVICE.get().expect("State not initialized");

    if let Ok(route_match) = service.query_router.at(&req.url) {
        let handler = route_match.value;
        let mut responce = handler(req).await;
        responce = run_post_query_middlewares(responce).await;
        return responce;
    }

    response::page_not_found(service.update_router.at(&req.url).is_ok())
}

#[update]
pub async fn http_request_update(req: HttpRequest) -> HttpResponse {
    let req = run_pre_update_middlewares(req).await;

    let service = HTTP_SERVICE.get().expect("State not initialized");

    let handler = service
        .update_router
        .at(&req.url)
        .expect("handler not found")
        .value;

    let mut responce = handler(req).await;
    responce = run_post_update_middlewares(responce).await;
    responce
}
