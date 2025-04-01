use std::ops::DerefMut;
use tokio::sync::RwLock;

use telers::{
    errors::{EventErrorKind, MiddlewareError},
    event::EventReturn,
    middlewares::{outer::MiddlewareResponse, OuterMiddleware},
    Request,
};

use async_trait::async_trait;

use crate::application::{
    commands::create_user::create_user, common::traits::uow::UoW as UoWTrait,
    user::dto::create::Create,
};

#[derive(Debug)]
pub struct CreateUserMiddleware<UoW> {
    uow: RwLock<UoW>,
}

impl<UoW> CreateUserMiddleware<UoW>
where
    UoW: UoWTrait,
{
    pub fn new(uow: UoW) -> Self {
        Self {
            uow: RwLock::new(uow),
        }
    }
}

#[async_trait]
impl<UoW> OuterMiddleware for CreateUserMiddleware<UoW>
where
    UoW: UoWTrait + Send + Sync,
    for<'a> UoW::UserRepo<'a>: Send + Sync,
{
    async fn call(&self, request: Request) -> Result<MiddlewareResponse, EventErrorKind> {
        let mut uow = self.uow.write().await;

        let user_id = match request.update.from_id() {
            Some(id) => id,
            None => {
                return Ok((request, EventReturn::Skip));
            }
        };

        let uow = uow.deref_mut();

        create_user(uow, Create::new(user_id))
            .await
            .map_err(MiddlewareError::new)?;

        Ok((request, EventReturn::default()))
    }
}
