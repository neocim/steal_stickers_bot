use async_trait::async_trait;
use telers::{
    Request,
    errors::{EventErrorKind, MiddlewareError},
    event::EventReturn,
    middlewares::{OuterMiddleware, outer::MiddlewareResponse},
};

use crate::application::{
    commands::create_user::create_user, common::traits::uow::UoW as UoWTrait,
    user::dto::create::Create,
};

#[derive(Debug, Clone)]
pub struct CreateUserMiddleware<UoW> {
    uow: UoW,
}

impl<UoW> CreateUserMiddleware<UoW>
where
    UoW: UoWTrait,
{
    pub fn new(uow: UoW) -> Self {
        Self { uow: uow }
    }
}

#[async_trait]
impl<UoW> OuterMiddleware for CreateUserMiddleware<UoW>
where
    UoW: UoWTrait + Send + Sync + 'static + Clone,
    for<'a> UoW::UserRepo<'a>: Send + Sync,
{
    async fn call(&mut self, request: Request) -> Result<MiddlewareResponse, EventErrorKind> {
        let user_id = match request.update.from_id() {
            Some(id) => id,
            None => {
                return Ok((request, EventReturn::Skip));
            }
        };

        create_user(&mut self.uow, Create::new(user_id))
            .await
            .map_err(MiddlewareError::new)?;

        Ok((request, EventReturn::default()))
    }
}
