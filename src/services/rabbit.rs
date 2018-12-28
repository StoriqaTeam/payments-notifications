use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::future::{self, Either};
use futures::stream::{self, Stream};
use serde::Deserialize;
use serde_json;
use tokio::timer::Delay;

use super::error::*;
use client::{CallbackClient, EmailClient, IosClient};
use models::*;
use prelude::*;
use rabbit::TransactionPublisher;
use utils::log_error;

#[derive(Clone)]
pub struct Notificator {
    ios_client: Arc<dyn IosClient>,
    callback_client: Arc<dyn CallbackClient>,
    email_client: Arc<dyn EmailClient>,
    publisher: Arc<dyn TransactionPublisher>,
}

impl Notificator {
    pub fn new(
        ios_client: Arc<dyn IosClient>,
        callback_client: Arc<dyn CallbackClient>,
        email_client: Arc<dyn EmailClient>,
        publisher: Arc<dyn TransactionPublisher>,
    ) -> Self {
        Self {
            ios_client,
            callback_client,
            email_client,
            publisher,
        }
    }
}

impl Notificator {
    pub fn handle_message(&self, data: Vec<u8>, queue_name: String) -> impl Future<Item = (), Error = Error> + Send {
        let self_clone = self.clone();
        match &*queue_name {
            "pushes" => Box::new(future::ok(())), // Push notifications are temporarily disabled
            "callbacks" => Box::new(
                parse::<Callback>(data)
                    .into_future()
                    .and_then(move |callback| self_clone.send_callback(callback)),
            ) as Box<Future<Item = (), Error = Error> + Send>,
            "emails" => Box::new(
                parse::<Email>(data)
                    .into_future()
                    .and_then(move |email| self_clone.send_email(email)),
            ) as Box<Future<Item = (), Error = Error> + Send>,
            _ => Box::new(future::err(
                ectx!(err ErrorContext::NotSupported, ErrorKind::Internal => queue_name),
            )) as Box<Future<Item = (), Error = Error> + Send>,
        }
    }

    // Push notifications are temporarily disabled
    #[allow(dead_code)]
    fn send_push(&self, push: PushNotifications) -> impl Future<Item = (), Error = Error> + Send {
        let ios_client = self.ios_client.clone();
        let publisher = self.publisher.clone();
        let push_clone2 = push.clone();
        stream::iter_ok::<_, ()>(vec![2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048])
            .for_each(move |delay| {
                let push_clone = push.clone();
                ios_client
                    .push_notify(push.clone())
                    .map_err(ectx!(convert => push_clone))
                    .then(move |res: Result<(), Error>| match res {
                        Ok(_) => Either::A(future::err(())),
                        Err(e) => {
                            log_error(&e);
                            Either::B(Delay::new(Instant::now() + Duration::from_secs(delay)).map_err(|_| ()))
                        }
                    })
            })
            .and_then(move |_| {
                publisher.error_pushes(push_clone2.clone()).map_err(|e| {
                    log_error(&e);
                    ()
                })
            })
            .then(|_| future::ok(()))
    }

    fn send_email(&self, email: Email) -> impl Future<Item = (), Error = Error> + Send {
        let email_client = self.email_client.clone();
        let publisher = self.publisher.clone();
        let email_clone2 = email.clone();
        stream::iter_ok::<_, ()>(vec![2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048])
            .for_each(move |delay| {
                let email_clone = email.clone();
                email_client
                    .send(email.clone())
                    .map_err(ectx!(convert => email_clone))
                    .then(move |res: Result<(), Error>| match res {
                        Ok(_) => Either::A(future::err(())),
                        Err(e) => {
                            log_error(&e);
                            Either::B(Delay::new(Instant::now() + Duration::from_secs(delay)).map_err(|_| ()))
                        }
                    })
            })
            .and_then(move |_| {
                publisher.error_emails(email_clone2.clone()).map_err(|e| {
                    log_error(&e);
                    ()
                })
            })
            .then(|_| future::ok(()))
    }

    fn send_callback(&self, callback: Callback) -> impl Future<Item = (), Error = Error> + Send {
        let callback_client = self.callback_client.clone();
        let publisher = self.publisher.clone();
        let callback_clone2 = callback.clone();
        stream::iter_ok::<_, ()>(vec![2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048])
            .for_each(move |delay| {
                let callback_clone = callback.clone();
                callback_client
                    .send(callback.clone())
                    .map_err(ectx!(convert => callback_clone))
                    .then(move |res: Result<(), Error>| match res {
                        Ok(_) => Either::A(future::err(())),
                        Err(e) => {
                            log_error(&e);
                            Either::B(Delay::new(Instant::now() + Duration::from_secs(delay)).map_err(|_| ()))
                        }
                    })
            })
            .and_then(move |_| {
                publisher.error_callbacks(callback_clone2.clone()).map_err(|e| {
                    log_error(&e);
                    ()
                })
            })
            .then(|_| future::ok(()))
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
