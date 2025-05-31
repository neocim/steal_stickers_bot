use sqlx::prelude::FromRow;

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct GetByTgID {
    tg_id: i64,
}

impl GetByTgID {
    pub const fn new(tg_id: i64) -> Self {
        Self { tg_id }
    }

    pub const fn tg_id(&self) -> i64 {
        self.tg_id
    }
}
