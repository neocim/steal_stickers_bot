use crate::{
    application::{
        common::exceptions::{RepoError, RepoKind},
        set::dto::count_by_tg_id::CountByTgID,
    },
    domain::entities::set::Set,
};

use super::{
    dto::{
        create::Create, delete_by_short_name::DeleteByShortName, get_all::GetAll,
        get_by_short_name::GetByShortName, get_by_tg_id::GetByTgID,
        set_deleted_col_by_short_name::SetDeletedColByShortName,
    },
    exceptions::{SetShortNameAlreadyExist, SetShortNameNotExist, SetTgIdNotExist},
};

pub trait SetRepo {
    fn create<'a>(
        &'a mut self,
        set: Create<'a>,
    ) -> impl Future<Output = Result<(), RepoKind<SetShortNameAlreadyExist>>> + Send;

    fn get_by_tg_id(
        &mut self,
        set: GetByTgID,
    ) -> impl Future<Output = Result<Vec<Set>, RepoKind<SetTgIdNotExist>>> + Send;

    fn get_set_counts_for_all_users(
        &mut self,
        set: GetAll,
    ) -> impl Future<Output = Result<Vec<i64>, RepoError>> + Send;

    fn delete_by_short_name<'a>(
        &'a mut self,
        set: DeleteByShortName<'a>,
    ) -> impl Future<Output = Result<(), RepoKind<SetShortNameNotExist>>> + Send;

    fn get_one_by_short_name<'a>(
        &'a mut self,
        set: GetByShortName<'a>,
    ) -> impl Future<Output = Result<Set, RepoKind<SetShortNameNotExist>>> + Send;

    fn set_deleted_col_by_short_name<'a>(
        &'a mut self,
        set: SetDeletedColByShortName<'a>,
    ) -> impl Future<Output = Result<(), RepoKind<SetShortNameNotExist>>> + Send;

    fn get_all(&mut self, set: GetAll) -> impl Future<Output = Result<Vec<Set>, RepoError>> + Send;

    fn count_by_tg_id(
        &mut self,
        set: CountByTgID,
    ) -> impl Future<Output = Result<i64, RepoError>> + Send;
}
