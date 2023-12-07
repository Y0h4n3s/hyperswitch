use std::collections::HashMap;
use std::ops::Deref;
use base64::Engine;
use masking::{ExposeOptionInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};
use common_enums::AttemptStatus;
use rsa::{pkcs8::DecodePublicKey, RsaPublicKey, Oaep};
use api_models::enums;

use crate::{types, errors, consts, types::api};
use crate::connector::utils::PaymentsAuthorizeRequestData;

#[derive(Clone, Default, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct PayrabbitRequestFingerprint {
    lat: String,
    lon: String,
    device: String,
    ip: String,
    os: String,
    browser_name: String,
    browser_version: String,
    is_bot: String,
    user_agent: String,
    #[serde(rename="screen-w")]
    screen_w: u32,
    #[serde(rename="screen-h")]
    screen_h: u32,
    timezone: String,
    language: String,
    user_ip: String,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct PaymentsRequest {
    email: String,
    phone: String,
    name: String,
    token: String,
    b64: String,
    b64h: String,
    dni_type: String,
    dni: String,
    installments: u8,
    account_type: String,
    fingerprint: PayrabbitRequestFingerprint

}
impl From<types::BrowserInformation> for PayrabbitRequestFingerprint {
    fn from(value: types::BrowserInformation) -> Self {
        Self {
            screen_h: value.screen_height.unwrap_or_default(),
            screen_w: value.screen_width.unwrap_or_default(),
            language: value.language.unwrap_or_default(),
            timezone: value.time_zone.unwrap_or_default().to_string(),
            ip: match value.ip_address {
                Some(ip) => ip.to_string(),
                None => "0.0.0.0".to_string()
            },
            user_ip: match value.ip_address {
                Some(ip) => ip.to_string(),
                None => "0.0.0.0".to_string()
            },
            user_agent: value.user_agent.unwrap_or_default(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct PaymentIntentRequest {
    amount: i64,
    #[serde(default)]
    cost_buyer: u8,
    #[serde(default)]
    vat_id: u8,
    fingerprint: Option<PayrabbitRequestFingerprint>,
    source: u64,
    description: String,
    references: Option<HashMap<String, String>>,
}

impl Default for PaymentIntentRequest {
    fn default() -> Self {
        Self {
            cost_buyer: 1,
            vat_id: 1,
            fingerprint: Some(PayrabbitRequestFingerprint::default()),
            source: 4,
            amount: 0,
            description: "".to_string(),
            references: Some(HashMap::new()),
        }
    }
}



#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct PayrabbitCard {
    number: String,
    expiry_month: String,
    expiry_year: String,
    cvc: String,
}

impl TryFrom<&types::PaymentsInitRouterData> for PaymentIntentRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsInitRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(_) => {
                Ok(Self {
                    amount: item.request.amount,
                    ..Default::default()
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}


//TODO: Fill the struct with respective fields
pub struct PayrabbitRouterData<T> {
    pub amount: i64, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for PayrabbitRouterData<T>
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
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PayrabbitPaymentInitResponseData {
    amount_due: f64,
    ticket_number: String,
    url_checkout: String,
    pkey: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaymentIntentResponse {
    code: u16,
    message: String,
    data: PayrabbitPaymentInitResponseData,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for PaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let browser_info = item.request.get_browser_info()?;
        let payment_intent: PaymentIntentResponse = if let None = item.connector_meta_data {
            return Err(errors::ConnectorError::ProcessingStepFailed(Some("Payment Not Initialized".into())).into());
        } else {
            serde_json::from_value(item.connector_meta_data
                .as_ref()
                .unwrap()
                .peek()
                .clone()).map_err(|_| errors::ConnectorError::MissingRequiredField { field_name: "connector_meta_data" })?
        };

        match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(req_card) => {
                let mut request = Self {
                    email: item.request.get_email()?.peek().to_string(),
                    phone: "".to_string(),
                    name: req_card.card_holder_name.peek().to_string(),
                    token: payment_intent.data.ticket_number,
                    b64: "".to_string(),
                    b64h: "".to_string(),
                    dni_type: "".to_string(),
                    dni: "".to_string(),
                    installments: 1,
                    account_type: "C".to_string(),
                    fingerprint: PayrabbitRequestFingerprint::from(browser_info)
                };
                match item.address.shipping.clone() {
                    Some(shipping) => {
                        request.phone = shipping.phone.map(|p| {
                            format!(
                                "{}{}",
                                p.country_code.unwrap_or_default(),
                                p.number.expose_option().unwrap_or_default()
                            ).into()
                        }).unwrap_or_default()
                    }
                    None => {}
                }
                let pem = consts::BASE64_ENGINE.decode(payment_intent.data.pkey)
                    .map_err(|e| errors::ConnectorError::RequestEncodingFailedWithReason(e.to_string()))?;

                let public_key = RsaPublicKey::from_public_key_pem(&String::from_utf8(pem).unwrap_or_default())
                    .map_err(|e| errors::ConnectorError::RequestEncodingFailedWithReason(e.to_string()))?;
                let mut rng = rand::thread_rng();
                let data = format!(
                    "{}|{}|{}/{}",
                    req_card.card_issuer.ok_or(errors::ConnectorError::MissingRequiredField { field_name: "card issuer" })?,
                    req_card.card_number.deref().peek().to_string(), req_card.card_exp_month.peek().to_string(),
                    req_card.card_exp_year.peek().to_string()
                );
                let full_data = data.clone() + "|" + req_card.card_cvc.peek();
                let encrypted_data = public_key.encrypt(&mut rng, Oaep::new::<sha2::Sha256>(), data.as_bytes())
                    .map_err(|e| errors::ConnectorError::RequestEncodingFailedWithReason(e.to_string()))?;
                let encrypted_full_data = public_key.encrypt(&mut rng, Oaep::new::<sha2::Sha256>(), full_data.as_bytes())
                    .map_err(|e| errors::ConnectorError::RequestEncodingFailedWithReason(e.to_string()))?;

                request.b64 = consts::BASE64_ENGINE.encode(&encrypted_full_data);
                request.b64h = consts::BASE64_ENGINE.encode(&encrypted_data);
                Ok(request)
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}


impl<F, T> TryFrom<types::ResponseRouterData<F, PaymentIntentResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            PaymentIntentResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let ticket = item.response.data.ticket_number.to_string();

        Ok(Self {
            status: match item.response.code {
                400 | 205 => AttemptStatus::Failure,
                200 => AttemptStatus::Authorized,
                _ => AttemptStatus::Pending,
            },
            response: Ok(types::PaymentsResponseData::PreProcessingResponse {
                pre_processing_id:  types::PreprocessingResponseId::ConnectorTransactionId(ticket),
                connector_metadata: Some(serde_json::to_value(&item.response).unwrap()),
                connector_response_reference_id: None,
                session_token: None,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaymentsResponse {
    code: u16,
    message: String,
    ticket_number: Option<String>,
}

impl<F, T> TryFrom<types::ResponseRouterData<F, PaymentsResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            PaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {

        Ok(Self {
            status: match item.response.code {
                400 | 205 | 404 => AttemptStatus::Failure,
                200 => AttemptStatus::Charged,
                _ => AttemptStatus::Pending,
            },
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: if item.response.ticket_number.is_some() { types::ResponseId::ConnectorTransactionId(item.response.ticket_number.as_ref().unwrap().to_string()) } else { types::ResponseId::NoResponseId },
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: Some(serde_json::to_value(&item.response).unwrap()),
                network_txn_id: None,
                connector_response_reference_id: None,
            }),
            ..item.data
        })
    }
}


//TODO: Fill the struct with respective fields
// Auth Struct
pub struct PayrabbitAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for PayrabbitAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PayrabbitPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<PayrabbitPaymentStatus> for AttemptStatus {
    fn from(item: PayrabbitPaymentStatus) -> Self {
        match item {
            PayrabbitPaymentStatus::Succeeded => Self::Charged,
            PayrabbitPaymentStatus::Failed => Self::Failure,
            PayrabbitPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PayrabbitPaymentsResponse {
    status: PayrabbitPaymentStatus,
    id: String,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, PayrabbitPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            PayrabbitPaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct PayrabbitRefundRequest {
    pub amount: i64,
}

impl<F> TryFrom<&PayrabbitRouterData<&types::RefundsRouterData<F>>> for PayrabbitRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PayrabbitRouterData<&types::RefundsRouterData<F>>,
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
pub struct PayrabbitErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}


#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct PayrabbitAuthRequest {
    didentity: Secret<String>,

}

impl TryFrom<&types::RefreshTokenRouterData> for PayrabbitAuthRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefreshTokenRouterData) -> Result<Self, Self::Error> {
        let auth = PayrabbitAuthType::try_from(&item.connector_auth_type)?;
        let key_format = format!("{{\"apikey\": \"{}\"}}", auth.api_key.peek());
        let encoded = Secret::new(consts::BASE64_ENGINE.encode(key_format));
        Ok(Self {
            /// TODO: change back to encoded
            didentity: encoded
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct PayrabbitAuthResponse {
    code: u16,
    jwt_access: Secret<String>,
    message: String,
}

impl<F, T> TryFrom<types::ResponseRouterData<F, PayrabbitAuthResponse, T, types::AccessToken>>
for types::RouterData<F, T, types::AccessToken>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, PayrabbitAuthResponse, T, types::AccessToken>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::AccessToken {
                token: item.response.jwt_access,
                expires: (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() + 5 * 60) as i64,
            }),
            ..item.data
        })
    }
}

impl TryFrom<&types::PaymentsAuthorizeSessionTokenRouterData> for PayrabbitAuthRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeSessionTokenRouterData) -> Result<Self, Self::Error> {
        let auth = PayrabbitAuthType::try_from(&item.connector_auth_type)?;
        let key_format = format!("{{\"apikey\": \"{}\"}}", auth.api_key.peek());
        let encoded = consts::BASE64_ENGINE.encode(key_format);
        Ok(Self {
            didentity: Secret::new(encoded)
        })
    }
}

impl<F, T> TryFrom<types::ResponseRouterData<F, PayrabbitAuthResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, PayrabbitAuthResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            access_token: Some(types::AccessToken {
                token: item.response.jwt_access,
                expires: (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() + 5 * 60) as i64,
            }),
            ..item.data
        })
    }
}