use grammers_client::{Client, InvocationError};
use telers::{
    Bot, Extension,
    enums::ParseMode,
    errors::{HandlerError, TelegramErrorKind, session::ErrorKind},
    event::{EventReturn, telegram::HandlerResult},
    fsm::{Context, Storage},
    methods::{CreateNewStickerSet, DeleteMessage, GetMe, GetStickerSet, SendMessage},
    types::{
        FileId, InputFile, InputSticker, KeyboardButton, Message, MessageText, ReplyKeyboardMarkup,
        ReplyParameters, Sticker,
    },
};
use tracing::error;

use crate::{
    application::{
        common::traits::uow::UoWFactory as UoWFactoryTrait, interactors::create_set::create_set,
        set::dto::create::Create as CreateSet,
    },
    core::helpers::{
        common::{generate_sticker_set_name_and_link, set_created_by, sticker_format},
        constants::{
            CREATE_SET_IN_ONE_GO_LENGTH_LIMIT, MAX_SET_TITLE_LENGTH, MAX_STICKER_SET_LENGTH,
            MIN_SET_TITLE_LENGTH, TELEGRAM_STICKER_SET_URL,
        },
        texts::{
            ask_for_sticker_or_link, ask_for_stickers_to_add, link_or_sticker, now_enter_set_name,
            pack_full, pack_owned_by_user_and_bot, pack_owned_by_user_not_bot, set_title_too_long,
            set_title_too_short, stealing_pack_started, sticker_without_pack, stolen_set_message,
            you_can_steal_at_any_time,
        },
    },
    presentation::{
        commands::{
            common::{
                AddStickersError, InvalidSetLinkError, add_stickers, check_set_name,
                parse_sticker_set_link, send_detailed_error_msg, send_invalid_set_msg,
            },
            states::{add_stickers::AddStickerState, steal_sticker_set::StealStickerSetState},
        },
        telegram_application::{errors::Error as TGError, get_sticker_set_user_id},
    },
};

pub async fn steal_sticker_set_handler<S: Storage>(
    bot: Bot,
    message: MessageText,
    fsm: Context<S>,
) -> HandlerResult {
    fsm.finish().await.map_err(Into::into)?;

    fsm.set_state(StealStickerSetState::StealStickerSetName)
        .await
        .map_err(Into::into)?;

    bot.send(SendMessage::new(
        message.chat.id(),
        ask_for_sticker_or_link(),
    ))
    .await?;

    Ok(EventReturn::Finish)
}

pub async fn get_sticker_set_name<S: Storage>(
    bot: Bot,
    message: Message,
    fsm: Context<S>,
) -> HandlerResult {
    if let Some(sticker) = message.sticker() {
        process_sticker_to_steal(
            &bot,
            message.chat().id(),
            message.message_id(),
            &fsm,
            sticker,
        )
        .await?;
    } else {
        process_link_to_steal(
            &bot,
            message.chat().id(),
            message.message_id(),
            &fsm,
            // filter for texts and stickers during registrations
            message.text().expect("Message not text"),
        )
        .await?;
    }

    fsm.set_state(StealStickerSetState::CreateNewStickerSet)
        .await
        .map_err(Into::into)?;

    bot.send(
        SendMessage::new(message.chat().id(), now_enter_set_name()).reply_parameters(
            ReplyParameters::new()
                .chat_id(message.chat().id())
                .message_id(message.message_id()),
        ),
    )
    .await?;

    Ok(EventReturn::Finish)
}

/// ### Panics
/// - Panics if user is unknown (only if message sent in channel)
pub async fn create_new_sticker_set<S, UoWFactory>(
    bot: Bot,
    message: MessageText,
    fsm: Context<S>,
    Extension(uow_factory): Extension<UoWFactory>,
) -> HandlerResult
where
    UoWFactory: UoWFactoryTrait,
    S: Storage,
{
    // if user enter wrong sticker set title, process it
    let new_set_title = if message.text.len() > 64 {
        bot.send(
            SendMessage::new(message.chat.id(), set_title_too_long(MAX_SET_TITLE_LENGTH))
                .parse_mode(ParseMode::HTML),
        )
        .await?;

        return Ok(EventReturn::Finish);
    } else if message.text.len() < 1 {
        bot.send(
            SendMessage::new(
                message.chat.id(),
                set_title_too_short(MIN_SET_TITLE_LENGTH, MAX_SET_TITLE_LENGTH),
            )
            .parse_mode(ParseMode::HTML),
        )
        .await?;

        return Ok(EventReturn::Finish);
    } else {
        message.text
    };

    // only panic if i'm forget call fsm.set_value() in function steal_sticker_set_name()
    let steal_sticker_set_name: String = fsm
        .get_value("steal_sticker_set_name")
        .await
        .map_err(Into::into)?
        .expect("Sticker set name for sticker set user want steal should be set");

    fsm.finish().await.map_err(Into::into)?;

    let steal_sticker_set = bot
        .send(GetStickerSet::new(steal_sticker_set_name.as_ref()))
        .await?;

    let steal_stickers_from_sticker_set = steal_sticker_set.stickers;

    // cant panic because bot cant be without username
    let bot_username = bot
        .send(GetMe::new())
        .await?
        .username
        .expect("bot without username :/");

    // only panic if bot using in channels, but i'm using private filter in launch function
    let user_id = message.from.expect("user without id").id;

    // prepare name for new sticker set and link to use it in message later
    let (mut new_set_name, mut new_set_link) = generate_sticker_set_name_and_link(&bot_username);

    let message_delete = bot
        .send(
            SendMessage::new(
                message.chat.id(),
                stealing_pack_started(&new_set_title, CREATE_SET_IN_ONE_GO_LENGTH_LIMIT),
            )
            .parse_mode(ParseMode::HTML),
        )
        .await?;

    let limit_sticker_set_length =
        if steal_stickers_from_sticker_set.len() > CREATE_SET_IN_ONE_GO_LENGTH_LIMIT {
            CREATE_SET_IN_ONE_GO_LENGTH_LIMIT
        } else {
            steal_stickers_from_sticker_set.len()
        };

    while let Err(err) = bot
        .send(CreateNewStickerSet::new(
            user_id,
            new_set_name.as_str(),
            new_set_title.as_ref(),
            steal_stickers_from_sticker_set
                .iter()
                .take(limit_sticker_set_length)
                .map(|sticker| {
                    InputSticker::new(
                        InputFile::Id(FileId::new(sticker.file_id())),
                        sticker_format(sticker),
                        sticker.emoji(),
                    )
                }),
        ))
        .await
    {
        match err {
            ErrorKind::Telegram(err) => {
                if matches!(&err, TelegramErrorKind::BadRequest { message } if message.as_ref()
                    == "Bad Request: SHORTNAME_OCCUPY_FAILED")
                {
                    (new_set_name, new_set_link) =
                        generate_sticker_set_name_and_link(&bot_username);
                } else {
                    error!(
                        ?err,
                        ?new_set_name,
                        "Bad request error occurred while creating new sticker set: "
                    );

                    return send_detailed_error_msg(
                        &bot,
                        &err.to_string(),
                        message.chat.id(),
                        message.message_id,
                    )
                    .await;
                }
            }
            err => {
                error!(
                    ?err,
                    ?new_set_name,
                    "Unexpected error occurred while creating new sticker set:"
                );

                return send_detailed_error_msg(
                    &bot,
                    &err.to_string(),
                    message.chat.id(),
                    message.message_id,
                )
                .await;
            }
        }
    }

    if steal_stickers_from_sticker_set[limit_sticker_set_length..].len() > 0 {
        if let Err(AddStickersError::Other(err)) = add_stickers(
            &bot,
            user_id,
            new_set_name.as_ref(),
            steal_stickers_from_sticker_set[limit_sticker_set_length..].into(),
        )
        .await
        {
            send_detailed_error_msg(&bot, &err, message.chat.id(), message.message_id).await?;
        }
    }

    let mut uow = uow_factory.create_uow();

    create_set(
        &mut uow,
        CreateSet::new(user_id, new_set_name.as_str(), new_set_title.as_ref()),
    )
    .await
    .map_err(HandlerError::new)?;

    // delete unnecessary message
    bot.send(DeleteMessage::new(
        message_delete.chat().id(),
        message_delete.message_id(),
    ))
    .await?;

    bot.send(
        SendMessage::new(
            message.chat.id(),
            stolen_set_message(&new_set_title, &new_set_name, &new_set_link),
        )
        .parse_mode(ParseMode::HTML),
    )
    .await?;

    Ok(EventReturn::Finish)
}

pub async fn process_sticker_or_link<S: Storage>(
    bot: Bot,
    message: Message,
    fsm: Context<S>,
    Extension(client): Extension<Client>,
) -> HandlerResult {
    let set_name: String = if let Some(sticker) = message.sticker() {
        match sticker.set_name() {
            Some(name) => name.to_string(),
            None => return Ok(EventReturn::Skip),
        }
    } else {
        let Some(text) = message.text() else {
            return Ok(EventReturn::Skip);
        };
        if !text.contains(TELEGRAM_STICKER_SET_URL) {
            return Ok(EventReturn::Skip);
        }
        match parse_sticker_set_link(&bot, text).await {
            Ok(name) => name.to_string(),
            Err(_) => return Ok(EventReturn::Skip),
        }
    };

    let bot_username = bot
        .send(GetMe::new())
        .await?
        .username
        .expect("bot without username :/");

    let created_by_bot = set_created_by(&set_name, bot_username.as_ref());

    let user_id = match message.from() {
        Some(u) => u.id,
        None => return Ok(EventReturn::Skip),
    };

    let pack_user_id = match get_sticker_set_user_id(&set_name, &client).await {
        Ok(id) => Some(id),
        Err(TGError::InvocationError(InvocationError::Rpc(err)))
            if err.name == "STICKERSET_INVALID" =>
        {
            None
        }
        Err(err) => {
            error!(
                ?err,
                ?set_name,
                "Error occurred while getting sticker set user id in process_sticker_or_link: "
            );
            return Ok(EventReturn::Skip);
        }
    };

    let owned_by_user = pack_user_id == Some(user_id);

    if created_by_bot && owned_by_user {
        fsm.set_value("pending_pack_name", &set_name)
            .await
            .map_err(Into::into)?;

        bot.send(
            SendMessage::new(message.chat().id(), pack_owned_by_user_and_bot())
                .reply_markup(reply_keyboard_markup())
                .reply_parameters(
                    ReplyParameters::new()
                        .chat_id(message.chat().id())
                        .message_id(message.message_id()),
                ),
        )
        .await?;

        fsm.set_state(AddStickerState::ConfirmAddStickers)
            .await
            .map_err(Into::into)?;
    } else if owned_by_user && !created_by_bot {
        fsm.set_value("steal_sticker_set_name", &set_name)
            .await
            .map_err(Into::into)?;

        bot.send(
            SendMessage::new(message.chat().id(), pack_owned_by_user_not_bot())
                .reply_markup(reply_keyboard_markup())
                .reply_parameters(
                    ReplyParameters::new()
                        .chat_id(message.chat().id())
                        .message_id(message.message_id()),
                ),
        )
        .await?;

        fsm.set_state(StealStickerSetState::DoYouWantToSteal)
            .await
            .map_err(Into::into)?;
    } else {
        return Ok(EventReturn::Skip);
    }

    Ok(EventReturn::Finish)
}

pub async fn process_add_stickers_confirmation<S: Storage>(
    bot: Bot,
    message: MessageText,
    fsm: Context<S>,
) -> HandlerResult {
    let text = &*message.text;

    if text == "Yes" {
        let pack_name: String = fsm
            .get_value("pending_pack_name")
            .await
            .map_err(Into::into)?
            .expect("pending_pack_name should be set before confirmation");

        let set = bot.send(GetStickerSet::new(pack_name.as_str())).await?;
        let set_length = set.stickers.len();

        if set_length >= MAX_STICKER_SET_LENGTH {
            bot.send(SendMessage::new(message.chat.id(), pack_full()))
                .await?;

            fsm.finish().await.map_err(Into::into)?;
            return Ok(EventReturn::Finish);
        }

        fsm.set_value(
            "get_stolen_sticker_set",
            (pack_name.into_boxed_str(), set.title, set_length),
        )
        .await
        .map_err(Into::into)?;

        fsm.set_state(AddStickerState::GetStickersToAdd)
            .await
            .map_err(Into::into)?;

        bot.send(SendMessage::new(
            message.chat.id(),
            ask_for_stickers_to_add(),
        ))
        .await?;
    } else {
        bot.send(SendMessage::new(
            message.chat.id(),
            crate::core::helpers::texts::add_stickers_cancelled(),
        ))
        .await?;

        fsm.finish().await.map_err(Into::into)?;
    }

    Ok(EventReturn::Finish)
}

fn reply_keyboard_markup() -> ReplyKeyboardMarkup {
    let buttons = [[KeyboardButton::new("Yes"), KeyboardButton::new("No")]];
    ReplyKeyboardMarkup::new(buttons).one_time_keyboard(true)
}

pub async fn process_user_agreement<S: Storage>(
    bot: Bot,
    message: MessageText,
    fsm: Context<S>,
) -> HandlerResult {
    let text = &*message.text;

    if text == "Yes" {
        bot.send(SendMessage::new(message.chat.id(), now_enter_set_name()))
            .await?;

        fsm.set_state(StealStickerSetState::CreateNewStickerSet)
            .await
            .map_err(Into::into)?;

        return Ok(EventReturn::Finish);
    }
    bot.send(SendMessage::new(
        message.chat.id(),
        you_can_steal_at_any_time(),
    ))
    .await?;

    fsm.finish().await.map_err(Into::into)?;

    return Ok(EventReturn::Finish);
}

pub async fn process_link_to_steal<S: Storage>(
    bot: &Bot,
    chat_id: i64,
    message_id: i64,
    fsm: &Context<S>,
    text: &str,
) -> HandlerResult {
    if let Err(error) = parse_sticker_set_link(&bot, text).await {
        handle_error(error, bot, chat_id, message_id).await?;
        return Ok(EventReturn::Finish);
    }

    let set_name = match parse_sticker_set_link(&bot, text).await {
        Ok(set_name) => set_name,
        Err(error) => {
            return handle_error(error, bot, chat_id, message_id).await;
        }
    };

    fsm.set_value("steal_sticker_set_name", set_name)
        .await
        .map_err(Into::into)?;

    Ok(EventReturn::Finish)
}

pub async fn process_sticker_to_steal<S: Storage>(
    bot: &Bot,
    chat_id: i64,
    message_id: i64,
    fsm: &Context<S>,
    sticker: &Sticker,
) -> HandlerResult {
    let set_name = match sticker.set_name() {
        Some(set_name) => set_name,
        None => {
            bot.send(SendMessage::new(chat_id, sticker_without_pack()))
                .await?;

            return Ok(EventReturn::Finish);
        }
    };

    if let Err(error) = check_set_name(&bot, set_name).await {
        handle_error(error, bot, chat_id, message_id).await?;
        return Ok(EventReturn::Finish);
    }

    fsm.set_value("steal_sticker_set_name", set_name.to_string())
        .await
        .map_err(Into::into)?;

    Ok(EventReturn::Finish)
}

async fn handle_error(
    error: telers::errors::handler::Error,
    bot: &Bot,
    chat_id: i64,
    message_id: i64,
) -> HandlerResult {
    match error.downcast_ref() {
        Some(InvalidSetLinkError) => send_invalid_set_msg(&bot, chat_id, message_id).await,
        None => send_detailed_error_msg(&bot, &error.to_string(), chat_id, message_id).await,
    }
}
