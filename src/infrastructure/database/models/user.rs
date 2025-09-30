use crate::domain::entities::user::User as UserEntitie;
use sqlx::{FromRow, types::time::OffsetDateTime};

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct User {
    pub tg_id: i64,
    pub created: OffsetDateTime,
}

impl From<User> for UserEntitie {
    fn from(value: User) -> Self {
        Self {
            tg_id: value.tg_id,
            created: value.created,
        }
    }
}
