use CallbackData::*;

pub enum CallbackData {
    MyStickers,
    Stats,
}

impl CallbackData {
    pub const fn as_str(&self) -> &str {
        match self {
            MyStickers => "MyStickers",
            Stats => "Stats",
        }
    }
}
