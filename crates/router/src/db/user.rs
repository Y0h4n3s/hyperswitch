use error_stack::{IntoReport, ResultExt};
use masking::Secret;
use storage_impl::redis::cache::ACCOUNTS_CACHE;

use super::{MockDb, Store};
use crate::{
    connection,
    core::errors::{CustomResult, StorageError},
    types::{
        domain::{
            behaviour::{Conversion, ReverseConversion},
            MerchantKeyStore, User,
        },
        storage,
    },
};

#[async_trait::async_trait]
pub trait UserAccountInterface
where
    User: Conversion<DstType = storage::User, NewDstType = storage::UserNew>,
{
    async fn insert_user(
        &self,
        user_account: User,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<User, StorageError>;

    async fn find_user_by_email(
        &self,
        email: &str,
    ) -> CustomResult<diesel_models::User, StorageError>;
}

#[async_trait::async_trait]
impl UserAccountInterface for Store {
    async fn insert_user(
        &self,
        user_account: User,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<User, StorageError> {
        let conn = connection::pg_connection_write(self).await?;

        user_account
            .construct_new()
            .await
            .change_context(StorageError::EncryptionError)?
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()?
            .convert(key_store.key.get_inner())
            .await
            .change_context(StorageError::DecryptionError)
    }

    async fn find_user_by_email(
        &self,
        email: &str,
    ) -> CustomResult<diesel_models::User, StorageError> {
        let fetch_func = || async {
            let conn = connection::pg_connection_read(self).await?;
            storage::User::find_by_email(&conn, Secret::new(email.to_string()))
                .await
                .map_err(Into::into)
                .into_report()
        };

        #[cfg(not(feature = "accounts_cache"))]
        {
            fetch_func().await
        }

        #[cfg(feature = "accounts_cache")]
        {
            super::cache::get_or_populate_in_memory(self, email, fetch_func, &ACCOUNTS_CACHE).await
        }
    }
}

#[async_trait::async_trait]
impl UserAccountInterface for MockDb {
    #[allow(clippy::panic)]
    async fn insert_user(
        &self,
        mut user_account: User,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<User, StorageError> {
        let mut accounts = self.users.lock().await;
        user_account.id.get_or_insert(
            accounts
                .len()
                .try_into()
                .into_report()
                .change_context(StorageError::MockDbError)?,
        );
        let account = Conversion::convert(user_account)
            .await
            .change_context(StorageError::EncryptionError)?;
        accounts.push(account.clone());
        account
            .convert(key_store.key.get_inner())
            .await
            .change_context(StorageError::DecryptionError)
    }

    #[allow(clippy::panic)]
    async fn find_user_by_email(
        &self,
        _email: &str,
    ) -> CustomResult<diesel_models::User, StorageError> {
        Err(StorageError::MockDbError.into())
    }
}
