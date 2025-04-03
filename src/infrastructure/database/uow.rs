use async_trait::async_trait;
use sqlx::{Database, Pool, Transaction};
use telers::FromContext;

use super::repositories::{set::SetRepoImpl, user::UserRepoImpl};
use crate::application::{
    common::{
        exceptions::{BeginError, CommitError, RollbackError},
        traits::uow::{UoW as UnitOfWork, UoWFactory as UoWFactoryTrait},
    },
    set::traits::SetRepo,
    user::traits::UserRepo,
};

impl From<sqlx::Error> for BeginError {
    fn from(error: sqlx::Error) -> Self {
        Self::new(error.to_string())
    }
}

impl From<sqlx::Error> for CommitError {
    fn from(error: sqlx::Error) -> Self {
        Self::new(error.to_string())
    }
}

impl From<sqlx::Error> for RollbackError {
    fn from(error: sqlx::Error) -> Self {
        Self::new(error.to_string())
    }
}

#[derive(FromContext)]
#[context(key = "uow_factory")]
pub struct UoWFactory<DB: Database> {
    pool: Pool<DB>,
}

impl<DB: Database> Clone for UoWFactory<DB> {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
        }
    }
}

pub struct UoW<DB: Database> {
    pool: Pool<DB>,
    transaction: Option<Transaction<'static, DB>>,
}

impl<DB: Database> UoWFactory<DB> {
    pub const fn new(pool: Pool<DB>) -> Self {
        Self { pool }
    }
}

impl<DB: Database> UoW<DB> {
    pub fn new(pool: Pool<DB>, transaction: Option<Transaction<'static, DB>>) -> Self {
        Self { pool, transaction }
    }
}

impl<DB> UoWFactoryTrait for UoWFactory<DB>
where
    DB: Database,
    for<'a> UserRepoImpl<&'a mut DB::Connection>: UserRepo,
    for<'a> SetRepoImpl<&'a mut DB::Connection>: SetRepo,
{
    type UoW = UoW<DB>;

    fn create_uow(&self) -> Self::UoW {
        UoW::new(self.pool.clone(), None)
    }
}

#[async_trait]
impl<DB> UnitOfWork for UoW<DB>
where
    DB: Database,
    for<'a> UserRepoImpl<&'a mut DB::Connection>: UserRepo,
    for<'a> SetRepoImpl<&'a mut DB::Connection>: SetRepo,
{
    type Connection<'a> = &'a mut DB::Connection;
    type UserRepo<'a> = UserRepoImpl<Self::Connection<'a>>;
    type SetRepo<'a> = SetRepoImpl<Self::Connection<'a>>;

    async fn connect(&mut self) -> Result<Self::Connection<'_>, BeginError> {
        if self.transaction.is_none() {
            self.begin().await?
        }

        Ok(self
            .transaction
            .as_mut()
            .expect("transaction is not specified"))
    }

    async fn begin(&mut self) -> Result<(), BeginError> {
        match self.pool.try_begin().await? {
            Some(transaction) => self.transaction = Some(transaction),

            None => self.transaction = Some(self.pool.begin().await?),
        }

        Ok(())
    }

    async fn commit(&mut self) -> Result<(), CommitError> {
        if let Some(transaction) = self.transaction.take() {
            transaction.commit().await?;
        }

        Ok(())
    }

    async fn rollback(&mut self) -> Result<(), RollbackError> {
        if let Some(transaction) = self.transaction.take() {
            transaction.rollback().await.map_err(Into::into)
        } else {
            Ok(())
        }
    }

    async fn set_repo(&mut self) -> Result<Self::SetRepo<'_>, BeginError> {
        Ok(SetRepoImpl::new(self.connect().await?))
    }

    async fn user_repo(&mut self) -> Result<Self::UserRepo<'_>, BeginError> {
        Ok(UserRepoImpl::new(self.connect().await?))
    }
}
