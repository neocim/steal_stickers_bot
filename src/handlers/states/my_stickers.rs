use std::borrow::Cow;

#[derive(Clone)]
pub enum MyStickersState {
    EditStickerSetsListMessage,
    StickerSetsListInlineKeyboardMarkup,
    PreviousCallbackQuery,
    PagesNumber,
}

impl MyStickersState {
    const fn as_str(&self) -> &'static str {
        match self {
            MyStickersState::EditStickerSetsListMessage => "edit_sticker_sets_list_message",
            MyStickersState::StickerSetsListInlineKeyboardMarkup => {
                "sticker_sets_list_inline_keyboard_markup"
            }
            MyStickersState::PreviousCallbackQuery => "previous_callback_query",
            MyStickersState::PagesNumber => "pages_number",
        }
    }
}

impl From<MyStickersState> for Cow<'static, str> {
    fn from(state: MyStickersState) -> Self {
        Cow::Borrowed(state.as_str())
    }
}

impl PartialEq<&str> for MyStickersState {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}
