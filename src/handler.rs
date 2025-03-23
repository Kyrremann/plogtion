use axum::{response::Html, Form};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
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

        // Save the image to a file
        let path = format!("images/{}", resized_image.filename);
        img.save(path).expect("Failed to save image to file");
    }

    Html(format!(
        "Title: {}, Description: {}, Date: {}, Images: {}",
        form.title,
        form.description,
        form.date,
        resized_images.len()
    ))
}
