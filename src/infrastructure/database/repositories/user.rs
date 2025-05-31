use async_trait::async_trait;
use sea_query::{Alias, Expr, PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder as _;
use sqlx::PgConnection;
use tracing::debug;

use crate::{
    application::{
        common::exceptions::RepoKind,
        user::{
            dto::{create::Create, get_by_tg_id::GetByTgID},
            exceptions::{UserTgIdAlreadyExists, UserTgIdNotExist},
            repository::UserRepo,
        },
    },
    domain::entities::user::User,
    infrastructure::database::models::user::User as UserModel,
};

pub struct UserRepoImpl<Conn> {
    conn: Conn,
}

impl<Conn> UserRepoImpl<Conn> {
    pub fn new(conn: Conn) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl UserRepo for UserRepoImpl<&mut PgConnection> {
    async fn create(&mut self, user: Create) -> Result<(), RepoKind<UserTgIdAlreadyExists>> {
        let (sql_query, values) = Query::insert()
            .into_table(Alias::new("users"))
            .columns([Alias::new("tg_id")])
            .values_panic([user.tg_id().into()])
            .build_sqlx(PostgresQueryBuilder);

        debug!("Postgres query: {sql_query};\nValues for query: {values:?}");

        sqlx::query_with(&sql_query, values)
            .execute(&mut *self.conn)
            .await
            .map(|_| ())
            .map_err(|err| {
                if let Some(err) = err.as_database_error() {
                    if let Some(code) = err.code() {
                        // if unique `tg_id` already exists
                        if code == "23505" {
                            return RepoKind::exception(UserTgIdAlreadyExists::new(
                                user.tg_id(),
                                err.to_string(),
                            ));
                        }
                    }
                }

                RepoKind::unexpected(err)
            })
    }

    async fn get_by_tg_id(&mut self, user: GetByTgID) -> Result<User, RepoKind<UserTgIdNotExist>> {
        let (sql_query, values) = Query::select()
            .columns([Alias::new("tg_id"), Alias::new("created")])
            .from(Alias::new("users"))
            .and_where(Expr::col(Alias::new("tg_id")).eq(user.tg_id()))
            .build_sqlx(PostgresQueryBuilder);

        debug!("Postgres query: {sql_query};\nValues for query: {values:?}");

        sqlx::query_as_with(&sql_query, values)
            .fetch_one(&mut *self.conn)
            .await
            .map(|user_model: UserModel| user_model.into())
            .map_err(|err| {
                if let sqlx::Error::RowNotFound = err {
                    return RepoKind::exception(UserTgIdNotExist::new(
                        user.tg_id(),
                        err.to_string(),
                    ));
                }

                RepoKind::unexpected(err)
            })
    }
}
