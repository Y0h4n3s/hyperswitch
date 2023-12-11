use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    core::errors,
    types::{self, api, storage::enums},
};
use crate::connector::utils::AccessTokenRequestInfo;

//TODO: Fill the struct with respective fields
pub struct CreditbancoRouterData<T> {
    pub amount: i64, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T>
TryFrom<(
    &types::api::CurrencyUnit,
    types::storage::enums::Currency,
    i64,
    T,
)> for CreditbancoRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (_currency_unit, _currency, amount, item): (
            &types::api::CurrencyUnit,
            types::storage::enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct CreditbancoReference {
    reference_key: String,
    reference_description: String
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct CreditbancoPaymentsRequest {
    amount: i64,
    card_data: CreditbancoCard,
    unique_code: String,
    terminal_id: String,
    references: Vec<CreditbancoReference>,
    currency_code: u16

}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct CreditbancoCard {
    name: Secret<String>,
    card_number: cards::CardNumber,
    card_expire_month: Secret<String>,
    card_expire_year: Secret<String>,
    cvv: Secret<String>,
    brand_id: Secret<u8>,
    card_account_type_id: Secret<u8>,
}

impl TryFrom<&CreditbancoRouterData<&types::PaymentsAuthorizeRouterData>>
for CreditbancoPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CreditbancoRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(req_card) => {
                let card_data = CreditbancoCard {
                    name: req_card.card_holder_name.ok_or(
                        errors::ConnectorError::MissingRequiredField { field_name: "card_holder_name"},
                    )?,
                    card_number: req_card.card_number,
                    card_expire_month: req_card.card_exp_month,
                    card_expire_year: req_card.card_exp_year,
                    cvv: req_card.card_cvc,
                    brand_id: Default::default(),
                    card_account_type_id: Default::default(),
                };
                Ok(Self {
                    amount: item.amount.to_owned(),
                    card_data,
                    ..Default::default()
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct CreditbancoAuthType {
    pub(super) client_id: Secret<String>,
    pub(super) client_secret: Secret<String>,
    pub(super) username: Secret<String>,
    pub(super) password: Secret<String>,

}

impl TryFrom<&types::ConnectorAuthType> for CreditbancoAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::MultiAuthKey { api_key, key1, api_secret, key2 } => Ok(Self {
                client_id: api_key.to_owned(),
                client_secret: key1.to_owned(),
                username: api_secret.to_owned(),
                password: key2.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CreditbancoPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<CreditbancoPaymentStatus> for enums::AttemptStatus {
    fn from(item: CreditbancoPaymentStatus) -> Self {
        match item {
            CreditbancoPaymentStatus::Succeeded => Self::Charged,
            CreditbancoPaymentStatus::Failed => Self::Failure,
            CreditbancoPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreditbancoPaymentsResponse {
    status: CreditbancoPaymentStatus,
    id: String,
}

impl<F, T>
TryFrom<
    types::ResponseRouterData<F, CreditbancoPaymentsResponse, T, types::PaymentsResponseData>,
> for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            CreditbancoPaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct CreditbancoRefundRequest {
    pub amount: i64,
}

impl<F> TryFrom<&CreditbancoRouterData<&types::RefundsRouterData<F>>> for CreditbancoRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CreditbancoRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Processing => Self::Pending,
            //TODO: Review mapping
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: RefundStatus,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct CreditbancoErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub enum CreditbancoAuthGrantType {
    #[default]
    #[serde(rename = "password")]
    Password
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct CreditbancoAuthRequest {
    pub username: Secret<String>,
    pub password: Secret<String>,
    pub grant_type: CreditbancoAuthGrantType,
    pub client_id: Secret<String>,
    pub client_secret: Secret<String>,
}

impl TryFrom<&types::RefreshTokenRouterData> for CreditbancoAuthRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefreshTokenRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            username: item.request.username.clone().unwrap_or(Secret::new("".to_string())),
            password: item.request.password.clone().unwrap_or(Secret::new("".to_string())),
            grant_type: CreditbancoAuthGrantType::Password,
            client_id: item.request.app_id.clone(),
            client_secret:  item.get_request_id()?,
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct CreditbancoAuthResponse {
    pub access_token: Secret<String>,
    pub refresh_expires_in: i64,
    pub expires_in: i64,
    pub refresh_token: Secret<String>,
    pub token_type: String,
}

impl<F, T> TryFrom<types::ResponseRouterData<F, CreditbancoAuthResponse, T, types::AccessToken>>
for types::RouterData<F, T, types::AccessToken>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, CreditbancoAuthResponse, T, types::AccessToken>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::AccessToken {
                token: item.response.access_token,
                expires: item.response.expires_in,
            }),
            ..item.data
        })
    }
}