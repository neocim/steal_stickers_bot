use telers::{
    Bot, Extension,
    enums::ParseMode,
    errors::{HandlerError, TelegramErrorKind, session::ErrorKind},
    event::{EventReturn, telegram::HandlerResult},
    fsm::{Context as FSMContext, Storage},
    methods::{AnswerCallbackQuery, EditMessageText, SendMessage},
    types::{CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup, Message},
};

use crate::{
    application::{
        common::traits::uow::{UoW as UoWTrait, UoWFactory as UoWFactoryTrait},
        set::{
            dto::{count_by_tg_id::CountByTgID, get_all::GetAll},
            repository::SetRepo as _,
        },
    },
    core::helpers::{
        stats::{GlobalStats, GreaterThan, PersonalStats},
        texts::{global_stats_message, personal_stats_message},
    },
    presentation::commands::states::callback_data::CallbackDataPrefix,
};

pub async fn stats_handler<S, UoWFactory>(
    bot: Bot,
    message: Message,
    fsm: FSMContext<S>,
    Extension(uow_factory): Extension<UoWFactory>,
) -> HandlerResult
where
    S: Storage,
    UoWFactory: UoWFactoryTrait,
{
    fsm.finish().await.map_err(Into::into)?;

    let mut uow = uow_factory.create_uow();

    let user_id = match message.from_id() {
        Some(id) => id,
        None => return Ok(EventReturn::Finish),
    };
    let chat_id = message.chat().id();

    let (keyboard_markup, personal_stats) = personal_stats(user_id, &mut uow).await?;

    bot.send(
        SendMessage::new(chat_id, personal_stats_message(personal_stats))
            .parse_mode(ParseMode::HTML)
            .reply_markup(keyboard_markup),
    )
    .await?;

    Ok(EventReturn::Finish)
}

pub async fn process_buttons<UoWFactory>(
    bot: Bot,
    callback_query: CallbackQuery,
    Extension(uow_factory): Extension<UoWFactory>,
) -> HandlerResult
where
    UoWFactory: UoWFactoryTrait,
{
    let mut uow = uow_factory.create_uow();

    let (chat_id, message_id) = match (callback_query.chat_id(), callback_query.message_id()) {
        (Some(chat_id), Some(message_id)) => (chat_id, message_id),
        _ => return Ok(EventReturn::Finish),
    };
    let user_id = callback_query.from.id;

    // if user came from personal statistics message, change it to global statistics message to personal otherwise
    if callback_query.data.unwrap().ends_with("Global") {
        let (keyboard_markup, global_stats) = global_stats(&mut uow).await?;

        send_edit_message(
            &bot,
            global_stats_message(global_stats),
            chat_id,
            message_id,
            keyboard_markup,
        )
        .await?;
    } else {
        let (keyboard_markup, personal_stats) = personal_stats(user_id, &mut uow).await?;

        send_edit_message(
            &bot,
            personal_stats_message(personal_stats),
            chat_id,
            message_id,
            keyboard_markup,
        )
        .await?;
    }

    bot.send(AnswerCallbackQuery::new(callback_query.id))
        .await?;

    Ok(EventReturn::Finish)
}

/// Get inline keyboard markup, total user sets count and not deleted user sets count
async fn personal_stats<UoW>(
    user_id: i64,
    uow: &mut UoW,
) -> Result<(InlineKeyboardMarkup, PersonalStats), HandlerError>
where
    UoW: UoWTrait,
{
    let total_user_sets_count = uow
        .set_repo()
        .await
        .map_err(HandlerError::new)?
        .count_by_tg_id(CountByTgID::new(user_id, None))
        .await
        .map_err(HandlerError::new)?;

    let not_deleted_user_sets_count = uow
        .set_repo()
        .await
        .map_err(HandlerError::new)?
        .count_by_tg_id(CountByTgID::new(user_id, Some(false)))
        .await
        .map_err(HandlerError::new)?;

    let buttons = vec![vec![
        InlineKeyboardButton::new("Global Statistics »").callback_data(format!(
            "{prefix}Global",
            prefix = CallbackDataPrefix::Stats.as_str()
        )),
    ]];

    let inline_keyboard_markup = InlineKeyboardMarkup::new(buttons);

    Ok((
        inline_keyboard_markup,
        PersonalStats::new(total_user_sets_count, not_deleted_user_sets_count),
    ))
}

async fn global_stats<UoW>(
    uow: &mut UoW,
) -> Result<(InlineKeyboardMarkup, GlobalStats), HandlerError>
where
    UoW: UoWTrait,
{
    let all_set_counts: Vec<i64> = uow
        .set_repo()
        .await
        .map_err(HandlerError::new)?
        .get_set_counts_for_all_users(GetAll::new(None))
        .await
        .map_err(HandlerError::new)?;

    let mut gt_fourth = 0u32;
    let mut gt_third = 0u32;
    let mut gt_second = 0u32;
    let mut gt_first = 0u32;

    for user_sets_count in all_set_counts.iter() {
        match *user_sets_count {
            count if count >= GreaterThan::FourthLevel as i64 => {
                gt_fourth += 1;
                gt_third += 1;
                gt_second += 1;
                gt_first += 1;
            }
            count if count >= GreaterThan::ThirdLevel as i64 => {
                gt_third += 1;
                gt_second += 1;
                gt_first += 1;
            }
            count if count >= GreaterThan::SecondLevel as i64 => {
                gt_second += 1;
                gt_first += 1;
            }
            count if count >= GreaterThan::FirstLevel as i64 => gt_first += 1,
            // if we see a count that is less than `FistLevel`, then we skip all the
            // following counts, because we don't need to spend time for them.
            _ => break,
        }
    }

    let buttons = vec![vec![
        InlineKeyboardButton::new("« Personal Statistics").callback_data(format!(
            "{prefix}Personal",
            prefix = CallbackDataPrefix::Stats.as_str()
        )),
    ]];

    let inline_keyboard_markup = InlineKeyboardMarkup::new(buttons);

    Ok((
        inline_keyboard_markup,
        GlobalStats::new(
            all_set_counts.into_iter().sum(),
            gt_first,
            gt_second,
            gt_third,
            gt_fourth,
        ),
    ))
}

async fn send_edit_message(
    bot: &Bot,
    text: impl Into<String>,
    chat_id: i64,
    message_id: i64,
    keyboard_markup: InlineKeyboardMarkup,
) -> HandlerResult {
    let edit_message = EditMessageText::new(text)
        .chat_id(chat_id)
        .message_id(message_id)
        .parse_mode(ParseMode::HTML)
        .reply_markup(keyboard_markup);
    if let Err(error) = bot.send(edit_message.parse_mode(ParseMode::HTML)).await {
        match &error {
            ErrorKind::Telegram(TelegramErrorKind::BadRequest { message }) => {
                // we need to ignore this bad request error
                if !message.contains("message is not modified") {
                    return Err(error.into());
                }
            }
            _ => return Err(error.into()),
        }
    }

    Ok(EventReturn::Finish)
}
