use crate::{application::common::exceptions::RepoKind, domain::entities::user::User};

use super::{
    dto::{create::Create, get_by_tg_id::GetByTgID},
    exceptions::{UserTgIdAlreadyExists, UserTgIdNotExist},
};

pub trait UserRepo {
    fn create(
        &mut self,
        user: Create,
    ) -> impl Future<Output = Result<(), RepoKind<UserTgIdAlreadyExists>>> + Send;

    fn get_by_tg_id(
        &mut self,
        user: GetByTgID,
    ) -> impl Future<Output = Result<User, RepoKind<UserTgIdNotExist>>> + Send;
}
