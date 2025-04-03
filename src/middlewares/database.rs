use telers::{
    Request,
    errors::EventErrorKind,
    event::EventReturn,
    middlewares::{OuterMiddleware, outer::MiddlewareResponse},
};

use async_trait::async_trait;

use crate::application::common::traits::uow::UoWFactory as UoWFactoryTrait;

#[derive(Debug, Clone)]
pub struct DatabaseMiddleware<UoWF> {
    uow_factory: UoWF,
}

impl<UoWF> DatabaseMiddleware<UoWF> {
    pub const fn new(uow_factory: UoWF) -> Self {
        Self { uow_factory }
    }
}

#[async_trait]
impl<UoWF> OuterMiddleware for DatabaseMiddleware<UoWF>
where
    UoWF: Send + Sync + UoWFactoryTrait + Clone + 'static,
{
    async fn call(&mut self, mut request: Request) -> Result<MiddlewareResponse, EventErrorKind> {
        request
            .context
            .insert("uow_factory", Box::new(self.uow_factory.clone()));

        Ok((request, EventReturn::default()))
    }
}
