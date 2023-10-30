use common_utils::{
    crypto::Encryptable,
    errors::{CustomResult, ValidationError},
};
use error_stack::ResultExt;
use masking::{PeekInterface, Secret};
use time::PrimitiveDateTime;

use crate::types::domain::types::TypeEncryption;

#[derive(Clone, Debug, serde::Serialize)]
pub struct User {
    pub id: Option<i32>,
    pub merchant_id: String,
    pub name: Encryptable<Secret<String>>,
    pub email: String,
    pub password: Encryptable<Secret<serde_json::Value>>,
    pub created_at: PrimitiveDateTime,
}

#[async_trait::async_trait]
impl super::behaviour::Conversion for User {
    type DstType = diesel_models::user::User;
    type NewDstType = diesel_models::user::UserNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(diesel_models::user::User {
            id: self.id.ok_or(ValidationError::MissingRequiredField {
                field_name: "id".to_string(),
            })?,
            merchant_id: self.merchant_id,
            name: self.name.into(),
            email: self.email.into(),
            password: self.password.into(),
            created_at: self.created_at,
        })
    }

    async fn convert_back(
        item: Self::DstType,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        async {
            Ok(Self {
                id: Some(item.id),
                merchant_id: item.merchant_id,
                name: Encryptable::decrypt(item.name, key.peek(), common_utils::crypto::GcmAes256)
                    .await
                    .change_context(ValidationError::InvalidValue {
                        message: "Failed while decrypting user name".to_string(),
                    })?,
                email: item.email,
                password: Encryptable::decrypt(
                    item.password,
                    key.peek(),
                    common_utils::crypto::GcmAes256,
                )
                .await
                .change_context(ValidationError::InvalidValue {
                    message: "Failed while decrypting user password".to_string(),
                })?,
                created_at: item.created_at,
            })
        }
        .await
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting user data".to_string(),
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        let now = common_utils::date_time::now();
        Ok(diesel_models::user::UserNew {
            merchant_id: self.merchant_id,
            name: self.name.into(),
            email: self.email.into(),
            password: self.password.into(),
            created_at: now,
        })
    }
}
