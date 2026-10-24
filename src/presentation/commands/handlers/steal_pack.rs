use telers::{
    Bot, Extension,
    enums::ParseMode,
    errors::{HandlerError, TelegramErrorKind, session::ErrorKind},
    event::{EventReturn, telegram::HandlerResult},
    fsm::{Context, Storage},
    methods::{CreateNewStickerSet, DeleteMessage, GetMe, GetStickerSet, SendMessage},
    types::{FileId, InputFile, InputSticker, Message, MessageText, ReplyParameters},
};
use tracing::error;

use crate::{
    application::{
        common::traits::uow::UoWFactory as UoWFactoryTrait, interactors::create_set::create_set,
        set::dto::create::Create as CreateSet,
    },
    core::helpers::{
        common::{generate_sticker_set_name_and_link, sticker_format},
        constants::{
            CREATE_SET_IN_ONE_GO_LENGTH_LIMIT, MAX_SET_TITLE_LENGTH, MIN_SET_TITLE_LENGTH,
            TELEGRAM_STICKER_SET_URL,
        },
        texts::{
            ask_for_sticker_or_link, link_or_sticker, now_enter_set_name, set_title_too_long,
            set_title_too_short, stealing_pack_started, sticker_without_pack, stolen_set_message,
            you_can_steal_at_any_time,
        },
    },
    presentation::commands::{
        common::{
            AddStickersError, add_stickers, check_set_name, handle_parse_link_error,
            parse_sticker_set_link, send_detailed_error_msg,
        },
        states::steal_sticker_set::StealStickerSetState,
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
    let chat_id = message.chat().id();
    let message_id = message.message_id();

    let set_name = if let Some(sticker) = message.sticker() {
        let set_name = match sticker.set_name() {
            Some(set_name) => set_name,
            None => {
                bot.send(
                    SendMessage::new(chat_id, sticker_without_pack()).reply_parameters(
                        ReplyParameters::new()
                            .chat_id(chat_id)
                            .message_id(message_id),
                    ),
                )
                .await?;

                return Ok(EventReturn::Finish);
            }
        };

        if let Err(error) = check_set_name(&bot, set_name).await {
            handle_parse_link_error(error, &bot, chat_id, message_id).await?;
            return Ok(EventReturn::Finish);
        }
        set_name.to_owned()
    } else {
        let text = message
            .text()
            .expect("Message is not text. Check the filters");

        if !text.contains(TELEGRAM_STICKER_SET_URL) {
            bot.send(
                SendMessage::new(chat_id, link_or_sticker()).reply_parameters(
                    ReplyParameters::new()
                        .chat_id(chat_id)
                        .message_id(message_id),
                ),
            )
            .await?;

            return Ok(EventReturn::Finish);
        }

        match parse_sticker_set_link(&bot, text).await {
            Ok(set_name) => set_name,
            Err(error) => {
                handle_parse_link_error(error, &bot, chat_id, message_id).await?;
                return Ok(EventReturn::Finish);
            }
        }
    };

    fsm.set_value("steal_sticker_set_name", set_name.to_string())
        .await
        .map_err(Into::into)?;

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

pub async fn process_steal_pack_confirmation<S: Storage>(
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
