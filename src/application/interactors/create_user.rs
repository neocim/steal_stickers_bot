use crate::application::{
    common::{
        exceptions::{RepoKind, TransactionKind},
        traits::uow::UoW as UoWTrait,
    },
    user::{dto::create::Create, traits::UserRepo as _},
};

pub async fn create_user<UoW>(uow: &mut UoW, user: Create) -> Result<(), TransactionKind>
where
    UoW: UoWTrait,
{
    let result = uow
        .user_repo()
        .await
        .map_err(TransactionKind::begin_err)?
        .create(user)
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
