use models::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendGridPayload {
    pub personalizations: Vec<Personalization>,
    pub from: Address,
    pub subject: String,
    pub content: Vec<Content>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Personalization {
    pub to: Vec<Address>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub email: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    #[serde(rename = "type")]
    pub type_field: String,
    pub value: String,
}

impl SendGridPayload {
    pub fn from_email(email: Email, from: String) -> Self {
        let mut to: Vec<Address> = Vec::new();
        to.push(Address {
            email: email.to.clone(),
            name: None,
        });

        let mut personalizations: Vec<Personalization> = Vec::new();
        personalizations.push(Personalization { to });

        let from = Address { email: from, name: None };

        let subject = email.subject.clone();

        let mut content: Vec<Content> = Vec::new();
        content.push(Content {
            type_field: "text/html".to_string(),
            value: email.text,
        });

        Self {
            personalizations,
            from,
            subject,
            content,
        }
    }
}
