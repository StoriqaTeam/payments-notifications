mod error;

use std::fs::File;
use std::io::Read;

use config::Config;

use failure::Fail;
use futures::future::Either;
use futures::prelude::*;
use hyper;
use hyper::{client::HttpConnector, Body, Request, Response};
use hyper_tls::HttpsConnector;
use log::{self, Level};
use native_tls::Identity;
use native_tls::TlsConnector;

use self::error::*;
use utils::read_body;

pub trait HttpClient: Send + Sync + 'static {
    fn request(&self, req: Request<Body>) -> Box<Future<Item = Response<Body>, Error = Error> + Send>;
}

#[derive(Clone)]
pub struct HttpClientImpl {
    cli: hyper::Client<HttpsConnector<HttpConnector>>,
}

impl HttpClientImpl {
    pub fn new(config: &Config) -> Self {
        debug!("Reading public key file {}", &config.client.der_path);
        let mut f = File::open(config.client.der_path.clone()).unwrap();
        let mut der: Vec<u8> = Vec::new();
        f.read_to_end(&mut der).unwrap();
        let identity = Identity::from_pkcs12(&der, &config.client.tls_password).unwrap();
        let tls_connector = TlsConnector::builder()
            .identity(identity)
            .danger_accept_invalid_certs(true)
            .build()
            .unwrap();
        let mut http_connector = HttpConnector::new(config.client.dns_threads);
        http_connector.enforce_http(false);
        let connector = (http_connector, tls_connector).into();
        let cli = hyper::Client::builder().build(connector);
        Self { cli }
    }
}

impl HttpClient for HttpClientImpl {
    fn request(&self, req: Request<Body>) -> Box<Future<Item = Response<Body>, Error = Error> + Send> {
        let cli = self.cli.clone();
        let level = log::max_level();
        let fut = if level == Level::Debug || level == Level::Trace {
            let (parts, body) = req.into_parts();
            Either::A(
                read_body(body)
                    .map_err(ectx!(ErrorSource::Hyper, ErrorKind::Internal))
                    .and_then(move |body| {
                        debug!(
                            "HttpClient, sent request {} {}, headers: {:#?}, body: {:?}",
                            parts.method,
                            parts.uri,
                            parts.headers,
                            String::from_utf8(body.clone()).ok()
                        );
                        let req = Request::from_parts(parts, body.into());
                        cli.request(req).map_err(ectx!(ErrorSource::Hyper, ErrorKind::Internal))
                    })
                    .and_then(|resp| {
                        let (parts, body) = resp.into_parts();
                        read_body(body)
                            .map_err(ectx!(ErrorSource::Hyper, ErrorKind::Internal))
                            .map(|body| (parts, body))
                    })
                    .map(|(parts, body)| {
                        debug!(
                            "HttpClient, recieved response with status {} headers: {:#?} and body: {:?}",
                            parts.status.as_u16(),
                            parts.headers,
                            String::from_utf8(body.clone()).ok()
                        );
                        Response::from_parts(parts, body.into())
                    }),
            )
        } else {
            Either::B(cli.request(req).map_err(ectx!(ErrorSource::Hyper, ErrorKind::Internal)))
        };

        Box::new(fut.and_then(|resp| {
            if resp.status().is_client_error() || resp.status().is_server_error() {
                match resp.status().as_u16() {
                    400 => Err(ectx!(err ErrorSource::Server, ErrorKind::BadRequest)),
                    401 => Err(ectx!(err ErrorSource::Server, ErrorKind::Unauthorized)),
                    404 => Err(ectx!(err ErrorSource::Server, ErrorKind::NotFound)),
                    500 => Err(ectx!(err ErrorSource::Server, ErrorKind::Internal)),
                    502 => Err(ectx!(err ErrorSource::Server, ErrorKind::BadGateway)),
                    504 => Err(ectx!(err ErrorSource::Server, ErrorKind::GatewayTimeout)),
                    _ => Err(ectx!(err ErrorSource::Server, ErrorKind::UnknownServerError)),
                }
            } else {
                Ok(resp)
            }
        }))
    }
}
