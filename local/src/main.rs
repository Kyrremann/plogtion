use axum::{
    Router,
    body::Body,
    extract::Multipart,
    http::Request,
    response::{Html, IntoResponse, Response},
    routing::{delete, get, post},
};

#[tokio::main]
async fn main() {
    env_logger::init();

    let app = Router::new()
        .route("/", get(show_index))
        .route("/post", post(upload_handler))
        .route("/image", post(image_handler))
        .route("/image", delete(delete_image_handler))
    // .layer(DefaultBodyLimit::max(
    //     1024 * 1024 * 6, // 6 MB
    // ))
        ;

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
    match post_form::handle(multipart).await {
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
    <p>Error ({status_code}): {message}</p>
    <a href="/">Go back to the homepage</a>
  </body>
</html>
"#)),
    }
}

async fn image_handler(req: Request<Body>) -> Response<Body> {
    image_process::handle(req).await
}

async fn delete_image_handler(req: Request<Body>) {
    image_revert::handle(req).await;
}
