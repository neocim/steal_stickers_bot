use std::borrow::Cow;

#[derive(Clone)]
pub enum StealStickerSetState {
    StealStickerSetName,
    CreateNewStickerSet,
}

impl StealStickerSetState {
    const fn as_str(&self) -> &'static str {
        match self {
            StealStickerSetState::StealStickerSetName => "steal_sticker_set_name",
            StealStickerSetState::CreateNewStickerSet => "create_new_sticker_set",
        }
    }
}

impl From<StealStickerSetState> for Cow<'static, str> {
    fn from(state: StealStickerSetState) -> Self {
        Cow::Borrowed(state.as_str())
    }
}

impl PartialEq<&str> for StealStickerSetState {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}
