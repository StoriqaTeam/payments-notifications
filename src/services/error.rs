use std::fmt;
use std::fmt::Display;

use failure::{Backtrace, Context, Fail};
use validator::ValidationErrors;

use client::callback::ErrorKind as CallbackClientErrorKind;
use client::email::ErrorKind as EmailClientErrorKind;
use client::ios::ErrorKind as IosClientErrorKind;

#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Fail)]
pub enum ErrorKind {
    #[fail(display = "service error - unauthorized")]
    Unauthorized,
    #[fail(display = "service error - malformed input")]
    MalformedInput,
    #[fail(display = "service error - invalid input, errors: {}", _0)]
    InvalidInput(ValidationErrors),
    #[fail(display = "service error - internal error")]
    Internal,
    #[fail(display = "service error - not found")]
    NotFound,
    #[fail(display = "service error - balance failure")]
    Balance,
    #[fail(display = "service error - fake error")]
    Fake,
}

#[allow(dead_code)]
#[derive(Copy, Clone, Eq, PartialEq, Debug, Fail)]
pub enum ErrorSource {
    #[fail(display = "service error source - r2d2")]
    R2D2,
    #[fail(display = "service error source - repos")]
    Repo,
}

#[allow(dead_code)]
#[derive(Copy, Clone, Eq, PartialEq, Debug, Fail)]
pub enum ErrorContext {
    #[fail(display = "service error context - no auth token received")]
    NoAuthToken,
    #[fail(display = "service error context - invalid auth token")]
    InvalidToken,
    #[fail(display = "service error context - no account found")]
    NoAccount,
    #[fail(display = "service error context - no transaction found")]
    NoTransaction,
    #[fail(display = "service error context - not enough funds")]
    NotEnoughFunds,
    #[fail(display = "service error context - invalid currency")]
    InvalidCurrency,
    #[fail(display = "service error context - exchange rate is required, but not found")]
    MissingExchangeRate,
    #[fail(display = "service error context - invalid utf8 bytes")]
    UTF8,
    #[fail(display = "service error context - failed to parse string to json")]
    Json,
    #[fail(display = "service error context - balance overflow")]
    BalanceOverflow,
    #[fail(display = "service error context - transaction between two dr accounts")]
    InvalidTransaction,
    #[fail(display = "service error context - invalid uuid")]
    InvalidUuid,
    #[fail(display = "service error context - operation not yet supproted")]
    NotSupported,
    #[fail(display = "service error context - tokio timer error")]
    Timer,
    #[fail(display = "service error context - rabbit error")]
    Lapin,
    #[fail(display = "service error context - fake error")]
    Fake,
}

derive_error_impls!();

impl From<IosClientErrorKind> for ErrorKind {
    fn from(err: IosClientErrorKind) -> Self {
        match err {
            IosClientErrorKind::Internal => ErrorKind::Internal,
            IosClientErrorKind::Unauthorized => ErrorKind::Unauthorized,
            IosClientErrorKind::MalformedInput => ErrorKind::MalformedInput,
        }
    }
}

impl From<CallbackClientErrorKind> for ErrorKind {
    fn from(err: CallbackClientErrorKind) -> Self {
        match err {
            CallbackClientErrorKind::Internal => ErrorKind::Internal,
            CallbackClientErrorKind::Unauthorized => ErrorKind::Unauthorized,
            CallbackClientErrorKind::MalformedInput => ErrorKind::MalformedInput,
        }
    }
}

impl From<EmailClientErrorKind> for ErrorKind {
    fn from(err: EmailClientErrorKind) -> Self {
        match err {
            EmailClientErrorKind::Internal => ErrorKind::Internal,
            EmailClientErrorKind::Unauthorized => ErrorKind::Unauthorized,
            EmailClientErrorKind::MalformedInput => ErrorKind::MalformedInput,
        }
    }
}
