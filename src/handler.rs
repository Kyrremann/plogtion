mod brevo;
mod tera;

use crate::tera::{ImageMetadata, UploadForm};
use axum::extract::Multipart;
use axum::response::Html;
use chrono::{Datelike, NaiveDate};
use image::codecs::jpeg::JpegEncoder;
use image::{GenericImageView, ImageReader};
use s3::creds::Credentials;
use s3::{Bucket, Region};
use std::fs::File;
use std::io::{BufWriter, Cursor, Write};

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

            let resized_image = resize_with_quality(&file_name, &data).await;

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

            upload_image(&path, &content_type, resized_image).await;
            if let Some(metadata) = form.images.get_mut(&file_name) {
                metadata.image = path.clone();
            } else {
                form.images.insert(
                    file_name.clone(),
                    tera::ImageMetadata {
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

async fn upload_image(path: &str, content_type: &str, resized_image: BufWriter<Vec<u8>>) {
    let bucket_name = "kyrremann-plog";
    let region_name = "nl-ams".to_string();
    let endpoint = "https://s3.nl-ams.scw.cloud".to_string();
    let region = Region::Custom {
        region: region_name,
        endpoint,
    };
    let credentials =
        Credentials::new(None, None, None, None, None).expect("Failed to create credentials");
    let bucket = Bucket::new(bucket_name, region, credentials).expect("Failed to create bucket");

    let response = bucket
        .put_object_with_content_type(path, &resized_image.into_inner().unwrap(), content_type)
        .await;

    match response {
        Ok(_) => println!("Uploaded {}", path),
        Err(e) => eprintln!("Failed to upload {} to S3: {}", path, e),
    }
}

async fn resize_with_quality(file_name: &str, bytes: &[u8]) -> BufWriter<Vec<u8>> {
    let src_image = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .unwrap()
        .decode()
        .unwrap();

    // // Correct orientation based on EXIF metadata
    // if let Ok(exif_reader) = exif::Reader::new().read_from_container(&mut Cursor::new(bytes)) {
    //     if let Some(orientation) = exif_reader.get_field(exif::Tag::Orientation, exif::In::PRIMARY)
    //     {
    //         src_image = match orientation.value.get_uint(0) {
    //             Some(3) => src_image.rotate180(),
    //             Some(6) => src_image.rotate90(),
    //             Some(8) => src_image.rotate270(),
    //             _ => src_image, // No rotation needed
    //         };
    //     }
    // }

    // Resize logic
    let (src_width, src_height) = src_image.dimensions();
    if src_width <= 1440 && src_height <= 1440 {
        println!(
            "Image {} is already smaller than 1440 pixels on the longest side, skipping resize.",
            file_name
        );
        return BufWriter::new(bytes.to_vec());
    }

    let scale_factor = if src_width > src_height {
        1440.0 / src_width as f32
    } else {
        1440.0 / src_height as f32
    };

    let dst_width = (src_width as f32 * scale_factor) as u32;
    let dst_height = (src_height as f32 * scale_factor) as u32;
    println!(
        "Resizing image {} from {}x{} to {}x{}",
        file_name, src_width, src_height, dst_width, dst_height
    );

    let final_image =
        src_image.resize(dst_width, dst_height, image::imageops::FilterType::Lanczos3);

    let mut buf = BufWriter::new(Vec::new());
    let encoder = JpegEncoder::new_with_quality(&mut buf, 75);
    final_image.write_with_encoder(encoder).unwrap();

    buf
}
