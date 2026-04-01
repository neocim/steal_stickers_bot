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

impl AsRef<str> for StealStickerSetState {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq<&str> for StealStickerSetState {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}
