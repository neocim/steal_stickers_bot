use sqlx::prelude::FromRow;

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct Create {
    tg_id: i64,
}

impl Create {
    pub const fn new(tg_id: i64) -> Self {
        Self { tg_id }
    }

    pub const fn tg_id(&self) -> i64 {
        self.tg_id
    }
}
