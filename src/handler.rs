use std::io::Cursor;

use axum::{response::Html, Form};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use s3::creds::Credentials;
use s3::{Bucket, Region};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct UploadForm {
    title: String,
    description: String,
    date: String,
    #[serde(rename = "resizedImages")]
    resized_images: String,
}

#[derive(Deserialize)]
struct ResizedImage {
    filename: String,
    #[serde(rename = "dataUrl")]
    data_url: String,
}

pub async fn upload(Form(form): Form<UploadForm>) -> Html<String> {
    // Parse the JSON string of resized images
    let resized_images: Vec<ResizedImage> =
        serde_json::from_str(&form.resized_images).expect("Failed to parse resized images");

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

    for (_, resized_image) in resized_images.iter().enumerate() {
        // Decode the base64 data URL
        let base64_data = resized_image
            .data_url
            .split(',')
            .nth(1)
            .expect("Invalid data URL");
        let image_data = STANDARD
            .decode(base64_data)
            .expect("Failed to decode base64 data");

        // Load the image from memory
        let img = image::load_from_memory(&image_data).expect("Failed to load image from memory");
        let mut bytes: Vec<u8> = Vec::new();
        img.write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Jpeg)
            .expect("Failed to write image to buffer");
        let response = bucket
            .put_object_with_content_type(
                format!("images/{}", &resized_image.filename),
                &bytes,
                "image/jpeg",
            )
            .await;
        match response {
            Ok(_) => println!("Uploaded {} to S3", resized_image.filename),
            Err(e) => eprintln!("Failed to upload {} to S3: {}", resized_image.filename, e),
        }
    }

    Html(format!(
        "Title: {}, Description: {}, Date: {}, Images: {}",
        form.title,
        form.description,
        form.date,
        resized_images.len()
    ))
}
