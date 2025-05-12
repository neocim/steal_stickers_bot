use sqlx::FromRow;

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct GetAll {
    /// If:
    /// `None` -> get all
    /// `Some(true)` -> get only deleted
    /// `Some(false)` -> get only NOT deleted
    get_deleted: Option<bool>,
}

impl GetAll {
    pub const fn new(get_deleted: Option<bool>) -> Self {
        Self { get_deleted }
    }

    pub const fn get_deleted(&self) -> Option<bool> {
        self.get_deleted
    }
}
