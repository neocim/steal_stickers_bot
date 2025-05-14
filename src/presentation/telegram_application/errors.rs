use grammers_client::{
    SignInError,
    client::bots::{AuthorizationError, InvocationError},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    // grammers errors
    #[error(transparent)]
    AuthorizationError(#[from] AuthorizationError),
    #[error(transparent)]
    InvocationError(#[from] InvocationError),
    #[error(transparent)]
    SignInError(#[from] SignInError),

    // other
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
    #[error(transparent)]
    Std(#[from] std::io::Error),
}
