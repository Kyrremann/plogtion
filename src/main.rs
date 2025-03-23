use axum::{
    extract::Form,
    response::Html,
    routing::{get, post},
    Router,
};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::Deserialize;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(show_index))
        .route("/post", post(handle_upload));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn show_index() -> Html<&'static str> {
    Html(
        r#"<!doctype html>
<html lang="en">
  <head>
    <title>Plogtion</title>
  </head>
  <body>
    <h1>Plogtion</h1>
      <p>You should probably go to <a href="https://kyrremann.no/plog">kyrremann.no/plog</a></p>
  </body>
</html>
"#,
    )
}

#[derive(Deserialize)]
struct UploadForm {
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

async fn handle_upload(Form(form): Form<UploadForm>) -> Html<String> {
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
