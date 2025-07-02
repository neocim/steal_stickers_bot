use async_trait::async_trait;
use sea_query::{Alias, Expr, Func, Order, PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use sqlx::PgConnection;
use tracing::debug;

use crate::{
    application::{
        common::exceptions::{RepoError, RepoKind},
        set::{
            dto::{
                count_by_tg_id::CountByTgID, create::Create,
                delete_by_short_name::DeleteByShortName, get_all::GetAll,
                get_by_short_name::GetByShortName, get_by_tg_id::GetByTgID,
                set_deleted_col_by_short_name::SetDeletedColByShortName,
            },
            exceptions::{SetShortNameAlreadyExist, SetShortNameNotExist, SetTgIdNotExist},
            repository::SetRepo,
        },
    },
    domain::entities::set::Set,
    infrastructure::database::models::set::{Set as SetModel, SetCount},
};

pub struct SetRepoImpl<Conn> {
    conn: Conn,
}

impl<Conn> SetRepoImpl<Conn> {
    pub fn new(conn: Conn) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl SetRepo for SetRepoImpl<&mut PgConnection> {
    async fn create<'a>(
        &'a mut self,
        set: Create<'a>,
    ) -> Result<(), RepoKind<SetShortNameAlreadyExist>> {
        let (sql_query, values) = Query::insert()
            .into_table(Alias::new("sets"))
            .columns([
                Alias::new("tg_id"),
                Alias::new("short_name"),
                Alias::new("title"),
            ])
            .values_panic([
                set.tg_id().into(),
                set.short_name().into(),
                set.title().into(),
            ])
            .build_sqlx(PostgresQueryBuilder);

        debug!("Postgres `create` query: `{sql_query}`;\nValues for query: `{values:?}`");

        sqlx::query_with(&sql_query, values)
            .execute(&mut *self.conn)
            .await
            .map(|_| ())
            .map_err(|err| {
                if let Some(err) = err.as_database_error() {
                    if let Some(code) = err.code() {
                        if code == "23505" {
                            return RepoKind::exception(SetShortNameAlreadyExist::new(
                                set.short_name().to_string(),
                                err.to_string(),
                            ));
                        }
                    }
                }

                RepoKind::unexpected(err)
            })
    }

    async fn delete_by_short_name<'a>(
        &'a mut self,
        set: DeleteByShortName<'a>,
    ) -> Result<(), RepoKind<SetShortNameNotExist>> {
        let (sql_query, values) = Query::delete()
            .from_table(Alias::new("sets"))
            .and_where(Expr::col(Alias::new("short_name")).eq(set.short_name()))
            .build_sqlx(PostgresQueryBuilder);

        debug!(
            "Postgres `delete_by_short_name` query: `{sql_query}`;\nValues for query: `{values:?}`"
        );

        sqlx::query_with(&sql_query, values)
            .execute(&mut *self.conn)
            .await
            .map(|_| ())
            .map_err(|err| {
                if let sqlx::Error::RowNotFound = err {
                    return RepoKind::exception(SetShortNameNotExist::new(
                        set.short_name().to_string(),
                        err.to_string(),
                    ));
                }

                RepoKind::unexpected(err)
            })
    }

    async fn get_by_tg_id(
        &mut self,
        set: GetByTgID,
    ) -> Result<Vec<Set>, RepoKind<SetTgIdNotExist>> {
        let (sql_query, values) = if set.get_deleted().is_some() {
            Query::select()
                .columns([
                    Alias::new("tg_id"),
                    Alias::new("short_name"),
                    Alias::new("title"),
                    Alias::new("deleted"),
                ])
                .from(Alias::new("sets"))
                .and_where(Expr::col(Alias::new("tg_id")).eq(set.tg_id()))
                .and_where(
                    Expr::col(Alias::new("deleted"))
                        .eq(set.get_deleted().expect("`get_deleted` is None")),
                )
                .build_sqlx(PostgresQueryBuilder)
        } else {
            Query::select()
                .columns([
                    Alias::new("tg_id"),
                    Alias::new("short_name"),
                    Alias::new("title"),
                    Alias::new("deleted"),
                ])
                .from(Alias::new("sets"))
                .and_where(Expr::col(Alias::new("tg_id")).eq(set.tg_id()))
                .build_sqlx(PostgresQueryBuilder)
        };

        debug!("Postgres `get_by_tg_id` query: `{sql_query}`;\nValues for query: `{values:?}`");

        sqlx::query_as_with(&sql_query, values)
            .fetch_all(&mut *self.conn)
            .await
            .map(|set_model: Vec<SetModel>| set_model.into_iter().map(Into::into).collect())
            .map_err(|err| {
                if let sqlx::Error::RowNotFound = err {
                    return RepoKind::exception(SetTgIdNotExist::new(set.tg_id(), err.to_string()));
                }

                RepoKind::unexpected(err)
            })
    }

    async fn get_one_by_short_name<'a>(
        &'a mut self,
        set: GetByShortName<'a>,
    ) -> Result<Set, RepoKind<SetShortNameNotExist>> {
        let (sql_query, values) = Query::select()
            .columns([
                Alias::new("tg_id"),
                Alias::new("short_name"),
                Alias::new("title"),
            ])
            .from(Alias::new("sets"))
            .and_where(Expr::col(Alias::new("short_name")).eq(set.short_name()))
            .build_sqlx(PostgresQueryBuilder);

        debug!(
            "Postgres `get_one_by_short_name` query: `{sql_query}`;\nValues for query: `{values:?}`"
        );

        sqlx::query_as_with(&sql_query, values)
            .fetch_one(&mut *self.conn)
            .await
            .map(|set_model: SetModel| set_model.into())
            .map_err(|err| {
                if let sqlx::Error::RowNotFound = err {
                    return RepoKind::exception(SetShortNameNotExist::new(
                        set.short_name().to_string(),
                        err.to_string(),
                    ));
                }

                RepoKind::unexpected(err)
            })
    }

    async fn set_deleted_col_by_short_name<'a>(
        &'a mut self,
        set: SetDeletedColByShortName<'a>,
    ) -> Result<(), RepoKind<SetShortNameNotExist>> {
        let (sql_query, values) = Query::update()
            .table(Alias::new("sets"))
            .value(Alias::new("deleted"), set.deleted())
            .and_where(Expr::col(Alias::new("short_name")).eq(set.short_name()))
            .build_sqlx(PostgresQueryBuilder);

        debug!(
            "Postgres `set_deleted_col_by_short_name` query: `{sql_query}`;\nValues for query: `{values:?}`"
        );

        sqlx::query_with(&sql_query, values)
            .execute(&mut *self.conn)
            .await
            .map(|_| ())
            .map_err(|err| {
                if let sqlx::Error::RowNotFound = err {
                    return RepoKind::exception(SetShortNameNotExist::new(
                        set.short_name().to_string(),
                        err.to_string(),
                    ));
                }

                RepoKind::unexpected(err)
            })
    }

    async fn get_all(&mut self, set: GetAll) -> Result<Vec<Set>, RepoError> {
        let (sql_query, values) = if set.get_deleted().is_some() {
            Query::select()
                .columns([
                    Alias::new("tg_id"),
                    Alias::new("short_name"),
                    Alias::new("title"),
                    Alias::new("deleted"),
                ])
                .from(Alias::new("sets"))
                .and_where(
                    Expr::col(Alias::new("deleted"))
                        .eq(set.get_deleted().expect("`get_deleted` is None")),
                )
                .build_sqlx(PostgresQueryBuilder)
        } else {
            Query::select()
                .columns([
                    Alias::new("tg_id"),
                    Alias::new("short_name"),
                    Alias::new("title"),
                    Alias::new("deleted"),
                ])
                .from(Alias::new("sets"))
                .build_sqlx(PostgresQueryBuilder)
        };

        debug!("Postgres `get_all` query: `{sql_query}`;\nValues for query: `{values:?}`");

        sqlx::query_as_with(&sql_query, values)
            .fetch_all(&mut *self.conn)
            .await
            .map(|set_model: Vec<SetModel>| set_model.into_iter().map(Into::into).collect())
            .map_err(|err| RepoError::new(err.to_string()))
    }

    async fn count_by_tg_id(&mut self, set: CountByTgID) -> Result<i64, RepoError> {
        let (sql_query, values) = if set.count_deleted().is_some() {
            Query::select()
                .expr(Func::count(Expr::col(Alias::new("tg_id"))))
                .from(Alias::new("sets"))
                .and_where(Expr::col(Alias::new("tg_id")).eq(set.tg_id()))
                .and_where(
                    Expr::col(Alias::new("deleted"))
                        .eq(set.count_deleted().expect("`get_deleted` is None")),
                )
                .build_sqlx(PostgresQueryBuilder)
        } else {
            Query::select()
                .expr(Func::count(Expr::col(Alias::new("tg_id"))))
                .from(Alias::new("sets"))
                .and_where(Expr::col(Alias::new("tg_id")).eq(set.tg_id()))
                .build_sqlx(PostgresQueryBuilder)
        };

        debug!("Postgres `count_by_tg_id` query: `{sql_query}`;\nValues for query: `{values:?}`");

        sqlx::query_as_with(&sql_query, values)
            .fetch_one(&mut *self.conn)
            .await
            .map(|count: SetCount| count.into())
            .map_err(|err| RepoError::new(err.to_string()))
    }

    async fn get_set_counts_for_all_users(&mut self, set: GetAll) -> Result<Vec<i64>, RepoError> {
        let (sql_query, values) = if set.get_deleted().is_some() {
            Query::select()
                .expr_as(Func::count(1), Alias::new("count"))
                .from(Alias::new("users"))
                .inner_join(
                    Alias::new("sets"),
                    Expr::col((Alias::new("sets"), Alias::new("tg_id")))
                        .eq(Expr::col((Alias::new("users"), Alias::new("tg_id")))),
                )
                .and_where(
                    Expr::col(Alias::new("deleted"))
                        .eq(set.get_deleted().expect("`deleted` is None")),
                )
                .group_by_col((Alias::new("users"), Alias::new("tg_id")))
                .order_by(Alias::new("count"), Order::Desc)
                .build_sqlx(PostgresQueryBuilder)
        } else {
            Query::select()
                .expr_as(Func::count(1), Alias::new("count"))
                .from(Alias::new("users"))
                .inner_join(
                    Alias::new("sets"),
                    Expr::col((Alias::new("sets"), Alias::new("tg_id")))
                        .eq(Expr::col((Alias::new("users"), Alias::new("tg_id")))),
                )
                .group_by_col((Alias::new("users"), Alias::new("tg_id")))
                .order_by(Alias::new("count"), Order::Desc)
                .build_sqlx(PostgresQueryBuilder)
        };

        debug!(
            "Postgres `get_sets_count_for_all_users` query: `{sql_query}`;\nValues for query: `{values:?}`"
        );

        sqlx::query_as_with(&sql_query, values)
            .fetch_all(&mut *self.conn)
            .await
            .map(|set_counts: Vec<SetCount>| set_counts.into_iter().map(Into::into).collect())
            .map_err(|err| RepoError::new(err.to_string()))
    }
}
