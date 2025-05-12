use async_trait::async_trait;

use crate::{application::common::exceptions::RepoKind, domain::entities::set::Set};

use super::{
    dto::{
        create::Create, delete_by_short_name::DeleteByShortName, get_by_short_name::GetByShortName,
        get_by_tg_id::GetByTgID, set_deleted_col_by_short_name::SetDeletedColByShortName,
    },
    exceptions::{SetShortNameAlreadyExist, SetShortNameNotExist, SetTgIdNotExist},
};

#[async_trait]
pub trait SetRepo {
    async fn create<'a>(
        &'a mut self,
        set: Create<'a>,
    ) -> Result<(), RepoKind<SetShortNameAlreadyExist>>;

    async fn delete_by_short_name<'a>(
        &'a mut self,
        set: DeleteByShortName<'a>,
    ) -> Result<(), RepoKind<SetShortNameNotExist>>;

    async fn get_by_tg_id(&mut self, set: GetByTgID)
    -> Result<Vec<Set>, RepoKind<SetTgIdNotExist>>;

    async fn get_one_by_short_name<'a>(
        &'a mut self,
        set: GetByShortName<'a>,
    ) -> Result<Set, RepoKind<SetShortNameNotExist>>;

    async fn set_deleted_col_by_short_name<'a>(
        &'a mut self,
        set: SetDeletedColByShortName<'a>,
    ) -> Result<(), RepoKind<SetShortNameNotExist>>;
}
