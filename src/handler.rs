mod brevo;
mod git;
mod image;
mod tera;

use crate::tera::UploadForm;
use axum::extract::Multipart;
use axum::http::StatusCode;
use axum::response::Html;
use chrono::{Datelike, Local, NaiveDate};
use log::{error, info};
use serde::Deserialize;
use std::path::Path;
use std::{fs::File, io::Write};

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

pub async fn upload(mut multipart: Multipart) -> Result<Html<String>, (StatusCode, String)> {
    let mut first_image: String = String::new();
    let mut form = UploadForm {
        ..Default::default()
    };

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
                form.images.entry(key.to_string()).or_default().description = text; // Todo: enable newlines text.replace("\r\n", "\n");
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
                let file_name = field.file_name().unwrap_or_default().to_string();
                let content_type = field.content_type().unwrap_or("image/jpeg").to_string();
                let data = field.bytes().await.unwrap_or_default();

                let local_path = format!("images/{}", file_name);
                if let Err(err) = save_file(&local_path, &data) {
                    error!("Failed to save file {}: {}", local_path, err);
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to save file".to_string(),
                    ));
                }

                let date_from_name = file_name.split('_').next().unwrap_or_default();
                let date_only =
                    NaiveDate::parse_from_str(date_from_name, "%Y%m%d").unwrap_or_else(|_| {
                        error!("Failed to parse date from file name: {}", date_from_name);
                        NaiveDate::parse_from_str(&form.date, "%Y-%m-%d")
                            .unwrap_or_else(|_| Local::now().date_naive())
                    });
                let path = format!(
                    "images/{}/{:02}/{}",
                    date_only.year(),
                    date_only.month(),
                    file_name
                );

                if let Err(err) = image::upload_image(&path, &content_type, data.to_vec()).await {
                    error!("Failed to upload image {}: {}", path, err);
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to upload image".to_string(),
                    ));
                }

                form.images.entry(file_name.clone()).or_default().image_url =
                    format!("{DEFAULT_IMAGE_URL}/{}", path);

                if first_image.is_empty() {
                    first_image = file_name;
                }
            }
            _ => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    format!("Unexpected field: {}", name),
                ));
            }
        }
    }

    if let Some(first) = form.images.get(&first_image) {
        form.main = first.clone();
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
        "Title: {}, Categories: {}, Strava: {}, Date: {}, Main.location: {}, Main.coordinates: {}, Images: {}",
        form.title,
        form.categories,
        form.strava,
        form.date,
        form.main.location,
        form.main.coordinates,
        form.images.len(),
    );
    let post_file_name = tera::create_post(&form).map_err(|err| {
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

    let post_url = format!(
        "https://kyrremann.no/plog/post/{}/{}/{}",
        date.year(),
        date.month(),
        post_file_name
    );

    info!("Post URL: {}", post_url);
    todo!("Implement git commit and push");
    // git::commit_and_push(&post_file_name, &form.title).map_err(|err| {
    //     error!("Failed to commit and push post: {}", err);
    //     return Err(StatusCode::INTERNAL_SERVER_ERROR);
    // })?;

    brevo::post_campaign(
        form.title.clone(),
        form.main.description.clone(),
        form.main.image_url.clone(),
        post_url.clone(),
    )
    .await;

    Ok(Html(
        "Form and multipart data processed successfully!".to_string(),
    ))
}

fn save_file(path: &str, data: &[u8]) -> Result<(), String> {
    let parent = Path::new(path).parent().ok_or("Invalid file path")?;
    std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create directories: {}", e))?;
    let mut file = File::create(path).map_err(|e| format!("Failed to create file: {}", e))?;
    file.write_all(data)
        .map_err(|e| format!("Failed to write to file: {}", e))
}
