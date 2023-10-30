use diesel::{associations::HasTable, ExpressionMethods};
use masking::{PeekInterface, Secret};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    schema::users::dsl,
    user::{User, UserNew},
    PgPooledConn, StorageResult,
};

impl UserNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<User> {
        generics::generic_insert(conn, self).await
    }
}

impl User {
    #[instrument(skip(conn))]
    pub async fn find_by_email(conn: &PgPooledConn, email: Secret<String>) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::email.eq(email.peek().to_owned()),
        )
        .await
    }
}
