mod handlers;
mod middlewares;
mod response;
mod router;

use std::future::Future;
use std::pin::Pin;
use std::sync::OnceLock;

use ic_cdk::{query, update};

use router::Router;

use crate::types::http::{HttpRequest, HttpResponse};

pub static HTTP_SERVICE: OnceLock<HttpService> = OnceLock::new();

type Handler =
    Box<dyn Fn(HttpRequest) -> Pin<Box<dyn Future<Output = HttpResponse>>> + Sync + Send>;
type PreMiddleware =
    Box<dyn Fn(HttpRequest) -> Pin<Box<dyn Future<Output = HttpRequest>>> + Sync + Send>;
type PostMiddleware =
    Box<dyn Fn(HttpResponse) -> Pin<Box<dyn Future<Output = HttpResponse>>> + Sync + Send>;

pub struct HttpRouter {
    inner: Router<Handler>,
    pre_middlewares: Vec<PreMiddleware>,
    post_middlewares: Vec<PostMiddleware>,
}

pub struct HttpService {
    query_router: HttpRouter,
    update_router: HttpRouter,
}

impl HttpService {
    pub fn init() {
        let service = Self {
            query_router: Self::init_query_router(),
            update_router: Self::init_update_router(),
        };

        HTTP_SERVICE.get_or_init(|| service);
    }

    pub fn init_query_router() -> HttpRouter {
        let router = Router::<Handler>::new();

        let pre_middlewares: Vec<PreMiddleware> = vec![];

        let post_middlewares: Vec<PostMiddleware> = vec![];

        HttpRouter {
            inner: router,
            pre_middlewares,
            post_middlewares,
        }
    }

    pub fn init_update_router() -> HttpRouter {
        let mut router = Router::<Handler>::new();
        router
            .insert(
                "/get_asset_data:query",
                Box::new(|request| Box::pin(handlers::get_asset_data_request(request))),
            )
            .expect("Failed to insert handler");

        router
            .insert(
                "/get_asset_data_with_proof:query",
                Box::new(|request| Box::pin(handlers::get_asset_data_with_proof_request(request))),
            )
            .expect("Failed to insert handler");

        let pre_middlewares: Vec<PreMiddleware> =
            vec![Box::new(|request| Box::pin(middlewares::logger(request)))];

        let post_middlewares: Vec<PostMiddleware> =
            vec![Box::new(|resp| Box::pin(middlewares::cors(resp)))];

        HttpRouter {
            inner: router,
            pre_middlewares,
            post_middlewares,
        }
    }
}

impl HttpRouter {
    pub async fn run_pre_middlewares(&self, mut req: HttpRequest) -> HttpRequest {
        for middleware in &self.pre_middlewares {
            req = middleware(req).await;
        }

        req
    }

    pub async fn run_post_middlewares(&self, mut res: HttpResponse) -> HttpResponse {
        for middleware in &self.post_middlewares {
            res = middleware(res).await;
        }

        res
    }
}

#[query]
pub async fn http_request(req: HttpRequest) -> HttpResponse {
    let service = HTTP_SERVICE.get().expect("HTTP service not initialized");

    let req = service.query_router.run_pre_middlewares(req).await;

    if let Some(route_match) = service.query_router.inner.at(&req.url) {
        let handler = route_match.value;
        let response = handler(req).await;
        return service.query_router.run_post_middlewares(response).await;
    }

    response::page_not_found(service.update_router.inner.at(&req.url).is_some())
}

#[update]
pub async fn http_request_update(req: HttpRequest) -> HttpResponse {
    let service = HTTP_SERVICE.get().expect("HTTP service not initialized");

    let req = service.update_router.run_pre_middlewares(req).await;

    let handler = service
        .update_router
        .inner
        .at(&req.url)
        .expect("handler not found")
        .value;
    let response = handler(req).await;

    service.update_router.run_post_middlewares(response).await
}
