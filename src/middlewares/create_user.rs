use async_trait::async_trait;
use telers::{
    Request,
    errors::{EventErrorKind, MiddlewareError},
    event::EventReturn,
    middlewares::{OuterMiddleware, outer::MiddlewareResponse},
};

use crate::application::{
    common::traits::uow::{UoW as UoWTrait, UoWFactory},
    interactors::create_user::create_user,
    user::dto::create::Create,
};

#[derive(Debug, Clone)]
pub struct CreateUserMiddleware<UoWF> {
    uowf: UoWF,
}

impl<UoWF> CreateUserMiddleware<UoWF>
where
    UoWF: UoWFactory,
{
    pub const fn new(uowf: UoWF) -> Self {
        Self { uowf }
    }
}

#[async_trait]
impl<UoWF> OuterMiddleware for CreateUserMiddleware<UoWF>
where
    UoWF: UoWFactory + Send + Sync + 'static + Clone,
    for<'a> UoWF::UoW: Send + Sync,
    for<'a> <UoWF::UoW as UoWTrait>::UserRepo<'a>: Send + Sync,
{
    async fn call(&mut self, request: Request) -> Result<MiddlewareResponse, EventErrorKind> {
        let user_id = match request.update.from_id() {
            Some(id) => id,
            None => {
                return Ok((request, EventReturn::Skip));
            }
        };

        create_user(&mut self.uowf.create_uow(), Create::new(user_id))
            .await
            .map_err(MiddlewareError::new)?;

        Ok((request, EventReturn::default()))
    }
}
