use crate::application::{
    common::exceptions::{BeginError, CommitError, RollbackError},
    set::repository::SetRepo,
    user::repository::UserRepo,
};

pub trait UoW: Send {
    type Connection<'a>
    where
        Self: 'a;

    type UserRepo<'a>: UserRepo
    where
        Self: 'a;

    type SetRepo<'a>: SetRepo
    where
        Self: 'a;

    fn connect(&mut self) -> impl Future<Output = Result<Self::Connection<'_>, BeginError>> + Send;

    fn begin(&mut self) -> impl Future<Output = Result<(), BeginError>> + Send;

    fn commit(&mut self) -> impl Future<Output = Result<(), CommitError>> + Send;

    fn rollback(&mut self) -> impl Future<Output = Result<(), RollbackError>> + Send;

    fn user_repo(&mut self) -> impl Future<Output = Result<Self::UserRepo<'_>, BeginError>> + Send;

    fn set_repo(&mut self) -> impl Future<Output = Result<Self::SetRepo<'_>, BeginError>> + Send;
}

pub trait UoWFactory {
    type UoW: UoW;

    fn create_uow(&self) -> Self::UoW;
}
