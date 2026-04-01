#[derive(Clone)]
pub enum AddStickerState {
    GetStolenStickerSet,
    GetStickersToAdd,
}

impl AddStickerState {
    const fn as_str(&self) -> &'static str {
        match self {
            AddStickerState::GetStolenStickerSet => "get_stolen_sticker_set",
            AddStickerState::GetStickersToAdd => "get_stickers_to_add",
        }
    }
}

impl AsRef<str> for AddStickerState {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq<&str> for AddStickerState {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}
