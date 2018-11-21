#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Email {
    pub to: String,
    pub subject: String,
    pub text: String,
}
