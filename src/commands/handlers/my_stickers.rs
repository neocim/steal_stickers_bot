use std::borrow::Cow;

use telers::{
    Bot,
    enums::ParseMode,
    errors::HandlerError,
    event::{EventReturn, telegram::HandlerResult},
    fsm::{Context as FSMContext, Storage},
    methods::{EditMessageText, SendMessage},
    types::{
        CallbackQuery, ChatIdKind, InlineKeyboardButton, InlineKeyboardMarkup, Message,
        MessageText, ReplyMarkup,
    },
};
use tracing::error;

use crate::{
    application::{
        common::{
            exceptions::BeginError,
            traits::uow::{UoW as _, UoWFactory as UoWFactoryTrait},
        },
        set::{dto::get_by_tg_id::GetByTgID as GetSetByTgID, traits::SetRepo as _},
    },
    core::stickers_helpers::constants::STICKER_SETS_NUMBER_PER_PAGE,
    core::texts::current_page_message,
    domain::entities::set::Set,
};

use super::states::my_stickers::MyStickersState;

impl From<BeginError> for HandlerError {
    fn from(value: BeginError) -> Self {
        HandlerError::new(value)
    }
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("Error occurded while getting buttons: {message}")]
struct GetButtonsError {
    message: Cow<'static, str>,
}

impl GetButtonsError {
    fn new(message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

pub async fn my_stickers_handler<S, UoWFactory>(
    bot: Bot,
    message: MessageText,
    fsm: FSMContext<S>,
    uow_factory: UoWFactory,
) -> HandlerResult
where
    UoWFactory: UoWFactoryTrait,
    S: Storage,
{
    fsm.finish().await.map_err(Into::into)?;

    let mut uow = uow_factory.create_uow();

    // only panic if messages uses in channels, but i'm using private filter in main function
    let user_id = message.from.expect("user not specified").id;

    let sticker_sets = uow
        .set_repo()
        .await
        .map_err(HandlerError::new)?
        .get_by_tg_id(GetSetByTgID::new(user_id, Some(false)))
        .await
        .map_err(HandlerError::new)?;

    let mut buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();

    let page_count = match get_buttons(
        sticker_sets.as_ref(),
        STICKER_SETS_NUMBER_PER_PAGE,
        &mut buttons,
    ) {
        Ok(pages) => pages,
        Err(err) => {
            bot.send(SendMessage::new(message.chat.id(), err.message.to_string()))
                .await?;

            return Ok(EventReturn::Finish);
        }
    };

    let inline_keyboard_markup = InlineKeyboardMarkup::new(buttons);
    let inline_keyboard = ReplyMarkup::InlineKeyboard(inline_keyboard_markup.clone());

    let sticker_sets_list_message = bot
        .send(
            SendMessage::new(
                message.chat.id(),
                current_page_message(
                    1,
                    page_count,
                    STICKER_SETS_NUMBER_PER_PAGE,
                    sticker_sets.as_ref(),
                ),
            )
            .parse_mode(ParseMode::HTML)
            .reply_markup(inline_keyboard),
        )
        .await?;

    fsm.set_value("edit_sticker_sets_list_message", sticker_sets_list_message)
        .await
        .map_err(Into::into)?;

    fsm.set_value(
        "sticker_sets_list_inline_keyboard_markup",
        inline_keyboard_markup,
    )
    .await
    .map_err(Into::into)?;

    fsm.set_value("pages_number", page_count)
        .await
        .map_err(Into::into)?;

    fsm.set_state(MyStickersState::EditStickerSetsListMessage)
        .await
        .map_err(Into::into)?;

    Ok(EventReturn::Finish)
}

pub async fn process_button<S, UoWFactory>(
    bot: Bot,
    callback_query: CallbackQuery,
    fsm: FSMContext<S>,
    uow_factory: UoWFactory,
) -> HandlerResult
where
    UoWFactory: UoWFactoryTrait,
    S: Storage,
{
    let mut uow = uow_factory.create_uow();

    let message_data = match callback_query.data {
        Some(message_data) => message_data,
        None => {
            error!(
                "None value occurded while processed callback query from inline keyboard button!"
            );

            bot.send(SendMessage::new(
                callback_query.chat_id().expect("chat not found"),
                "Sorry, an error occurded. Try again :(",
            ))
            .await?;

            return Ok(EventReturn::Finish);
        }
    };

    // process if user click to one button several times in a row
    match fsm
        .get_value::<_, Box<str>>("previous_callback_query")
        .await
        .map_err(Into::into)?
    {
        Some(msg_data) => {
            if msg_data == message_data {
                // do nothing
                return Ok(EventReturn::Finish);
            } else {
                fsm.set_value("previous_callback_query", Some(message_data.as_ref()))
                    .await
                    .map_err(Into::into)?;
            }
        }
        None => {
            fsm.set_value("previous_callback_query", Some(message_data.as_ref()))
                .await
                .map_err(Into::into)?;
        }
    };

    let current_page = message_data
        .parse::<usize>()
        .expect("fail to convert `message_data` string into usize");

    let pages_number: u32 = fsm
        .get_value("pages_number")
        .await
        .map_err(Into::into)?
        .expect("Number of pages should be set");

    if pages_number == 1 {
        return Ok(EventReturn::Finish);
    }

    let user_id = callback_query.from.id;

    let sticker_sets = uow
        .set_repo()
        .await
        .map_err(HandlerError::new)?
        .get_by_tg_id(GetSetByTgID::new(user_id, Some(false)))
        .await
        .map_err(HandlerError::new)?;

    let sticker_sets_page = current_page_message(
        current_page,
        pages_number,
        STICKER_SETS_NUMBER_PER_PAGE,
        &sticker_sets,
    );

    let message_to_edit: Message = fsm
        .get_value("edit_sticker_sets_list_message")
        .await
        .map_err(Into::into)?
        .expect("Sticker sets list message should be set");

    let message_to_edit_reply_markup: InlineKeyboardMarkup = fsm
        .get_value("sticker_sets_list_inline_keyboard_markup")
        .await
        .map_err(Into::into)?
        .expect("Inline keyboard for sticker sets list should be set");

    let message_to_edit_chat_id = ChatIdKind::id(message_to_edit.chat().id());
    let message_to_edit_id = message_to_edit.id();

    let edit_message = EditMessageText::new(sticker_sets_page)
        .chat_id(message_to_edit_chat_id)
        .message_id(message_to_edit_id)
        .reply_markup(message_to_edit_reply_markup);

    bot.send(edit_message.parse_mode(ParseMode::HTML)).await?;

    Ok(EventReturn::Finish)
}

fn get_buttons(
    list: &[Set],
    sticker_sets_number_per_page: usize,
    buttons: &mut Vec<Vec<InlineKeyboardButton>>,
) -> Result<u32, GetButtonsError> {
    let mut page_count: u32 = 0;
    let mut current_row_index = 0;

    if list.len() > sticker_sets_number_per_page || !list.is_empty() {
        list.iter()
            .enumerate()
            .filter(|(index, _)| index % sticker_sets_number_per_page == 0)
            .for_each(|_| {
                // create a new row every 5 buttons
                if page_count % 5 == 0 {
                    page_count += 1;
                    current_row_index += 1;

                    buttons.push(vec![
                        InlineKeyboardButton::new(format!("page {page_count}",))
                            .callback_data(format!("{page_count}",)),
                    ])
                // else push button into current row
                } else {
                    page_count += 1;

                    buttons[current_row_index - 1].push(
                        InlineKeyboardButton::new(format!("Page {page_count}",))
                            .callback_data(format!("{page_count}",)),
                    );
                }
            })
    // otherwise user does not have sticker sets stolen by this bot
    } else {
        return Err(GetButtonsError::new(
            "You don't have a single stolen sticker pack. \
            Steal any sticker pack using the /stealpack command and it will appear in this list.",
        ));
    };

    Ok(page_count)
}
