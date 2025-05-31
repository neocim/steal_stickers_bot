use sqlx::FromRow;

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct GetByTgID {
    tg_id: i64,
    /// If: `None` -> get all
    /// Some(true) -> get only deleted
    /// Some(false) -> get only NOT deleted
    get_deleted: Option<bool>,
}

impl GetByTgID {
    pub const fn new(tg_id: i64, get_deleted: Option<bool>) -> Self {
        Self { tg_id, get_deleted }
    }

    pub const fn tg_id(&self) -> i64 {
        self.tg_id
    }

    pub const fn get_deleted(&self) -> Option<bool> {
        self.get_deleted
    }
}
