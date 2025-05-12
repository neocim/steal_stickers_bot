use crate::application::{
    common::{
        exceptions::{RepoKind, TransactionKind},
        traits::uow::UoW as UoWTrait,
    },
    set::{dto::create::Create, traits::SetRepo as _},
};

pub async fn create_set<'a, UoW>(uow: &'a mut UoW, set: Create<'a>) -> Result<(), TransactionKind>
where
    UoW: UoWTrait,
{
    let result = uow
        .set_repo()
        .await
        .map_err(TransactionKind::begin_err)?
        .create(set)
        .await;

    match result {
        Ok(_) => (),
        Err(RepoKind::Unexpected(_)) => {
            uow.rollback()
                .await
                .map_err(TransactionKind::rollback_err)?;
        }
        // skip if created
        Err(RepoKind::Exception(_)) => {
            return Ok(());
        }
    };

    uow.commit().await.map_err(TransactionKind::commit_err)?;

    Ok(())
}
