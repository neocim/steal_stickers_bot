use grammers_client::{InvocationError, SignInError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    // grammers errors
    #[error(transparent)]
    SignInError(#[from] SignInError),
    #[error(transparent)]
    InvocationError(#[from] InvocationError),
    // other
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
    #[error(transparent)]
    Std(#[from] std::io::Error),
}
