use std::borrow::Cow;

pub trait ApplicationException {}
pub trait UnexpectedError {}

#[derive(Debug, thiserror::Error)]
#[error("Repository error: {message}")]
pub struct RepoError {
    message: Cow<'static, str>,
}

impl UnexpectedError for RepoError {}

impl RepoError {
    pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RepoKind<RepoException>
where
    RepoException: ApplicationException,
{
    #[error(transparent)]
    Exception(RepoException),

    #[error(transparent)]
    Unexpected(RepoError),
}

impl<RepoException> RepoKind<RepoException>
where
    RepoException: ApplicationException,
{
    pub fn exception(exception: impl Into<RepoException>) -> Self {
        Self::Exception(exception.into())
    }

    pub fn unexpected(error: impl Into<RepoError>) -> Self {
        Self::Unexpected(error.into())
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Begin transaction error: {message}")]
pub struct BeginError {
    message: Cow<'static, str>,
}

impl BeginError {
    pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl ApplicationException for BeginError {}

#[derive(Debug, thiserror::Error)]
#[error("Commit transaction error: {message}")]
pub struct CommitError {
    message: Cow<'static, str>,
}

impl CommitError {
    pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl ApplicationException for CommitError {}

#[derive(Debug, thiserror::Error)]
#[error("Rollback transaction error: {message}")]
pub struct RollbackError {
    message: Cow<'static, str>,
}

impl RollbackError {
    pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl ApplicationException for RollbackError {}

#[derive(Debug, thiserror::Error)]
pub enum TransactionKind {
    #[error(transparent)]
    BeginError(BeginError),

    #[error(transparent)]
    CommitError(CommitError),

    #[error(transparent)]
    RollbackError(RollbackError),
}

impl TransactionKind {
    pub fn begin_err(err: impl Into<BeginError>) -> Self {
        Self::BeginError(err.into())
    }

    pub fn commit_err(err: impl Into<CommitError>) -> Self {
        Self::CommitError(err.into())
    }

    pub fn rollback_err(err: impl Into<RollbackError>) -> Self {
        Self::RollbackError(err.into())
    }
}

impl ApplicationException for TransactionKind {}
