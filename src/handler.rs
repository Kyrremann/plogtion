mod brevo;
mod image;
mod tera;

use crate::tera::{ImageMetadata, UploadForm};
use axum::extract::Multipart;
use axum::response::Html;
use chrono::{Datelike, NaiveDate};
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

            let resized_image = image::resize_with_quality(&file_name, &data).await;

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

            image::upload_image(&path, &content_type, resized_image).await;
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

    println!(
        "Title: {}, Categories: {}, Description: {}, Date: {}, Main: {}, Images: {}",
        form.title,
        form.categories,
        form.description,
        form.date,
        form.main_image,
        form.images.len(),
    );

    let post_file_name = tera::create_post(&form);
    let date = NaiveDate::parse_from_str(&form.date, "%Y-%m-%d").unwrap();

    println!(
        "Post URL: https://kyrremann.no/plog/post/{}/{}/{}",
        date.year(),
        date.month(),
        post_file_name
    );

    // brevo::post_campaign(
    //     form.title,
    //     form.description,
    //     form.main_image,
    //     format!(
    //         "https://kyrremann.no/plog/post/{}/{}/{}",
    //         date.year(),
    //         date.month(),
    //         post_file_name
    //     ),
    // )
    // .await;

    Html("Form and multipart data processed successfully!".to_string())
}
