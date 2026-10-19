use std::time::Duration;

use grammers_client::{
    Client,
    InvocationError::{self},
};
use telers::{
    Bot, Extension,
    enums::ParseMode,
    errors::{HandlerError, TelegramErrorKind, session::ErrorKind},
    event::{EventReturn, telegram::HandlerResult},
    fsm::{Context, Storage},
    methods::{DeleteMessage, GetMe, GetStickerSet, SendMessage, SendSticker},
    types::{FileId, InputFile, Message, MessageSticker, MessageText, ReplyParameters, Sticker},
};
use tracing::error;

use crate::{
    application::{
        common::traits::uow::{UoW, UoWFactory as UoWFactoryTrait},
        set::{dto::count_by_tg_id::CountByTgID, repository::SetRepo as _},
    },
    core::helpers::{
        common::set_created_by,
        constants::{MAX_STICKER_SET_LENGTH, TELEGRAM_STICKER_SET_URL},
        texts::{
            added_stickers_message, adding_stickers, all_stickers_removed_done,
            all_stickers_removed_undo, ask_for_stickers_to_add, ask_stolen_sticker_or_link,
            link_or_sticker, no_stickers_sent_done, not_pack_owner, nothing_to_remove,
            pack_current_length, pack_deleted, pack_full, pack_not_stolen_by_bot,
            sticker_limit_reached, sticker_processed, sticker_removed_from_list,
            sticker_without_emoji, sticker_without_pack, user_doesnt_has_stickers_yet,
        },
    },
    presentation::{
        commands::{
            common::{
                AddStickersError, InvalidSetLinkError, add_stickers, parse_sticker_set_link,
                send_default_error_msg, send_detailed_error_msg, send_invalid_set_msg,
            },
            states::add_stickers::AddStickerState,
        },
        telegram_application::{errors::Error as TGError, get_sticker_set_user_id},
    },
};

pub async fn add_stickers_handler<S: Storage, UoWFactory>(
    bot: Bot,
    message: MessageText,
    fsm: Context<S>,
    Extension(uow_factory): Extension<UoWFactory>,
) -> HandlerResult
where
    UoWFactory: UoWFactoryTrait,
{
    fsm.finish().await.map_err(Into::into)?;

    let user_id = match message.from {
        Some(user) => user.id,
        None => return Ok(EventReturn::Finish),
    };

    let mut uow = uow_factory.create_uow();

    let set_count = uow
        .set_repo()
        .await
        .map_err(HandlerError::new)?
        .count_by_tg_id(CountByTgID::new(user_id, Some(false)))
        .await
        .map_err(HandlerError::new)?;

    if set_count == 0 {
        bot.send(SendMessage::new(
            message.chat.id(),
            user_doesnt_has_stickers_yet(),
        ))
        .await?;

        return Ok(EventReturn::Finish);
    }

    fsm.set_state(AddStickerState::GetStolenStickerSet)
        .await
        .map_err(Into::into)?;

    bot.send(SendMessage::new(
        message.chat.id(),
        ask_stolen_sticker_or_link(),
    ))
    .await?;

    Ok(EventReturn::Finish)
}
pub async fn get_stolen_sticker_set<S, UoWFactory>(
    bot: Bot,
    message: Message,
    fsm: Context<S>,
    Extension(client): Extension<Client>,
) -> HandlerResult
where
    UoWFactory: UoWFactoryTrait,
    S: Storage,
{
    // Извлекаем shortname пака либо из стикера, либо из ссылки в тексте
    let sticker_set_name: String = if let Some(sticker) = message.sticker() {
        match sticker.set_name() {
            Some(name) => name.to_string(),
            None => {
                bot.send(SendMessage::new(
                    message.chat().id(),
                    sticker_without_pack(),
                ))
                .await?;

                return Ok(EventReturn::Finish);
            }
        }
    } else {
        let text = message.text().expect("Provided message is not a text");

        if text.find(TELEGRAM_STICKER_SET_URL).is_none() {
            bot.send(SendMessage::new(message.chat().id(), link_or_sticker()))
                .await?;

            return Ok(EventReturn::Finish);
        }

        match parse_sticker_set_link(&bot, text).await {
            Ok(name) => name.to_string(),
            Err(err) => match err.downcast_ref() {
                Some(InvalidSetLinkError) => {
                    return send_invalid_set_msg(&bot, message.chat().id(), message.message_id())
                        .await;
                }
                None => {
                    send_detailed_error_msg(
                        &bot,
                        &err.to_string(),
                        message.chat().id(),
                        message.message_id(),
                    )
                    .await?;

                    return Ok(EventReturn::Finish);
                }
            },
        }
    };

    // Проверяем, что пак существует и доступен
    let result = bot
        .send(GetStickerSet::new(sticker_set_name.as_str()))
        .await;

    if let Err(ref error) = result {
        if matches!(error, ErrorKind::Telegram(TelegramErrorKind::BadRequest { message }) if message.as_ref() == "Bad Request: STICKERSET_INVALID")
        {
            bot.send(SendMessage::new(
                message.chat().id(),
                sticker_without_pack(),
            ))
            .await?;

            return Ok(EventReturn::Finish);
        }

        error!(
            ?error,
            "Error occurred while getting sticker set name to steal: "
        );

        send_default_error_msg(&bot, message.chat().id(), message.message_id()).await?;

        return Ok(EventReturn::Finish);
    }

    let sticker_set = result.unwrap();

    let bot_username = bot
        .send(GetMe::new())
        .await?
        .username
        .expect("bot without username :/");

    if !set_created_by(sticker_set_name.as_str(), bot_username.as_ref()) {
        bot.send(SendMessage::new(
            message.chat().id(),
            pack_not_stolen_by_bot(),
        ))
        .await?;

        return Ok(EventReturn::Finish);
    }

    let sticker_set_user_id = match get_sticker_set_user_id(&sticker_set_name, &client).await {
        Ok(id) => id,
        Err(TGError::InvocationError(InvocationError::Rpc(err))) => {
            if err.name == "STICKERSET_INVALID" {
                bot.send(SendMessage::new(message.chat().id(), pack_deleted()))
                    .await?;
            }

            return Ok(EventReturn::Finish);
        }
        Err(err) => {
            error!(
                ?err,
                ?sticker_set_name,
                "Error occurred while getting sticker set user id: "
            );

            send_detailed_error_msg(
                &bot,
                &err.to_string(),
                message.chat().id(),
                message.message_id(),
            )
            .await?;

            return Ok(EventReturn::Finish);
        }
    };

    // only panic if messages uses in channels, but i'm using private filter
    let user_id = message.from().expect("user not specified").id;

    if user_id != sticker_set_user_id {
        bot.send(SendMessage::new(message.chat().id(), not_pack_owner()))
            .await?;

        return Ok(EventReturn::Finish);
    }

    let set_length = sticker_set.stickers.len();

    let message_delete = if MAX_STICKER_SET_LENGTH - set_length > 0 {
        bot.send(
            SendMessage::new(
                message.chat().id(),
                pack_current_length(set_length, MAX_STICKER_SET_LENGTH - set_length),
            )
            .parse_mode(ParseMode::HTML)
            .reply_parameters(
                ReplyParameters::new()
                    .chat_id(message.chat().id())
                    .message_id(message.message_id()),
            ),
        )
        .await?
    } else {
        bot.send(
            SendMessage::new(message.chat().id(), pack_full()).reply_parameters(
                ReplyParameters::new()
                    .chat_id(message.chat().id())
                    .message_id(message.message_id()),
            ),
        )
        .await?;

        return Ok(EventReturn::Finish);
    };

    fsm.set_value(
        "get_stolen_sticker_set",
        (sticker_set_name, sticker_set.title, set_length),
    )
    .await
    .map_err(Into::into)?;

    fsm.set_state(AddStickerState::GetStickersToAdd)
        .await
        .map_err(Into::into)?;

    bot.send(SendMessage::new(
        message.chat().id(),
        ask_for_stickers_to_add(),
    ))
    .await?;

    // delete unnecessary message after 15 sec
    tokio::time::sleep(Duration::from_secs(15)).await;
    bot.send(DeleteMessage::new(
        message_delete.chat().id(),
        message_delete.message_id(),
    ))
    .await?;

    Ok(EventReturn::Finish)
}

pub async fn get_stickers_to_add<S, UoWFactory>(
    bot: Bot,
    message: MessageSticker,
    fsm: Context<S>,
) -> HandlerResult
where
    UoWFactory: UoWFactoryTrait,
    S: Storage,
{
    let (_, _, sticker_set_length): (Box<str>, Box<str>, usize) = fsm
        .get_value("get_stolen_sticker_set")
        .await
        .map_err(Into::into)?
        // only panic if i'm forget call fsm.set_value() in function get_stolen_sticker_set()
        .expect("sticker set name and sticker set title for sticker set should be set");

    let sticker_to_add = *message.sticker;

    if sticker_to_add.emoji().is_none() {
        bot.send(
            SendMessage::new(message.chat.id(), sticker_without_emoji()).reply_parameters(
                ReplyParameters::new()
                    .chat_id(message.chat.id())
                    .message_id(message.message_id),
            ),
        )
        .await?;

        return Ok(EventReturn::Finish);
    }

    let stickers_vec: Vec<Sticker> = match fsm
        .get_value::<&str, Vec<Sticker>>("get_stickers_to_add")
        .await
        .map_err(Into::into)?
    {
        Some(mut stickers_vec) => {
            let stickers_vec_len = stickers_vec.len();

            if sticker_set_length + stickers_vec_len >= MAX_STICKER_SET_LENGTH {
                bot.send(
                    SendMessage::new(message.chat.id(), sticker_limit_reached())
                        .parse_mode(ParseMode::HTML),
                )
                .await?;

                return Ok(EventReturn::Finish);
            }

            stickers_vec.push(sticker_to_add);
            stickers_vec
        }
        None => vec![sticker_to_add],
    };

    fsm.set_value("get_stickers_to_add", stickers_vec)
        .await
        .map_err(Into::into)?;

    bot.send(
        SendMessage::new(message.chat.id(), sticker_processed()).reply_parameters(
            ReplyParameters::new()
                .chat_id(message.chat.id())
                .message_id(message.message_id),
        ),
    )
    .await?;

    Ok(EventReturn::Finish)
}

pub async fn undo_last_sticker<S: Storage>(
    bot: Bot,
    message: MessageText,
    fsm: Context<S>,
) -> HandlerResult {
    let mut stickers_vec: Vec<Sticker> = match fsm
        .get_value("get_stickers_to_add")
        .await
        .map_err(Into::into)?
    {
        Some(stickers_vec) => stickers_vec,
        None => {
            bot.send(SendMessage::new(message.chat.id(), nothing_to_remove()))
                .await?;

            return Ok(EventReturn::Finish);
        }
    };

    let sticker = match stickers_vec.pop() {
        Some(sticker) => sticker,
        None => {
            bot.send(SendMessage::new(message.chat.id(), nothing_to_remove()))
                .await?;

            return Ok(EventReturn::Finish);
        }
    };

    let user_removed_all_stickers = stickers_vec.len() == 0;

    fsm.set_value("get_stickers_to_add", stickers_vec)
        .await
        .map_err(Into::into)?;

    let sticker_message = bot
        .send(SendSticker::new(
            message.chat.id(),
            InputFile::Id(FileId::new(sticker.file_id())),
        ))
        .await?;

    bot.send(
        SendMessage::new(message.chat.id(), sticker_removed_from_list())
            .reply_parameters(ReplyParameters::new().message_id(sticker_message.message_id()))
            .chat_id(sticker_message.chat().id()),
    )
    .await?;

    if user_removed_all_stickers {
        bot.send(SendMessage::new(
            message.chat.id(),
            all_stickers_removed_undo(),
        ))
        .await?;
    }

    Ok(EventReturn::Finish)
}

/// ### Panics
/// - Panics if user is unknown (only if message sent in channel)
pub async fn add_stickers_to_user_owned_sticker_set<S: Storage>(
    bot: Bot,
    message: MessageText,
    fsm: Context<S>,
) -> HandlerResult {
    let (sticker_set_name, sticker_set_title, _): (Box<str>, Box<str>, usize) = fsm
        .get_value("get_stolen_sticker_set")
        .await
        .map_err(Into::into)?
        // only panic if i'm forget call fsm.set_value() in function get_stolen_sticker_set()
        .expect("Sticker set name for sticker set should be set");

    let stickers = match fsm
        .get_value::<_, Vec<Sticker>>("get_stickers_to_add")
        .await
        .map_err(Into::into)?
    {
        Some(stickers_vec) if stickers_vec.len() == 0 => {
            bot.send(SendMessage::new(
                message.chat.id(),
                all_stickers_removed_done(),
            ))
            .await?;

            return Ok(EventReturn::Finish);
        }
        Some(stickers_vec) => stickers_vec,
        None => {
            bot.send(SendMessage::new(message.chat.id(), no_stickers_sent_done()))
                .await?;

            return Ok(EventReturn::Finish);
        }
    };

    fsm.finish().await.map_err(Into::into)?;

    // only panic if messages uses in channels, but i'm using private filter in main function
    let user_id = message.from.expect("Error while parsing user").id;

    let message_delete = bot
        .send(
            SendMessage::new(
                message.chat.id(),
                adding_stickers(stickers.len(), &sticker_set_name),
            )
            .parse_mode(ParseMode::HTML),
        )
        .await?;

    if let Err(AddStickersError::Other(err)) =
        add_stickers(&bot, user_id, sticker_set_name.as_ref(), stickers).await
    {
        send_detailed_error_msg(&bot, &err, message.chat.id(), message.message_id).await?;
    }

    // delete unnecessary message
    bot.send(DeleteMessage::new(
        message_delete.chat().id(),
        message_delete.message_id(),
    ))
    .await?;

    bot.send(
        SendMessage::new(
            message.chat.id(),
            added_stickers_message(&sticker_set_name, &sticker_set_title),
        )
        .parse_mode(ParseMode::HTML),
    )
    .await?;

    Ok(EventReturn::Finish)
}
