use telers::{
    errors::EventErrorKind,
    event::EventReturn,
    middlewares::{outer::MiddlewareResponse, OuterMiddleware},
    Request,
};

use async_trait::async_trait;

use crate::application::common::traits::uow::UoWFactory as UoWFactoryTrait;

#[derive(Debug)]
pub struct DatabaseMiddleware<UoWFactory> {
    uow_factory: UoWFactory,
}

impl<UoWFactory> DatabaseMiddleware<UoWFactory> {
    pub const fn new(uow_factory: UoWFactory) -> Self {
        Self { uow_factory }
    }
}

#[async_trait]
impl<UoWFactory> OuterMiddleware for DatabaseMiddleware<UoWFactory>
where
    UoWFactory: Send + Sync + UoWFactoryTrait + Clone + 'static,
{
    async fn call(&self, mut request: Request) -> Result<MiddlewareResponse, EventErrorKind> {
        request
            .context
            .insert("uow_factory", Box::new(self.uow_factory.clone()));

        Ok((request, EventReturn::default()))
    }
}
