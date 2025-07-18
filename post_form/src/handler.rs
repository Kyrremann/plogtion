mod brevo;
mod git;
mod tera;

use crate::tera::UploadForm;
use axum::extract::Multipart;
use axum::http::StatusCode;
use axum::response::Html;
use chrono::{Datelike, NaiveDate};
use log::{error, info};
use serde::Deserialize;

const DEFAULT_IMAGE_URL: &str = "https://kyrremann-plog.s3.nl-ams.scw.cloud";

#[derive(Deserialize)]
pub struct Geocoding {
    #[serde(default)]
    pub suburb: String,
    #[serde(default)]
    pub city: String,
    #[serde(default)]
    pub municipality: String,
    pub country: String,
}

#[derive(Deserialize)]
pub struct Location {
    pub geocoding: Geocoding,
    pub latitude: f64,
    pub longitude: f64,
}

impl Location {
    fn geocoding_as_string(&self) -> String {
        let mut parts = vec![];
        if !self.geocoding.suburb.is_empty() {
            parts.push(self.geocoding.suburb.clone());
        }
        if !self.geocoding.city.is_empty() && parts.contains(&self.geocoding.city) {
            parts.push(self.geocoding.city.clone());
        }
        if !self.geocoding.municipality.is_empty() && !parts.contains(&self.geocoding.municipality)
        {
            parts.push(self.geocoding.municipality.clone());
        }
        if !self.geocoding.country.is_empty() && !parts.contains(&self.geocoding.country) {
            parts.push(self.geocoding.country.clone());
        }

        parts.join(", ")
    }
}

pub async fn handle(mut multipart: Multipart) -> Result<Html<String>, (StatusCode, String)> {
    env_logger::try_init().unwrap_or_else(|_| {
        eprintln!("Failed to initialize logger, using default settings");
    });
    info!("Payload received...");

    let mut form = UploadForm {
        ..Default::default()
    };
    let mut token = String::new();

    while let Some(field) = multipart.next_field().await.map_err(|err| {
        error!("Failed to read multipart field: {}", err);
        (
            StatusCode::BAD_REQUEST,
            "Failed to read multipart field".to_string(),
        )
    })? {
        let name = field.name().unwrap_or_default().to_string();
        info!("Processing field: {}", name);

        match name.as_str() {
            "token" => token = field.text().await.unwrap_or_default(),
            "title" => form.title = field.text().await.unwrap_or_default(),
            "strava" => form.strava = field.text().await.unwrap_or_default(),
            "date" => form.date = field.text().await.unwrap_or_default(),
            "categories" => form.categories = field.text().await.unwrap_or_default(),
            name if name.ends_with("_alt_text") => {
                let text = field.text().await.unwrap_or_default();
                let key = name.strip_suffix("_alt_text").unwrap_or_default();
                form.images.entry(key.to_string()).or_default().alt_text = text;
            }
            name if name.ends_with("_caption") => {
                let text = field.text().await.unwrap_or_default();
                let key = name.strip_suffix("_caption").unwrap_or_default();
                form.images.entry(key.to_string()).or_default().caption = text;
            }
            name if name.ends_with("_description") => {
                let text = field.text().await.unwrap_or_default();
                let key = name.strip_suffix("_description").unwrap_or_default();
                form.images.entry(key.to_string()).or_default().description =
                    text.replace("\r\n", "\n");
            }
            name if name.ends_with("_location") => {
                let text = field.text().await.unwrap_or_default();
                let key = name.strip_suffix("_location").unwrap_or_default();
                match serde_json::from_str::<Location>(&text) {
                    Ok(location) => {
                        let metadata = form.images.entry(key.to_string()).or_default();
                        metadata.location = location.geocoding_as_string();
                        metadata.coordinates =
                            format!("{},{}", location.latitude, location.longitude);
                    }
                    Err(err) => error!("Failed to parse location JSON: {}", err),
                }
            }
            "filepond" => {
                let path = field.text().await.unwrap_or_default();
                let file_name = path.split('/').last().unwrap_or_default().to_string();

                let im = form.images.entry(file_name.to_string()).or_default();
                im.file_name = file_name.clone();
                im.image_url = format!("{DEFAULT_IMAGE_URL}/{}", path);
            }
            _ => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    format!("Unexpected field: {}", name),
                ));
            }
        }
    }

    if token.is_empty() || token != std::env::var("TOKEN").unwrap_or_default() {
        return Err((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()));
    }

    let github_token = std::env::var("GITHUB_TOKEN").map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "GITHUB_TOKEN not set".to_string(),
        )
    })?;

    let repository = git::clone_repository(&github_token).await.map_err(|err| {
        error!("Failed to clone repository: {}", err);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to clone repository".to_string(),
        )
    })?;

    let mut keys: Vec<_> = form.images.keys().cloned().collect();
    keys.sort_by_key(|k| k.to_lowercase());
    let first_image = keys.first().cloned().unwrap_or_default();

    if let Some(first) = form.images.get(&first_image) {
        form.main = first.clone();
        form.main.file_name = first_image.clone();
    } else {
        error!("No main image specified or found");
        return Err((
            StatusCode::BAD_REQUEST,
            "No main image specified or found".to_string(),
        ));
    }

    if let Err(err) = form.validate() {
        error!("Form validation failed: {}", err);
        return Err((
            StatusCode::BAD_REQUEST,
            "Form validation failed".to_string(),
        ));
    }

    info!(
        "Title: {}, Categories: {}, Strava: {}, Date: {}, Main: {:?}, Images: {:?}",
        form.title, form.categories, form.strava, form.date, form.main, form.images,
    );
    let safe_file_name = tera::create_post(&form).map_err(|err| {
        error!("Failed to create post: {}", err);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to create post".to_string(),
        )
    })?;

    let date = NaiveDate::parse_from_str(&form.date, "%Y-%m-%d").map_err(|err| {
        error!("Failed to parse date {}: {}", form.date, err);
        (StatusCode::BAD_REQUEST, "Invalid date format".to_string())
    })?;

    let file_in_git_dir = format!("_posts/{}-{}.md", form.date, safe_file_name);

    git::commit_and_push(repository, &github_token, &file_in_git_dir, &form.title)
        .await
        .map_err(|err| {
            error!("Failed to commit and push: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to commit and push".to_string(),
            )
        })?;

    let post_url = format!(
        "https://kyrremann.no/plog/{}/{:02}/{}",
        date.year(),
        date.month(),
        safe_file_name
    );
    info!("Post URL: {}", post_url);

    brevo::post_campaign(
        form.title.clone(),
        form.main.description.clone(),
        form.main.image_url.clone(),
        post_url.clone(),
    )
    .await
    .map_err(|err| {
        error!("Failed to post campaign: {}", err);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to post campaign".to_string(),
        )
    })?;

    Ok(Html(
        "Form and multipart data processed successfully!".to_string(),
    ))
}
