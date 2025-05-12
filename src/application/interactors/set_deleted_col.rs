use crate::application::{
    common::{
        exceptions::{RepoKind, TransactionKind},
        traits::uow::UoW as UoWTrait,
    },
    set::{dto::set_deleted_col_by_short_name::SetDeletedColByShortName, traits::SetRepo as _},
};

pub async fn set_deleted_col<UoW>(
    uow: &mut UoW,
    set: SetDeletedColByShortName<'_>,
) -> Result<(), TransactionKind>
where
    UoW: UoWTrait,
{
    let result = uow
        .set_repo()
        .await
        .map_err(TransactionKind::begin_err)?
        .set_deleted_col_by_short_name(set)
        .await;

    match result {
        Ok(_) => (),
        Err(RepoKind::Unexpected(_)) => {
            uow.rollback()
                .await
                .map_err(TransactionKind::rollback_err)?;
        }
        Err(RepoKind::Exception(_)) => {
            return Ok(());
        }
    }

    uow.commit().await.map_err(TransactionKind::commit_err)?;

    Ok(())
}
