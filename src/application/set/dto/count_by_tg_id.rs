use sqlx::prelude::FromRow;

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct CountByTgID {
    tg_id: i64,
    /// If: `None` -> count all
    /// Some(true) -> count ONLY deleted
    /// Some(false) -> count ONLY NOT deleted
    count_deleted: Option<bool>,
}

impl CountByTgID {
    pub const fn new(tg_id: i64, count_deleted: Option<bool>) -> Self {
        Self {
            tg_id,
            count_deleted,
        }
    }

    pub const fn tg_id(&self) -> i64 {
        self.tg_id
    }

    pub const fn count_deleted(&self) -> Option<bool> {
        self.count_deleted
    }
}
