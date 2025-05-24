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
    subject: String,
    description: String,
    image_url: String,
    post_url: String,
) {
    let api_key = std::env::var("BREVO_API_KEY").expect("BREVO_API_KEY not set");

    let scheduled_at = chrono::Utc::now() + chrono::Duration::hours(1);
    let mut params = std::collections::HashMap::new();
    params.insert("TITLE".to_string(), subject.to_string());
    params.insert("DESCRIPTION".to_string(), description.to_string());
    params.insert("IMAGE_URL".to_string(), image_url.to_string());
    params.insert("POST_URL".to_string(), post_url.to_string());

    let campaign = EmailCampaign {
        tag: "plog".to_string(),
        name: subject.to_string(),
        subject: subject.to_string(),
        params: params,
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
        .json(&serde_json::json!(&campaign))
        .send()
        .await;
    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                println!("Campaign posted successfully!");
            } else {
                let status = resp.status();
                let error: BrevoResponse = resp.json().await.unwrap_or_else(|_| BrevoResponse {
                    code: "Unknown error".to_string(),
                    message: "Failed to parse error response".to_string(),
                });
                println!(
                    "Failed to post campaign: {}: {}:{}",
                    status, error.message, error.code
                );
                println!("JSON: {}", serde_json::to_string_pretty(&campaign).unwrap());
            }
        }
        Err(e) => {
            println!("Error occurred: {}", e);
        }
    }
}
