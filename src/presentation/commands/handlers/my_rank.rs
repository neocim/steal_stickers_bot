use telers::{
    Bot, Extension,
    event::{EventReturn, telegram::HandlerResult},
    fsm::{Context, Storage},
    methods::SendMessage,
    types::Message,
};

use crate::application::common::traits::uow::UoWFactory as UoWFactoryTrait;

pub async fn my_rank_handler<S: Storage, UoWFactory>(
    bot: Bot,
    message: Message,
    fsm: Context<S>,
    Extension(uow_factory): Extension<UoWFactory>,
) -> HandlerResult
where
    UoWFactory: UoWFactoryTrait,
{
    fsm.finish().await.map_err(Into::into)?;

    Ok(EventReturn::Finish)
}
