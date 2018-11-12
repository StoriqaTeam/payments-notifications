use futures::future;
use futures_cpupool::CpuPool;
use lapin_futures::channel::BasicConsumeOptions;
use lapin_futures::channel::Channel;
use lapin_futures::channel::QueueDeclareOptions;
use lapin_futures::consumer::Consumer;
use lapin_futures::types::FieldTable;
use r2d2::PooledConnection;
use tokio::net::tcp::TcpStream;

use super::error::*;
use super::r2d2::RabbitConnectionManager;
use super::r2d2::RabbitPool;
use prelude::*;

#[derive(Clone)]
pub struct TransactionConsumerImpl {
    rabbit_pool: RabbitPool,
    thread_pool: CpuPool,
}

impl TransactionConsumerImpl {
    pub fn new(rabbit_pool: RabbitPool, thread_pool: CpuPool) -> Self {
        Self { rabbit_pool, thread_pool }
    }

    pub fn subscribe(&self) -> impl Future<Item = Vec<(Consumer<TcpStream>, Channel<TcpStream>, String)>, Error = Error> {
        let self_clone = self.clone();
        let fs = vec!["pushes".to_string(), "callbacks".to_string()]
            .into_iter()
            .map(move |queue_name| {
                let self_clone2 = self_clone.clone();
                self_clone
                    .get_channel()
                    .and_then(move |channel| self_clone2.subscribe_for_currency(&channel, queue_name))
            });
        future::join_all(fs)
    }

    fn get_channel(&self) -> impl Future<Item = PooledConnection<RabbitConnectionManager>, Error = Error> {
        // unresolved at the moment - ideally we want to call get on other thread, since it's blocking
        // on the other hand doing so we escape from the thread that has tokio core reference and
        // therefore cannot do spawns
        // let rabbit_pool = self.rabbit_pool.clone();
        // self.thread_pool
        //     .spawn_fn(move || rabbit_pool.get().map_err(ectx!(ErrorSource::Lapin, ErrorKind::Internal)))
        self.rabbit_pool
            .get()
            .map_err(ectx!(ErrorSource::Lapin, ErrorKind::Internal))
            .into_future()
    }

    fn subscribe_for_currency(
        &self,
        channel: &Channel<TcpStream>,
        queue_name: String,
    ) -> impl Future<Item = (Consumer<TcpStream>, Channel<TcpStream>, String), Error = Error> {
        let channel_clone = channel.clone();
        let queue_name_clone = queue_name.clone();
        channel
            .queue_declare(
                &queue_name,
                QueueDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                Default::default(),
            ).map_err(ectx!(ErrorSource::Lapin, ErrorKind::Internal))
            .and_then(move |queue| {
                channel_clone
                    .basic_consume(&queue, "", BasicConsumeOptions::default(), FieldTable::new())
                    .map(move |consumer| (consumer, channel_clone, queue_name_clone))
                    .map_err(ectx!(ErrorSource::Lapin, ErrorKind::Internal))
            })
    }
}
