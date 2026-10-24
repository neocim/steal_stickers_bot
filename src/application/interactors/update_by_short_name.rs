use crate::application::{
    common::{
        exceptions::{RepoKind, TransactionKind},
        traits::uow::UoW as UoWTrait,
    },
    set::{dto::update_by_short_name::UpdateSetByShortName, repository::SetRepo as _},
};

pub async fn update_by_short_name<UoW>(
    uow: &mut UoW,
    set: UpdateSetByShortName<'_>,
) -> Result<(), TransactionKind>
where
    UoW: UoWTrait,
{
    let result = uow
        .set_repo()
        .await
        .map_err(TransactionKind::begin_err)?
        .update_by_short_name(set)
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
