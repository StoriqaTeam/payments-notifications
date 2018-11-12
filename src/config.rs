use std::env;

use config_crate::{Config as RawConfig, ConfigError, Environment, File};
use logger::{FileLogConfig, GrayLogConfig};
use sentry_integration::SentryConfig;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub server: Server,
    pub database: Database,
    pub client: Client,
    pub cpu_pool: CpuPool,
    pub rabbit: Rabbit,
    pub sentry: Option<SentryConfig>,
    pub graylog: Option<GrayLogConfig>,
    pub filelog: Option<FileLogConfig>,
    pub ios_credentials: IosCredentials,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Client {
    pub dns_threads: usize,
    pub storiqa_url: String,
    pub transactions_url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct IosCredentials {
    pub ios_url: String,
    pub ios_token: String,
    pub ios_user_id: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Server {
    pub host: String,
    pub port: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Database {
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CpuPool {
    pub size: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Rabbit {
    pub url: String,
    pub thread_pool_size: usize,
    pub connection_timeout_secs: usize,
    pub connection_pool_size: usize,
    pub restart_subscription_secs: usize,
}

impl Config {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = RawConfig::new();
        s.merge(File::with_name("config/base"))?;

        // Merge development.toml if RUN_MODE variable is not set
        let env = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());
        s.merge(File::with_name(&format!("config/{}", env)).required(false))?;
        s.merge(File::with_name("config/secret.toml").required(false))?;

        s.merge(Environment::with_prefix("STQ_PAYMENTS"))?;
        s.try_into()
    }
}
