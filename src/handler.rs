mod brevo;
mod image;
mod tera;

use crate::tera::{ImageMetadata, UploadForm};
use axum::extract::Multipart;
use axum::response::Html;
use chrono::{Datelike, Local, NaiveDate};
use log::{error, info};
use serde::Deserialize;
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

pub async fn upload(mut multipart: Multipart) -> Html<String> {
    let mut first_image: String = String::new();

    let mut form = UploadForm {
        ..Default::default()
    };

    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
        info!("Processing field: {}", name);

        if name == "title" {
            let text = field.text().await.unwrap();
            form.title = text.clone();
        } else if name == "strava" {
            let text = field.text().await.unwrap();
            form.strava = text.clone()
        } else if name == "date" {
            let text = field.text().await.unwrap();
            form.date = text.clone()
        } else if name == "categories" {
            let text = field.text().await.unwrap();
            form.categories = text.clone();
        } else if name.ends_with("_alt_text") {
            let text = field.text().await.unwrap();
            let key = name.split("_alt_text").next().unwrap();

            if let Some(value) = form.images.get_mut(key) {
                value.alt_text = text.clone();
            } else {
                form.images.insert(
                    key.to_string(),
                    ImageMetadata {
                        alt_text: text.clone(),
                        ..Default::default()
                    },
                );
            }
        } else if name.ends_with("_caption") {
            let text = field.text().await.unwrap();
            let key = name.split("_caption").next().unwrap();

            if let Some(value) = form.images.get_mut(key) {
                value.caption = text.clone();
            } else {
                form.images.insert(
                    key.to_string(),
                    ImageMetadata {
                        caption: text.clone(),
                        ..Default::default()
                    },
                );
            }
        } else if name.ends_with("_description") {
            let text = field.text().await.unwrap();
            let key = name.split("_description").next().unwrap();

            if let Some(value) = form.images.get_mut(key) {
                value.description = text.clone();
            } else {
                form.images.insert(
                    key.to_string(),
                    ImageMetadata {
                        description: text.clone(),
                        ..Default::default()
                    },
                );
            }
        } else if name.ends_with("_location") {
            let text = field.text().await.unwrap();
            let key = name.split("_location").next().unwrap();

            let location: Location = match serde_json::from_str(&text) {
                Ok(loc) => loc,
                Err(err) => {
                    error!("Failed to parse location JSON: {}", err);
                    continue;
                }
            };

            if let Some(value) = form.images.get_mut(key) {
                value.location = location.geocoding_as_string();
                value.coordinates = format!("{},{}", location.latitude, location.longitude);
            } else {
                form.images.insert(
                    key.to_string(),
                    ImageMetadata {
                        location: location.geocoding_as_string(),
                        coordinates: format!("{},{}", location.latitude, location.longitude),
                        ..Default::default()
                    },
                );
            }
        } else if name == "filepond" {
            let file_name = field.file_name().unwrap().to_string();
            let content_type = field.content_type().unwrap_or("image/jpeg").to_string();
            let data = field.bytes().await.unwrap();

            let mut file = File::create(format!("images/{file_name}")).unwrap();
            file.write_all(&data).unwrap();
            info!(
                "Size of {file_name} ({content_type}) is {} bytes",
                data.len()
            );

            let date_from_name = file_name.split("_").next().unwrap();
            let date_only =
                NaiveDate::parse_from_str(date_from_name, "%Y%m%d").unwrap_or_else(|_| {
                    error!("Failed to parse date from file name: {}", date_from_name);
                    NaiveDate::parse_from_str(&form.date, "%Y-%m-%d")
                        .unwrap_or(Local::now().date_naive())
                });
            let path = format!(
                "images/{}/{:02}/{}",
                date_only.year(),
                date_only.month(),
                file_name
            );

            log::info!("Uploading image: {}", file_name);
            if let Err(err) = image::upload_image(&path, &content_type, data.to_vec()).await {
                error!("Failed to upload image {}: {}", path, err);
                return Html("Image upload failed".to_string());
            }

            if let Some(metadata) = form.images.get_mut(&file_name) {
                metadata.image_url = path.clone();
            } else {
                form.images.insert(
                    file_name.clone(),
                    ImageMetadata {
                        image_url: format!("{DEFAULT_IMAGE_URL}/{path}"),
                        ..Default::default()
                    },
                );
            }

            if first_image.is_empty() {
                first_image = file_name.clone();
            }

            println!("Base64 image uploaded: {}", file_name);
        } else {
            error!("Unknown field: {}", name);
        }
    }

    if let Some(first) = form.images.get(&first_image) {
        form.main = first.clone();
    } else {
        error!("No main image specified or found");
        return Html("No images supplied".to_string());
    }

    if let Err(err) = form.validate() {
        error!("Form validation failed: {}", err);
        return Html(format!("Form validation failed: {}", err));
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

    let post_file_name = match tera::create_post(&form) {
        Ok(name) => name,
        Err(err) => {
            error!("Failed to create post: {}", err);
            return Html("Post creation failed".to_string());
        }
    };

    let date = match NaiveDate::parse_from_str(&form.date, "%Y-%m-%d") {
        Ok(parsed_date) => parsed_date,
        Err(err) => {
            error!("Failed to parse date {}: {}", form.date, err);
            return Html("Invalid date format".to_string());
        }
    };

    let post_url = format!(
        "https://kyrremann.no/plog/post/{}/{}/{}",
        date.year(),
        date.month(),
        post_file_name
    );

    info!("Post URL: {}", post_url);

    brevo::post_campaign(
        form.title.clone(),
        form.main.description.clone(),
        form.main.image_url.clone(),
        post_url.clone(),
    )
    .await;

    Html("Form and multipart data processed successfully!".to_string())
}
