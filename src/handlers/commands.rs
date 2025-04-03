use crate::{
    application::{set::traits::SetRepo, user::traits::UserRepo},
    handlers::states::{AddStickerState, MyStickersState, StealStickerSetState},
    infrastructure::database::{
        repositories::{set::SetRepoImpl, user::UserRepoImpl},
        uow::UoWFactory,
    },
};
use sqlx::Database;
use telers::{
    Filter as _, Router,
    client::Reqwest,
    enums::{ChatType as ChatTypeEnum, ContentType as ContentTypeEnum},
    filters::{ChatType, Command, ContentType, State as StateFilter},
    fsm::MemoryStorage,
};

use super::{
    add_stickers::{
        add_stickers_handler, add_stickers_to_user_owned_sticker_set, get_stickers_to_add,
        get_stolen_sticker_set, process_non_sticker as process_non_sticker_handler,
    },
    bot_src::source_handler,
    cancel::cancel_handler,
    my_stickers::{my_stickers_handler, process_button},
    start::start_handler,
    steal_pack::{create_new_sticker_set, get_sticker_set_name, steal_sticker_set_handler},
};

/// If the user simply writes to the bot without calling any commands, the bot will call specified function
pub async fn process_non_command(router: &mut Router<Reqwest>, ignore_commands: &'static [&str]) {
    router
        .message
        .filter(ChatType::one(ChatTypeEnum::Private))
        .register(start_handler::<MemoryStorage>)
        .filter(StateFilter::none())
        .filter(Command::many(ignore_commands.iter().map(ToOwned::to_owned)).invert());
}

/// Executes Telegram commands `/start` and `/help`
pub async fn start_command(router: &mut Router<Reqwest>, commands: &'static [&str]) {
    router
        .message
        .register(start_handler::<MemoryStorage>)
        .filter(ChatType::one(ChatTypeEnum::Private))
        .filter(Command::many(commands.iter().map(ToOwned::to_owned)));
}

/// Executes Telegram commands `/src` and `/source`
pub async fn source_command(router: &mut Router<Reqwest>, commands: &'static [&str]) {
    router
        .message
        .register(source_handler::<MemoryStorage>)
        .filter(ChatType::one(ChatTypeEnum::Private))
        .filter(Command::many(commands.iter().map(ToOwned::to_owned)));
}

/// Executes Telegram command `/cancel`
pub async fn cancel_command(router: &mut Router<Reqwest>, commands: &'static [&str]) {
    router
        .message
        .register(cancel_handler::<MemoryStorage>)
        .filter(ChatType::one(ChatTypeEnum::Private))
        .filter(Command::many(commands.iter().map(ToOwned::to_owned)));
}

/// Executes Telegram command `/add_stickers`
pub async fn add_stickers_command<DB>(
    router: &mut Router<Reqwest>,
    command: &'static str,
    done_command: &'static str,
) where
    DB: Database,
    for<'a> UserRepoImpl<&'a mut DB::Connection>: UserRepo,
    for<'a> SetRepoImpl<&'a mut DB::Connection>: SetRepo,
{
    router
        .message
        .register(add_stickers_handler::<MemoryStorage>)
        .filter(ChatType::one(ChatTypeEnum::Private))
        .filter(Command::one(command))
        .filter(ContentType::one(ContentTypeEnum::Text));

    router
        .message
        .register(get_stolen_sticker_set::<MemoryStorage, UoWFactory<DB>>)
        .filter(ContentType::one(ContentTypeEnum::Sticker))
        .filter(StateFilter::one(AddStickerState::GetStolenStickerSet));

    router
        .message
        .register(get_stickers_to_add::<MemoryStorage, UoWFactory<DB>>)
        .filter(ContentType::one(ContentTypeEnum::Sticker))
        .filter(StateFilter::one(AddStickerState::GetStickersToAdd));

    router
        .message
        .register(add_stickers_to_user_owned_sticker_set::<MemoryStorage>)
        .filter(Command::one(done_command))
        .filter(ContentType::one(ContentTypeEnum::Text))
        .filter(StateFilter::one(AddStickerState::GetStickersToAdd));
}

/// Executes Telegram command `/steal_pack`
pub async fn steal_sticker_set_command<DB>(router: &mut Router<Reqwest>, command: &'static str)
where
    DB: Database,
    for<'a> UserRepoImpl<&'a mut DB::Connection>: UserRepo,
    for<'a> SetRepoImpl<&'a mut DB::Connection>: SetRepo,
{
    router
        .message
        .register(steal_sticker_set_handler::<MemoryStorage>)
        .filter(ChatType::one(ChatTypeEnum::Private))
        .filter(Command::one(command))
        .filter(ContentType::one(ContentTypeEnum::Text));

    router
        .message
        .register(get_sticker_set_name::<MemoryStorage>)
        .filter(ContentType::one(ContentTypeEnum::Sticker))
        .filter(StateFilter::one(StealStickerSetState::StealStickerSetName));

    router
        .message
        .register(create_new_sticker_set::<MemoryStorage, UoWFactory<DB>>)
        .filter(ContentType::one(ContentTypeEnum::Text))
        .filter(StateFilter::one(StealStickerSetState::CreateNewStickerSet));
}

/// Show all user stolen sticker sets
pub async fn my_stickers<DB>(router: &mut Router<Reqwest>, command: &'static str)
where
    DB: Database,
    for<'a> UserRepoImpl<&'a mut DB::Connection>: UserRepo,
    for<'a> SetRepoImpl<&'a mut DB::Connection>: SetRepo,
{
    router
        .message
        .filter(ChatType::one(ChatTypeEnum::Private))
        .register(my_stickers_handler::<MemoryStorage, UoWFactory<DB>>)
        .filter(Command::one(command))
        .filter(ContentType::one(ContentTypeEnum::Text));

    router
        .callback_query
        .register(process_button::<MemoryStorage, UoWFactory<DB>>)
        .filter(StateFilter::one(
            MyStickersState::EditStickerSetsListMessage,
        ));
}

/// If user enter wrong content type, but the request type is <content_type>, this handler will process it
pub async fn process_non_sticker(router: &mut Router<Reqwest>, content_type: ContentTypeEnum) {
    router
        .message
        .filter(ChatType::one(ChatTypeEnum::Private))
        .register(process_non_sticker_handler)
        .filter(ContentType::one(content_type).invert())
        .filter(
            StateFilter::one(StealStickerSetState::StealStickerSetName).or(StateFilter::many([
                AddStickerState::GetStolenStickerSet,
                AddStickerState::GetStickersToAdd,
            ])),
        );
}
