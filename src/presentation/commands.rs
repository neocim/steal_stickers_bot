use sqlx::Database;
use telers::{
    Bot, Filter as _, Router,
    client::Reqwest,
    enums::ContentType as ContentTypeEnum,
    errors::HandlerError,
    filters::{Command, ContentType, State as StateFilter},
    fsm::MemoryStorage,
    methods::SetMyCommands,
    types::{BotCommand, BotCommandScopeAllPrivateChats},
};

mod handlers;
mod states;

use crate::{
    application::{set::traits::SetRepo, user::traits::UserRepo},
    infrastructure::database::{
        repositories::{set::SetRepoImpl, user::UserRepoImpl},
        uow::UoWFactory,
    },
    presentation::commands::handlers::my_rank::my_rank_handler,
};
pub use handlers::deleted_sets_upd::deleted_sets_upd;
use handlers::{
    add_stickers::{
        add_stickers_handler, add_stickers_to_user_owned_sticker_set, get_stickers_to_add,
        get_stolen_sticker_set, process_non_sticker_handler,
    },
    bot_src::source_handler,
    cancel::cancel_handler,
    my_stickers::{my_stickers_handler, process_button},
    start::start_handler,
    steal_pack::{
        create_new_sticker_set, get_sticker_set_name, process_non_text_handler,
        steal_sticker_set_handler,
    },
};
use states::{add_stickers::AddStickerState, steal_sticker_set::StealStickerSetState};

pub async fn set_commands(bot: &Bot) -> Result<(), HandlerError> {
    let help_cmd = BotCommand::new("help", "Show help message");
    let source_cmd = BotCommand::new("source", "Show the source of the bot");
    let src_cmd = BotCommand::new("src", "Show the source of the bot");
    let steal_pack_cmd = BotCommand::new("stealpack", "Steal sticker pack");
    let steal_sticker_cmd = BotCommand::new(
        "addstickers",
        "Add stickers to a sticker pack stolen by this bot",
    );
    let my_stickers_cmd = BotCommand::new("mystickers", "List of your stolen stickers");
    let myrank_cmd = BotCommand::new("myrank", "See your rank in sticker theft");
    let cancel_cmd = BotCommand::new("cancel", "Cancel last command");

    let private_chats = [
        help_cmd,
        source_cmd,
        src_cmd,
        steal_pack_cmd,
        steal_sticker_cmd,
        cancel_cmd,
        myrank_cmd,
        my_stickers_cmd,
    ];
    bot.send(SetMyCommands::new(private_chats).scope(BotCommandScopeAllPrivateChats {}))
        .await?;

    Ok(())
}

pub fn init_commands<DB>(router: &mut Router<Reqwest>)
where
    DB: Database,
    for<'a> UserRepoImpl<&'a mut DB::Connection>: UserRepo,
    for<'a> SetRepoImpl<&'a mut DB::Connection>: SetRepo,
{
    process_non_command(
        router,
        &[
            "source",
            "src",
            "stealpack",
            "addstickers",
            "help",
            "cancel",
            "mystickers",
            "myrank",
        ],
    );
    start_command(router, &["start", "help"]);
    source_command(router, &["src", "source"]);
    cancel_command(router, "cancel");
    add_stickers_command::<DB>(router, "addstickers", "done");
    steal_sticker_set_command::<DB>(router, "stealpack");
    my_rank_command::<DB>(router, "myrank");
    my_stickers_command::<DB>(router, "mystickers");
    process_non_sticker(router);
    process_non_text(router);
}

fn my_rank_command<DB>(router: &mut Router<Reqwest>, command: &'static str)
where
    DB: Database,
    for<'a> UserRepoImpl<&'a mut DB::Connection>: UserRepo,
    for<'a> SetRepoImpl<&'a mut DB::Connection>: SetRepo,
{
    router
        .message
        .register(my_rank_handler::<MemoryStorage, UoWFactory<DB>>)
        .filter(Command::one(command));
}

/// If the user simply writes to the bot without calling any commands, the bot will call specified function
fn process_non_command(router: &mut Router<Reqwest>, ignore_commands: &'static [&str]) {
    router
        .message
        .register(start_handler::<MemoryStorage>)
        .filter(StateFilter::none())
        .filter(Command::many(ignore_commands.iter().map(ToOwned::to_owned)).invert());
}

/// Executes Telegram commands `/start` and `/help`
fn start_command(router: &mut Router<Reqwest>, commands: &'static [&str]) {
    router
        .message
        .register(start_handler::<MemoryStorage>)
        .filter(Command::many(commands.iter().map(ToOwned::to_owned)));
}

/// Executes Telegram commands `/src` and `/source`
fn source_command(router: &mut Router<Reqwest>, commands: &'static [&str]) {
    router
        .message
        .register(source_handler::<MemoryStorage>)
        .filter(Command::many(commands.iter().map(ToOwned::to_owned)));
}

/// Executes Telegram command `/cancel`
fn cancel_command(router: &mut Router<Reqwest>, command: &'static str) {
    router
        .message
        .register(cancel_handler::<MemoryStorage>)
        .filter(Command::one(command));
}

/// Executes Telegram command `/add_stickers`
fn add_stickers_command<DB>(
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
fn steal_sticker_set_command<DB>(router: &mut Router<Reqwest>, command: &'static str)
where
    DB: Database,
    for<'a> UserRepoImpl<&'a mut DB::Connection>: UserRepo,
    for<'a> SetRepoImpl<&'a mut DB::Connection>: SetRepo,
{
    router
        .message
        .register(steal_sticker_set_handler::<MemoryStorage>)
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
fn my_stickers_command<DB>(router: &mut Router<Reqwest>, command: &'static str)
where
    DB: Database,
    for<'a> UserRepoImpl<&'a mut DB::Connection>: UserRepo,
    for<'a> SetRepoImpl<&'a mut DB::Connection>: SetRepo,
{
    router
        .message
        .register(my_stickers_handler::<MemoryStorage, UoWFactory<DB>>)
        .filter(Command::one(command))
        .filter(ContentType::one(ContentTypeEnum::Text));

    router
        .callback_query
        .register(process_button::<MemoryStorage, UoWFactory<DB>>);
}

fn process_non_sticker(router: &mut Router<Reqwest>) {
    router
        .message
        .register(process_non_sticker_handler)
        .filter(ContentType::one(ContentTypeEnum::Sticker).invert())
        .filter(
            StateFilter::one(StealStickerSetState::StealStickerSetName).or(StateFilter::many([
                AddStickerState::GetStolenStickerSet,
                AddStickerState::GetStickersToAdd,
            ])),
        );
}

fn process_non_text(router: &mut Router<Reqwest>) {
    router
        .message
        .register(process_non_text_handler)
        .filter(ContentType::one(ContentTypeEnum::Text).invert())
        .filter(StateFilter::one(StealStickerSetState::CreateNewStickerSet));
}
