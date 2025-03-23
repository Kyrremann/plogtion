use axum::{
    response::Html,
    routing::{get, post},
    Router,
};
use plogtion::upload;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(show_index))
        .route("/post", post(upload));

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
