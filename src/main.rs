use axum::{
    Router,
    extract::{DefaultBodyLimit, Multipart},
    response::{Html, IntoResponse},
    routing::{get, post},
};
use plogtion::upload;
use structured_logger::{Builder, async_json::new_writer};

#[tokio::main]
async fn main() {
    Builder::with_level("info")
        .with_target_writer("*", new_writer(tokio::io::stdout()))
        .init();

    let app = Router::new()
        .route("/", get(show_index))
        .route("/post", post(upload_handler))
        .layer(DefaultBodyLimit::max(
            1024 * 1024 * 100, // 100 MB
        ));

    log::info!("Starting Plogtion server...");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn show_index() -> impl IntoResponse {
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

async fn upload_handler(multipart: Multipart) -> Html<String> {
    match upload(multipart).await {
        Ok(_) => Html(
            r#"<!doctype html>
<html lang="en">
  <head>
    <title>Plogtion: Success</title>
  </head>
  <body>
    <h1>Upload Successful</h1>
    <p>Your form and multipart data were processed successfully!</p>
    <a href="/">Go back to the homepage</a>
  </body>
</html>
"#
            .to_string(),
        ),
        Err((status_code, message)) => Html(format!(
            r#"<!doctype html>
<html lang="en">
  <head>
    <title>Plogtion: Failed</title>
  </head>
  <body>
    <h1>Upload Failed</h1>
    <p>Error ({}): {}</p>
    <a href="/">Go back to the homepage</a>
  </body>
</html>
"#,
            status_code, message
        )),
    }
}
