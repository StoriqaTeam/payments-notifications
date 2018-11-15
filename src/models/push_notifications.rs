use models::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushNotifications {
    pub device_id: String,
    pub device_os: String,
    pub transaction: TransactionsResponse,
}
