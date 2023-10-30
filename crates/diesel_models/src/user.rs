use diesel::{Identifiable, Insertable, Queryable};
use time::PrimitiveDateTime;

use crate::{encryption::Encryption, schema::users};

#[derive(
    Clone,
    Debug,
    serde::Deserialize,
    serde::Serialize,
    Identifiable,
    Queryable,
    router_derive::DebugAsDisplay,
)]
#[diesel(table_name = users, primary_key(id))]
pub struct User {
    pub id: i32,
    pub merchant_id: String,
    pub name: Encryption,
    pub created_at: PrimitiveDateTime,
    pub email: String,
    pub password: Encryption,
}

#[derive(Clone, Debug, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = users)]
pub struct UserNew {
    pub merchant_id: String,
    pub name: Encryption,
    pub created_at: PrimitiveDateTime,
    pub email: String,
    pub password: Encryption,
}
