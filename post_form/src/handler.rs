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
    pub town: String,
    #[serde(default)]
    pub city: String,
    #[serde(default)]
    pub municipality: String,
    #[serde(default)]
    pub province: String,
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
        if !self.geocoding.town.is_empty() {
            parts.push(self.geocoding.town.clone());
        }
        if !self.geocoding.city.is_empty() {
            parts.push(self.geocoding.city.clone());
        }
        if !self.geocoding.municipality.is_empty() {
            parts.push(self.geocoding.municipality.clone());
        }
        if !self.geocoding.province.is_empty() {
            parts.push(self.geocoding.province.clone());
        }
        if !self.geocoding.country.is_empty() {
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
        error!("Failed to read multipart field: {err}");
        (
            StatusCode::BAD_REQUEST,
            "Failed to read multipart field".to_string(),
        )
    })? {
        let name = field.name().unwrap_or_default().to_string();
        info!("Processing field: {name}");

        let value = field.text().await.unwrap_or_default();

        match name.as_str() {
            "token" => token = value,
            "title" => form.title = value.trim().to_string(),
            "strava" => form.strava = value,
            "date" => form.date = value,
            "categories" => form.categories = value,
            "feature_image" => {
                let file_name = value;
                form.feature.file_name = file_name.clone();
            }
            name if name.ends_with("_alt_text") => {
                let text = value.trim();
                let key = name.strip_suffix("_alt_text").unwrap_or_default();
                form.images.entry(key.to_string()).or_default().alt_text = text.to_string();
            }
            name if name.ends_with("_caption") => {
                let text = value.trim();
                let key = name.strip_suffix("_caption").unwrap_or_default();
                form.images.entry(key.to_string()).or_default().caption = text.to_string();
            }
            name if name.ends_with("_description") => {
                let text = value;
                let key = name.strip_suffix("_description").unwrap_or_default();
                form.images.entry(key.to_string()).or_default().description =
                    text.replace("\r\n", "\n");
            }
            name if name.ends_with("_location") => {
                let text = value;
                let key = name.strip_suffix("_location").unwrap_or_default();
                match serde_json::from_str::<Location>(&text) {
                    Ok(location) => {
                        let metadata = form.images.entry(key.to_string()).or_default();
                        metadata.location = location.geocoding_as_string();
                        metadata.coordinates =
                            format!("{},{}", location.latitude, location.longitude);
                    }
                    Err(err) => error!("Failed to parse location JSON: {err}"),
                }
            }
            "filepond" => {
                let path = value;
                let file_name = path.split('/').next_back().unwrap_or_default().to_string();

                let im = form.images.entry(file_name.to_string()).or_default();
                im.file_name = file_name.clone();
                im.image_url = format!("{DEFAULT_IMAGE_URL}/{path}");
            }
            _ => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    format!("Unexpected field: {name}"),
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
        error!("Failed to clone repository: {err}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to clone repository".to_string(),
        )
    })?;

    if let Some(image) = form.images.get(&form.feature.file_name) {
        form.feature.image_url = image.image_url.clone();
        form.feature.description = image.description.clone(); // For the email campaign
    } else {
        info!("No featured image specified, selecting the first available image");
        let mut keys: Vec<_> = form.images.keys().cloned().collect();
        keys.sort_by_key(|k| k.to_lowercase());
        let featured_image_key = keys.first().cloned().unwrap_or_default();

        let image = form.images.get(&featured_image_key).unwrap();
        form.feature = image.clone();
        form.feature.file_name = featured_image_key.clone();
    }

    if let Err(err) = form.validate() {
        // at this point we're safe to say form isn't malformed, right?
        let serialized = serde_json::to_string(&form).unwrap();
        error!("Form validation failed: {err}");
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Form validation failed: {err}\n\n{serialized}"),
        ));
    }

    info!(
        "Title: {}, Categories: {}, Strava: {}, Date: {}, Feature: {:?}, Images: {:?}",
        form.title, form.categories, form.strava, form.date, form.feature, form.images,
    );

    let safe_file_name = tera::create_post(&form).map_err(|err| {
        error!("Failed to create post: {err}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to create post: {err}"),
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
            error!("Failed to commit and push: {err}");
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
    info!("Post URL: {post_url}");

    brevo::post_campaign(
        form.title.clone(),
        form.feature.description.clone(),
        form.feature.image_url.clone(),
        post_url.clone(),
    )
    .await
    .map_err(|err| {
        error!("Failed to post campaign: {err}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to post campaign".to_string(),
        )
    })?;

    Ok(Html(
        "Form and multipart data processed successfully!".to_string(),
    ))
}
