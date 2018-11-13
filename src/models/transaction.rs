use chrono::NaiveDateTime;

use models::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TransactionsResponse {
    pub id: TransactionId,
    pub from: Vec<TransactionAddressInfo>,
    pub to: TransactionAddressInfo,
    pub from_value: String,
    pub from_currency: Currency,
    pub to_value: String,
    pub to_currency: Currency,
    pub fee: String,
    pub status: TransactionStatus,
    pub blockchain_tx_id: Option<BlockchainTransactionId>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct TransactionAddressInfo {
    pub account_id: Option<AccountId>,
    pub owner_name: Option<String>,
    pub blockchain_address: AccountAddress,
}
