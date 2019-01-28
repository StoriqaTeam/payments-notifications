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
use std::thread;
use std::time::{Duration, Instant};

use futures::future::{self, Either};
use tokio::prelude::*;
use tokio::runtime::Runtime;
use tokio::timer::{Delay, Timeout};

use self::client::*;
use self::models::*;
use self::prelude::*;
use config::Config;
use rabbit::{Error, ErrorKind};
use rabbit::{RabbitConnectionManager, TransactionConsumerImpl, TransactionPublisherImpl};
use services::Notificator;
use utils::log_error;

pub const DELAY_BEFORE_NACK: u64 = 1000;
pub const DELAY_BEFORE_RECONNECT: u64 = 1000;

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

    let rabbit_thread_pool = futures_cpupool::CpuPool::new(config_clone.rabbit.thread_pool_size);
    let rabbit_connection_manager = core
        .run(RabbitConnectionManager::create(&config_clone))
        .map_err(|e| {
            log_error(&e);
        })
        .unwrap();
    let rabbit_connection_pool = r2d2::Pool::builder()
        .max_size(config_clone.rabbit.connection_pool_size as u32)
        .test_on_check_out(false)
        .max_lifetime(None)
        .idle_timeout(None)
        .build(rabbit_connection_manager)
        .expect("Cannot build rabbit connection pool");
    debug!("Finished creating rabbit connection pool");
    let consumer = TransactionConsumerImpl::new(rabbit_connection_pool.clone(), rabbit_thread_pool.clone());
    let mut publisher = TransactionPublisherImpl::new(rabbit_connection_pool, rabbit_thread_pool);
    core.run(publisher.init())
        .map_err(|e| {
            log_error(&e);
        })
        .unwrap();
    let publisher = Arc::new(publisher);
    let publisher_clone = publisher.clone();

    let fetcher = Notificator::new(
        Arc::new(ios_client),
        Arc::new(callback_client),
        Arc::new(email_client),
        publisher_clone,
    );
    thread::spawn(move || {
        let mut core = Runtime::new().expect("Can not create tokio core");
        let consumer_and_chans = core
            .block_on(consumer.subscribe())
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
                    Timeout::new(fetcher_future, timeout).then(move |res| {
                        trace!("send result: {:?}", res);
                        match res {
                            Ok(_) => Either::A(channel.basic_ack(delivery_tag, false)),
                            Err(e) => {
                                let when = if let Some(inner) = e.into_inner() {
                                    log_error(&inner);
                                    Instant::now() + Duration::from_millis(DELAY_BEFORE_NACK)
                                } else {
                                    let err: Error = ectx!(err format_err!("Timeout occured"), ErrorKind::Internal);
                                    log_error(&err);
                                    Instant::now() + Duration::from_millis(0)
                                };
                                Either::B(Delay::new(when).then(move |_| {
                                    channel.basic_nack(delivery_tag, false, true).map_err(|e| {
                                        error!("Error sending nack: {}", e);
                                        e
                                    })
                                }))
                            }
                        }
                    })
                })
                .map_err(|_| ())
        });

        let subscription = future::join_all(futures);
        let _ = core.block_on(subscription);
    });

    api::start_server(config);
}

fn get_config() -> Config {
    config::Config::new().unwrap_or_else(|e| panic!("Error parsing config: {}", e))
}
