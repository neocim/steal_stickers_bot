use sqlx::{Database, Pool};
use telers::{
    Bot, Filter as _, Router,
    enums::{ChatType::Private, MessageType as MessageTypeEnum},
    errors::HandlerError,
    event::telegram::Handler,
    filters::{ChatType, Command, MessageType, State as StateFilter, Text},
    fsm::{MemoryStorage, Strategy},
    methods::SetMyCommands,
    middlewares::{OuterMiddleware, outer::FSMContext},
    types::{BotCommand, BotCommandScopeAllPrivateChats},
};

mod common;
mod handlers;
mod states;

use crate::{
    application::{set::repository::SetRepo, user::repository::UserRepo},
    infrastructure::database::{
        repositories::{set::SetRepoImpl, user::UserRepoImpl},
        uow::UoWFactory,
    },
    presentation::{
        commands::{
            handlers::{
                add_stickers::undo_last_sticker,
                common::process_non_text_or_stickers_handler,
                get_owner::{get_owner_handler, get_owner_id},
                non_commands::process_non_commands_handler,
            },
            states::{callback_data::CallbackDataPrefix, get_owner::GetOwnerState},
        },
        middlewares::CreateUserMiddleware,
    },
};

pub use handlers::deleted_sets_upd::deleted_sets_upd;
use handlers::{
    add_stickers::{
        add_stickers_handler, add_stickers_to_user_owned_sticker_set, get_stickers_to_add,
        get_stolen_sticker_set,
    },
    bot_src::source_handler,
    cancel::cancel_handler,
    common::{process_non_sticker_handler, process_non_text_handler},
    my_stickers::{my_stickers_handler, process_buttons as process_my_stickers_buttons},
    start::start_handler,
    stats::{process_buttons as process_stats_buttons, stats_handler},
    steal_pack::{create_new_sticker_set, get_sticker_set_name, steal_sticker_set_handler},
};
use states::{add_stickers::AddStickerState, steal_sticker_set::StealStickerSetState};

pub struct PrivateRouterBuilder {
    router: Router,
}

const ALL_COMMANDS: &'static [&str] = &[
    "start",
    "help",
    "source",
    "src",
    "addstickers",
    "done",
    "undo",
    "stealpack",
    "mystickers",
    "cancel",
    "stats",
    "getowner",
];

impl PrivateRouterBuilder {
    pub fn build<DB>(router: Router, pool: Pool<DB>) -> Router
    where
        DB: Database,
        for<'a> UserRepoImpl<&'a mut DB::Connection>: UserRepo,
        for<'a> SetRepoImpl<&'a mut DB::Connection>: SetRepo,
        CreateUserMiddleware<UoWFactory<DB>>: OuterMiddleware,
    {
        let router = PrivateRouterBuilder { router }
            .start_command(&["start", "help"])
            .add_stickers_command::<DB>("addstickers", "done", "undo")
            .my_stickers_command::<DB>("mystickers")
            .stats_command::<DB>("stats")
            .get_owner_command("getowner")
            .source_command(&["src", "source"])
            .cancel_command("cancel")
            .steal_sticker_set_command::<DB>("stealpack")
            .process_non_text_and_stickers()
            .process_non_text()
            .process_non_sticker()
            .process_non_commands()
            .router;

        let router = router.on_all(|observer| {
            observer
                .filter(ChatType::one(Private))
                .register_outer_middleware(
                    FSMContext::new(MemoryStorage::new()).strategy(Strategy::UserInChat),
                )
                .register_outer_middleware(CreateUserMiddleware::new(UoWFactory::new(pool.clone())))
        });

        router
    }

    /// Executes Telegram commands `/start` and `/help`
    fn start_command(mut self, start_comands: &'static [&str]) -> Self {
        self.router = self.router.on_message(|observer| {
            observer.register(
                Handler::new(start_handler::<MemoryStorage>)
                    .filter(Command::many(start_comands.iter().map(ToOwned::to_owned))),
            )
        });
        self
    }

    /// Executes Telegram command `/stealpack`
    fn steal_sticker_set_command<DB>(mut self, command: &'static str) -> Self
    where
        DB: Database,
        for<'a> UserRepoImpl<&'a mut DB::Connection>: UserRepo,
        for<'a> SetRepoImpl<&'a mut DB::Connection>: SetRepo,
    {
        self.router = self.router.on_message(|observer| {
            observer.registers([
                Handler::new(steal_sticker_set_handler::<MemoryStorage>)
                    .filter(Command::one(command))
                    .filter(MessageType::one(MessageTypeEnum::Text)),
                Handler::new(get_sticker_set_name::<MemoryStorage>)
                    .filter(MessageType::many([
                        MessageTypeEnum::Sticker,
                        MessageTypeEnum::Text,
                    ]))
                    .filter(StateFilter::one(StealStickerSetState::StealStickerSetName)),
                Handler::new(create_new_sticker_set::<MemoryStorage, UoWFactory<DB>>)
                    .filter(MessageType::one(MessageTypeEnum::Text))
                    .filter(Command::many(ALL_COMMANDS.iter().map(ToOwned::to_owned)).invert())
                    .filter(StateFilter::one(StealStickerSetState::CreateNewStickerSet)),
            ])
        });
        self
    }

    /// Executes Telegram command `/add_stickers`
    fn add_stickers_command<DB>(
        mut self,
        command: &'static str,
        done_command: &'static str,
        undo_command: &'static str,
    ) -> Self
    where
        DB: Database,
        for<'a> UserRepoImpl<&'a mut DB::Connection>: UserRepo,
        for<'a> SetRepoImpl<&'a mut DB::Connection>: SetRepo,
    {
        self.router = self.router.on_message(|observer| {
            observer.registers([
                Handler::new(add_stickers_handler::<MemoryStorage, UoWFactory<DB>>)
                    .filter(Command::one(command))
                    .filter(MessageType::one(MessageTypeEnum::Text)),
                Handler::new(get_stolen_sticker_set::<MemoryStorage, UoWFactory<DB>>)
                    .filter(MessageType::one(MessageTypeEnum::Sticker))
                    .filter(StateFilter::one(AddStickerState::GetStolenStickerSet)),
                Handler::new(get_stickers_to_add::<MemoryStorage, UoWFactory<DB>>)
                    .filter(MessageType::one(MessageTypeEnum::Sticker))
                    .filter(StateFilter::one(AddStickerState::GetStickersToAdd)),
                Handler::new(add_stickers_to_user_owned_sticker_set::<MemoryStorage>)
                    .filter(Command::one(done_command))
                    .filter(StateFilter::one(AddStickerState::GetStickersToAdd)),
                Handler::new(undo_last_sticker::<MemoryStorage>)
                    .filter(Command::one(undo_command))
                    .filter(StateFilter::one(AddStickerState::GetStickersToAdd)),
            ])
        });
        self
    }

    /// Show all user stolen sticker sets
    fn my_stickers_command<DB>(mut self, command: &'static str) -> Self
    where
        DB: Database,
        for<'a> UserRepoImpl<&'a mut DB::Connection>: UserRepo,
        for<'a> SetRepoImpl<&'a mut DB::Connection>: SetRepo,
    {
        self.router = self
            .router
            .on_message(|observer| {
                observer.register(
                    Handler::new(my_stickers_handler::<MemoryStorage, UoWFactory<DB>>)
                        .filter(Command::one(command))
                        .filter(MessageType::one(MessageTypeEnum::Text)),
                )
            })
            .on_callback_query(|observer| {
                observer.register(
                    Handler::new(process_my_stickers_buttons::<UoWFactory<DB>>).filter(
                        Text::starts_with_single(CallbackDataPrefix::MyStickers.as_str()),
                    ),
                )
            });
        self
    }

    fn stats_command<DB>(mut self, command: &'static str) -> Self
    where
        DB: Database,
        for<'a> UserRepoImpl<&'a mut DB::Connection>: UserRepo,
        for<'a> SetRepoImpl<&'a mut DB::Connection>: SetRepo,
    {
        self.router = self.router.on_message(|observer| {
            observer.registers([
                Handler::new(stats_handler::<MemoryStorage, UoWFactory<DB>>)
                    .filter(Command::one(command)),
                Handler::new(process_stats_buttons::<UoWFactory<DB>>)
                    .filter(Text::starts_with_single(CallbackDataPrefix::Stats.as_str())),
            ])
        });
        self
    }

    fn get_owner_command(mut self, command: &'static str) -> Self {
        self.router = self.router.on_message(|observer| {
            observer.registers([
                Handler::new(get_owner_handler::<MemoryStorage>)
                    .filter(Command::one(command))
                    .filter(MessageType::one(MessageTypeEnum::Text)),
                Handler::new(get_owner_id)
                    .filter(MessageType::one(MessageTypeEnum::Sticker))
                    .filter(StateFilter::one(GetOwnerState::GetStickers)),
            ])
        });
        self
    }

    /// Executes Telegram commands `/src` and `/source`
    fn source_command(mut self, commands: &'static [&str]) -> Self {
        self.router = self.router.on_message(|observer| {
            observer.register(
                Handler::new(source_handler::<MemoryStorage>)
                    .filter(Command::many(commands.iter().map(ToOwned::to_owned))),
            )
        });
        self
    }

    /// Executes Telegram command `/cancel`
    fn cancel_command(mut self, command: &'static str) -> Self {
        self.router = self.router.on_message(|observer| {
            observer.register(
                Handler::new(cancel_handler::<MemoryStorage>).filter(Command::one(command)),
            )
        });
        self
    }

    fn process_non_commands(mut self) -> Self {
        self.router = self.router.on_message(|observer| {
            observer.register(
                Handler::new(process_non_commands_handler::<MemoryStorage>)
                    .filter(StateFilter::none())
                    .filter(Command::many(ALL_COMMANDS.iter().map(ToOwned::to_owned)).invert()),
            )
        });
        self
    }

    fn process_non_text_and_stickers(mut self) -> Self {
        self.router = self.router.on_message(|observer| {
            observer.register(
                Handler::new(process_non_text_or_stickers_handler)
                    .filter(
                        MessageType::many([MessageTypeEnum::Text, MessageTypeEnum::Sticker])
                            .invert(),
                    )
                    .filter(StateFilter::one(StealStickerSetState::StealStickerSetName)),
            )
        });
        self
    }

    fn process_non_sticker(mut self) -> Self {
        self.router = self.router.on_message(|observer| {
            observer.register(
                Handler::new(process_non_sticker_handler)
                    .filter(MessageType::one(MessageTypeEnum::Sticker).invert())
                    .filter(
                        StateFilter::one(StealStickerSetState::StealStickerSetName)
                            .or(StateFilter::many([
                                AddStickerState::GetStolenStickerSet,
                                AddStickerState::GetStickersToAdd,
                            ]))
                            .or(StateFilter::one(GetOwnerState::GetStickers)),
                    ),
            )
        });
        self
    }

    fn process_non_text(mut self) -> Self {
        self.router = self.router.on_message(|observer| {
            observer.register(
                Handler::new(process_non_text_handler)
                    .filter(MessageType::one(MessageTypeEnum::Text).invert())
                    .filter(StateFilter::one(StealStickerSetState::CreateNewStickerSet)),
            )
        });
        self
    }
}

pub async fn set_commands(bot: Bot) -> Result<(), HandlerError> {
    let help_cmd = BotCommand::new("help", "Show help message");
    let src_cmd = BotCommand::new("src", "Show the source code of the bot");
    let steal_pack_cmd = BotCommand::new("stealpack", "Steal sticker pack");
    let add_stickers_cmd = BotCommand::new(
        "addstickers",
        "Add stickers to a sticker pack stolen by this bot",
    );
    let my_stickers_cmd = BotCommand::new("mystickers", "List of your stolen stickers");
    let stats_cmd = BotCommand::new("stats", "See the bot statistics");
    let cancel_cmd = BotCommand::new("cancel", "Cancel last command");
    let get_owner_cmd = BotCommand::new("getowner", "Get the ID of the owner of stickers");

    let private_chats = [
        steal_pack_cmd,
        add_stickers_cmd,
        my_stickers_cmd,
        stats_cmd,
        help_cmd,
        get_owner_cmd,
        cancel_cmd,
        src_cmd,
    ];
    bot.send(SetMyCommands::new(private_chats).scope(BotCommandScopeAllPrivateChats {}))
        .await?;

    Ok(())
}
