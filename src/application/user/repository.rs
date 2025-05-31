use async_trait::async_trait;

use crate::{application::common::exceptions::RepoKind, domain::entities::user::User};

use super::{
    dto::{create::Create, get_by_tg_id::GetByTgID},
    exceptions::{UserTgIdAlreadyExists, UserTgIdNotExist},
};

#[async_trait]
pub trait UserRepo {
    async fn create(&mut self, user: Create) -> Result<(), RepoKind<UserTgIdAlreadyExists>>;

    async fn get_by_tg_id(&mut self, user: GetByTgID) -> Result<User, RepoKind<UserTgIdNotExist>>;
}
