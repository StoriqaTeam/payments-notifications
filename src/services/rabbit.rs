use std::sync::Arc;

use futures::future;
use serde::Deserialize;
use serde_json;

use super::error::*;
use client::{CallbackClient, IosClient};
use models::*;
use prelude::*;

#[derive(Clone)]
pub struct Notificator {
    ios_client: Arc<dyn IosClient>,
    callback_client: Arc<dyn CallbackClient>,
}

impl Notificator {
    pub fn new(ios_client: Arc<dyn IosClient>, callback_client: Arc<dyn CallbackClient>) -> Self {
        Self {
            ios_client,
            callback_client,
        }
    }
}

impl Notificator {
    pub fn handle_message(&self, data: Vec<u8>, queue_name: String) -> impl Future<Item = (), Error = Error> + Send {
        let self_clone = self.clone();
        match &*queue_name {
            "pushes" => Box::new(
                parse::<PushNotifications>(data)
                    .into_future()
                    .and_then(move |push| self_clone.send_push(push)),
            ) as Box<Future<Item = (), Error = Error> + Send>,
            "callbacks" => Box::new(
                parse::<Callback>(data)
                    .into_future()
                    .and_then(move |callback| self_clone.send_callback(callback)),
            ) as Box<Future<Item = (), Error = Error> + Send>,
            _ => Box::new(future::err(
                ectx!(err ErrorContext::NotSupported, ErrorKind::Internal => queue_name),
            )) as Box<Future<Item = (), Error = Error> + Send>,
        }
    }

    fn send_push(&self, push: PushNotifications) -> impl Future<Item = (), Error = Error> + Send {
        self.ios_client.push_notify(push.clone()).map_err(ectx!(convert => push))
    }

    fn send_callback(&self, callback: Callback) -> impl Future<Item = (), Error = Error> + Send {
        self.callback_client.send(callback.clone()).map_err(ectx!(convert => callback))
    }
}

fn parse<T>(data: Vec<u8>) -> Result<T, Error>
where
    T: for<'de> Deserialize<'de> + Send,
{
    let data_clone = data.clone();
    let string = String::from_utf8(data).map_err(|e| ectx!(try err e, ErrorContext::UTF8, ErrorKind::Internal => data_clone))?;
    serde_json::from_str(&string).map_err(ectx!(ErrorContext::Json, ErrorKind::Internal => string))
}
