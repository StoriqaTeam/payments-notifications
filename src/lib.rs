#![allow(proc_macro_derive_resolution_fallback)]

#[macro_use]
extern crate failure;
extern crate futures;
#[macro_use]
extern crate diesel;
extern crate env_logger;
extern crate futures_cpupool;
extern crate gelf;
extern crate hyper;
extern crate r2d2;
extern crate serde;
extern crate serde_json;
extern crate serde_qs;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate config as config_crate;
extern crate lapin_async;
extern crate lapin_futures;
#[macro_use]
extern crate http_router;
extern crate base64;
extern crate hyper_tls;
extern crate num;
extern crate rand;
extern crate regex;
extern crate validator;
#[macro_use]
extern crate sentry;
extern crate chrono;
extern crate native_tls;
extern crate secp256k1;
extern crate sha2;
extern crate simplelog;
extern crate tokio;
extern crate tokio_core;
extern crate uuid;

#[macro_use]
mod macros;
mod api;
mod client;
mod config;
mod logger;
mod models;
mod prelude;
mod rabbit;
mod schema;
mod sentry_integration;
mod services;
mod utils;

use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::future::{self, Either};
use tokio::prelude::*;
use tokio::timer::{Delay, Timeout};

use self::client::*;
use self::models::*;
use config::Config;
use rabbit::{RabbitConnectionManager, TransactionConsumerImpl, TransactionPublisherImpl};
use services::Notificator;
use utils::log_error;

pub const DELAY_BEFORE_NACK: u64 = 1000;

pub fn hello() {
    println!("Hello world");
}

pub fn print_config() {
    println!("Parsed config: {:?}", get_config());
}

pub fn start_server() {
    let config = get_config();
    // Prepare sentry integration
    let _sentry = sentry_integration::init(config.sentry.as_ref());
    // Prepare logger
    logger::init(&config);
    let config_clone = config.clone();
    let client = HttpClientImpl::new(&config);
    let ios_client = IosClientImpl::new(&config, client.clone());
    let callback_client = CallbackClientImpl::new(client.clone(), config.client.secp_private_key.clone());
    let email_client = EmailClientImpl::new(&config, client);

    let mut core = tokio_core::reactor::Core::new().unwrap();
    debug!("Started creating rabbit connection pool");

    let rabbit_connection_manager = core
        .run(RabbitConnectionManager::create(&config_clone))
        .map_err(|e| {
            log_error(&e);
        })
        .unwrap();
    debug!("Finished creating rabbit connection pool");
    let consumer = TransactionConsumerImpl::new(rabbit_connection_manager.clone());
    let channel = Arc::new(rabbit_connection_manager.get_channel().expect("Can not get channel from pool"));
    let publisher = core
        .run(TransactionPublisherImpl::init(channel))
        .map_err(|e| {
            log_error(&e);
        })
        .expect("Can not create publisher for transactions in rabbit");
    let publisher = Arc::new(publisher);
    let publisher_clone = publisher.clone();

    let fetcher = Notificator::new(
        Arc::new(ios_client),
        Arc::new(callback_client),
        Arc::new(email_client),
        publisher_clone,
    );
    let consumer_and_chans = core
        .run(consumer.subscribe())
        .expect("Can not create subscribers for transactions in rabbit");
    debug!("Subscribing to rabbit");
    let fetcher_clone = fetcher.clone();
    let timeout = config_clone.rabbit.restart_subscription_secs as u64;
    let futures = consumer_and_chans.into_iter().map(move |(stream, channel, queue)| {
        let fetcher_clone = fetcher_clone.clone();
        stream
            .for_each(move |message| {
                trace!("got message: {}", MessageDelivery::new(message.clone()));
                let delivery_tag = message.delivery_tag;
                let channel = channel.clone();
                let fetcher_future = fetcher_clone.handle_message(message.data, queue.clone());
                let timeout = Duration::from_secs(timeout);
                Timeout::new(fetcher_future, timeout).then(move |res| match res {
                    Ok(_) => Either::A(channel.basic_ack(delivery_tag, false).map_err(|e| {
                        error!("Error sending ack: {}", e);
                        e
                    })),
                    Err(e) => {
                        error!("Error during message handling: {}", e);
                        Either::B(
                            Delay::new(Instant::now() + Duration::from_millis(DELAY_BEFORE_NACK)).then(move |_| {
                                channel.basic_nack(0, true, true).map_err(|e| {
                                    error!("Error sending nack: {}", e);
                                    e
                                })
                            }),
                        )
                    }
                })
            })
            .map_err(|_| ())
    });

    let subscription = future::join_all(futures).map(|_| ());
    core.handle().spawn(subscription);
    api::start_server(core, config);
}

fn get_config() -> Config {
    config::Config::new().unwrap_or_else(|e| panic!("Error parsing config: {}", e))
}
