use crate::domain::entities::set::Set as SetEntitie;
use sqlx::FromRow;

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct Set {
    pub tg_id: i64,
    pub short_name: String,
    pub deleted: bool,
    pub title: String,
}

#[derive(FromRow)]
pub struct SetsCount(pub i64);

impl From<Set> for SetEntitie {
    fn from(value: Set) -> Self {
        Self {
            tg_id: value.tg_id,
            short_name: value.short_name,
            deleted: value.deleted,
            title: value.title,
        }
    }
}
