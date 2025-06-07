mod brevo;
mod image;
mod tera;

use crate::tera::{ImageMetadata, UploadForm};
use axum::extract::Multipart;
use axum::response::Html;
use chrono::{Datelike, NaiveDate};
use log::{error, info};
use std::{fs::File, io::Write};

const DEFAULT_IMAGE_URL: &str = "https://kyrremann-plog.s3.nl-ams.scw.cloud";

pub async fn upload(mut multipart: Multipart) -> Html<String> {
    let mut form = UploadForm {
        ..Default::default()
    };

    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();

        if name != "images" {
            let text = field.text().await.unwrap();

            if name == "title" {
                form.title = text.clone();
            } else if name == "description" {
                form.description = text.clone()
            } else if name == "date" {
                form.date = text.clone()
            } else if name == "categories" {
                form.categories = text.clone();
            } else if name.ends_with("_altText") {
                let key = name.split("_altText").next().unwrap();

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
            } else if name.ends_with("_description") {
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
                let key = name.split("_location").next().unwrap();

                if let Some(value) = form.images.get_mut(key) {
                    value.location = text.clone();
                } else {
                    form.images.insert(
                        key.to_string(),
                        ImageMetadata {
                            location: text.clone(),
                            ..Default::default()
                        },
                    );
                }
            }
            println!("Field: {} = {}", name, &text);
        } else {
            let file_name = field.file_name().unwrap().to_string();
            let content_type = field.content_type().unwrap().to_string();
            let data = field.bytes().await.unwrap();

            let mut file = File::create(format!("images/{file_name}")).unwrap();
            file.write_all(&data).unwrap();

            println!(
                "Size of {file_name} ({content_type}) is {} bytes",
                data.len()
            );

            let resized_image = image::resize_with_quality(&file_name, &data).await.unwrap();

            let date_from_name = file_name.split("_").next().unwrap();
            let date_only = NaiveDate::parse_from_str(date_from_name, "%Y%m%d").unwrap();
            let path = format!(
                "images/{}/{:02}/{}",
                date_only.year(),
                date_only.month(),
                file_name
            );

            if form.main_image.is_empty() {
                form.main_image = format!("{DEFAULT_IMAGE_URL}/{path}");
            }

            if let Err(err) = image::upload_image(&path, &content_type, resized_image).await {
                error!("Failed to upload image {}: {}", path, err);
                return Html("Image upload failed".to_string());
            }

            if let Some(metadata) = form.images.get_mut(&file_name) {
                metadata.image = path.clone();
            } else {
                form.images.insert(
                    file_name.clone(),
                    ImageMetadata {
                        image: format!("{DEFAULT_IMAGE_URL}/{path}"),
                        ..Default::default()
                    },
                );
            }
        }
    }

    info!(
        "Title: {}, Categories: {}, Description: {}, Date: {}, Main: {}, Images: {}",
        form.title,
        form.categories,
        form.description,
        form.date,
        form.main_image,
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
        form.description.clone(),
        form.main_image.clone(),
        post_url.clone(),
    )
    .await;

    Html("Form and multipart data processed successfully!".to_string())
}
