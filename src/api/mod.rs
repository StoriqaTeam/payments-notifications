use std::net::SocketAddr;

use failure::{Compat, Fail};
use futures::prelude::*;
use hyper;
use hyper::Server;
use hyper::{service::Service, Body, Request, Response};

use super::config::Config;
use super::utils::{log_and_capture_error, log_error, log_warn};
use utils::read_body;

mod controllers;
mod error;
mod utils;

use self::controllers::*;
use self::error::*;

#[derive(Clone)]
pub struct ApiService {
    server_address: SocketAddr,
    config: Config,
}

impl ApiService {
    fn from_config(config: &Config) -> Result<Self, Error> {
        let server_address = format!("{}:{}", config.server.host, config.server.port)
            .parse::<SocketAddr>()
            .map_err(ectx!(try
                ErrorContext::Config,
                ErrorKind::Internal =>
                config.server.host,
                config.server.port
            ))?;
        Ok(ApiService {
            config: config.clone(),
            server_address,
        })
    }
}

impl Service for ApiService {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = Compat<Error>;
    type Future = Box<Future<Item = Response<Body>, Error = Self::Error> + Send>;

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let (parts, http_body) = req.into_parts();

        Box::new(
            read_body(http_body)
                .map_err(ectx!(ErrorSource::Hyper, ErrorKind::Internal))
                .and_then(move |body| {
                    let router = router! {
                        _ => not_found,
                    };

                    let ctx = Context {
                        body,
                        method: parts.method.clone(),
                        uri: parts.uri.clone(),
                        headers: parts.headers,
                    };

                    debug!("Received request {}", ctx);
                    router(ctx, parts.method.into(), parts.uri.path())
                })
                .and_then(|resp| {
                    let (parts, body) = resp.into_parts();
                    read_body(body)
                        .map_err(ectx!(ErrorSource::Hyper, ErrorKind::Internal))
                        .map(|body| (parts, body))
                })
                .map(|(parts, body)| {
                    debug!(
                        "Sent response with status {}, headers: {:#?}, body: {:?}",
                        parts.status.as_u16(),
                        parts.headers,
                        String::from_utf8(body.clone()).ok()
                    );
                    Response::from_parts(parts, body.into())
                })
                .or_else(|e| match e.kind() {
                    ErrorKind::BadRequest => {
                        log_error(&e);
                        Ok(Response::builder()
                            .status(400)
                            .header("Content-Type", "application/json")
                            .body(Body::from(r#"{"description": "Bad request"}"#))
                            .unwrap())
                    }
                    ErrorKind::Unauthorized => {
                        log_warn(&e);
                        Ok(Response::builder()
                            .status(401)
                            .header("Content-Type", "application/json")
                            .body(Body::from(r#"{"description": "Unauthorized"}"#))
                            .unwrap())
                    }
                    ErrorKind::NotFound => {
                        log_warn(&e);
                        Ok(Response::builder()
                            .status(404)
                            .header("Content-Type", "application/json")
                            .body(Body::from(r#"{"description": "Not found"}"#))
                            .unwrap())
                    }
                    ErrorKind::UnprocessableEntity(errors) => {
                        log_warn(&e);
                        Ok(Response::builder()
                            .status(422)
                            .header("Content-Type", "application/json")
                            .body(Body::from(errors))
                            .unwrap())
                    }
                    ErrorKind::Internal => {
                        log_and_capture_error(e);
                        Ok(Response::builder()
                            .status(500)
                            .header("Content-Type", "application/json")
                            .body(Body::from(r#"{"description": "Internal server error"}"#))
                            .unwrap())
                    }
                }),
        )
    }
}

pub fn server(config: Config) -> Box<Future<Item = (), Error = ()> + Send> {
    let fut = ApiService::from_config(&config)
        .into_future()
        .and_then(move |api| {
            let api_clone = api.clone();
            let new_service = move || {
                let res: Result<_, hyper::Error> = Ok(api_clone.clone());
                res
            };
            let addr = api.server_address.clone();
            let server = Server::bind(&api.server_address)
                .serve(new_service)
                .map_err(ectx!(ErrorSource::Hyper, ErrorKind::Internal => addr));
            info!("Listening on http://{}", addr);
            server
        })
        .map_err(|e: Error| log_error(&e))
        .map(|_| ());

    Box::new(fut)
}
