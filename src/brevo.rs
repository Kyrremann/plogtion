use log::{error, info};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct Sender {
    pub id: i32,
}

#[derive(Serialize)]
struct Recipients {
    #[serde(rename = "listIds")]
    pub list_ids: Vec<i32>,
}

#[derive(Serialize)]
struct EmailCampaign {
    pub tag: String,
    pub name: String,
    pub sender: Sender,
    pub recipients: Recipients,
    pub subject: String,
    #[serde(rename = "templateId")]
    pub template_id: i32,
    pub params: std::collections::HashMap<String, String>,
    #[serde(rename = "scheduledAt")]
    pub scheduled_at: String,
}

#[derive(Deserialize)]
struct BrevoResponse {
    code: String,
    message: String,
}

pub async fn post_campaign(
    title: String,
    description: String,
    image_url: String,
    post_url: String,
) -> Result<(), String> {
    let api_key = std::env::var("BREVO_API_KEY").map_err(|_| "BREVO_API_KEY not set".to_string())?;

    let scheduled_at = chrono::Utc::now() + chrono::Duration::hours(1);
    let mut params = std::collections::HashMap::new();
    params.insert("TITLE".to_string(), title.clone());
    params.insert("DESCRIPTION".to_string(), description.clone());
    params.insert("IMAGE_URL".to_string(), image_url.clone());
    params.insert("POST_URL".to_string(), post_url.clone());

    let campaign = EmailCampaign {
        tag: "plog".to_string(),
        name: title.clone(),
        subject: title.clone(),
        params,
        scheduled_at: scheduled_at.to_rfc3339(),
        sender: Sender { id: 2 },
        recipients: Recipients { list_ids: vec![2] },
        template_id: 6,
    };

    let response = reqwest::Client::new()
        .post("https://api.brevo.com/v3/emailCampaigns")
        .header("api-key", api_key)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&campaign)
        .send()
        .await
        .map_err(|err| format!("Request failed: {}", err))?;

    if response.status().is_success() {
        info!("Campaign posted successfully!");
        Ok(())
    } else {
        let status = response.status();
        let error: BrevoResponse = response.json().await.unwrap_or_else(|_| BrevoResponse {
            code: "Unknown error".to_string(),
            message: "Failed to parse error response".to_string(),
        });
        error!("Failed to post campaign: {}: {}:{}", status, error.message, error.code);
        Err(format!("Failed to post campaign: {}: {}", status, error.message))
    }
}
