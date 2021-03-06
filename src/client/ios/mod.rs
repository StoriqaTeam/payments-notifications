mod error;

use std::sync::Arc;

use failure::Fail;
use futures::prelude::*;
use hyper::Method;
use hyper::{Body, Request};
use models::*;
use serde::Deserialize;
use serde_json;

pub use self::error::*;
use super::HttpClient;
use config::Config;
use utils::read_body;

pub trait IosClient: Send + Sync + 'static {
    fn push_notify(&self, push: PushNotifications) -> Box<Future<Item = (), Error = Error> + Send>;
}

#[derive(Clone)]
pub struct IosClientImpl {
    cli: Arc<HttpClient>,
    ios_url: String,
    ios_token: String,
    ios_user_id: String,
}

impl IosClientImpl {
    pub fn new<C: HttpClient>(config: &Config, cli: C) -> Self {
        Self {
            cli: Arc::new(cli),
            ios_url: config.ios_credentials.ios_url.clone(),
            ios_token: config.ios_credentials.ios_token.clone(),
            ios_user_id: config.ios_credentials.ios_user_id.clone(),
        }
    }

    fn exec_query<T: for<'de> Deserialize<'de> + Send>(&self, body: String) -> impl Future<Item = T, Error = Error> + Send {
        let query = self.ios_url.to_string();
        let query1 = query.clone();
        let query2 = query.clone();
        let query3 = query.clone();
        let cli = self.cli.clone();
        let mut builder = Request::builder();
        builder.uri(query).method(Method::POST);
        builder
            .body(Body::from(body))
            .map_err(ectx!(ErrorSource::Hyper, ErrorKind::MalformedInput => query3))
            .into_future()
            .and_then(move |req| cli.request(req).map_err(ectx!(ErrorKind::Internal => query1)))
            .and_then(move |resp| read_body(resp.into_body()).map_err(ectx!(ErrorSource::Hyper, ErrorKind::Internal => query2)))
            .and_then(|bytes| {
                let bytes_clone = bytes.clone();
                String::from_utf8(bytes).map_err(ectx!(ErrorSource::Utf8, ErrorKind::Internal => bytes_clone))
            })
            .and_then(|string| serde_json::from_str::<T>(&string).map_err(ectx!(ErrorSource::Json, ErrorKind::Internal => string)))
    }
}

impl IosClient for IosClientImpl {
    fn push_notify(&self, push: PushNotifications) -> Box<Future<Item = (), Error = Error> + Send> {
        let client = self.clone();
        Box::new(
            serde_json::to_string(&push)
                .map_err(ectx!(ErrorSource::Json, ErrorKind::Internal => push))
                .into_future()
                .and_then(move |body| client.exec_query::<()>(body)),
        )
    }
}

#[derive(Default)]
pub struct IosClientMock;

impl IosClient for IosClientMock {
    fn push_notify(&self, _push: PushNotifications) -> Box<Future<Item = (), Error = Error> + Send> {
        Box::new(Ok(()).into_future())
    }
}
