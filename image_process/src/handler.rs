use axum::body::Body;
use axum::extract::{FromRequest, Multipart};
use axum::http::Request;
use axum::http::{self, StatusCode};
use axum::response::Response;
use chrono::{Datelike, Local, NaiveDate};
use log::{error, info};
use s3::creds::Credentials;
use s3::{Bucket, Region};

pub fn with_permissive_cors(origin: String) -> http::response::Builder {
    let response = Response::builder()
        .header(
            "Access-Control-Allow-Headers",
            "content-type, x-auth-token, authorization, origin, accept",
        )
        .header("Access-Control-Allow-Methods", "OPTIONS, POST");

    if origin == "http://localhost:4000" || origin == "https://kyrremann.no" {
        return response.header("Access-Control-Allow-Origin", origin);
    }

    return response;
}

pub async fn handle(request: Request<Body>) -> Response<Body> {
    if let Err(e) = env_logger::try_init() {
        error!("Failed to initialize logger: {}", e);
    }

    let origin = match request.headers().get("origin").and_then(|v| v.to_str().ok()) {
        Some(value) => value.to_string(),
        None => {
            error!("Origin header missing or invalid");
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from("Bad Request"))
                .unwrap();
        }
    };
    info!("Request from origin: {}", origin);
    let response = with_permissive_cors(origin.clone());

    // Check if this is an OPTIONS request
    if request.method() == http::Method::OPTIONS {
        return response.status(StatusCode::OK).body(Body::empty()).unwrap();
    }

    let token = match std::env::var("TOKEN") {
        Ok(value) => value,
        Err(_) => {
            error!("TOKEN not set");
            return response
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Internal Server Error"))
                .unwrap();
        }
    };

    match request.headers().get("x-auth-token").and_then(|v| v.to_str().ok()) {
        Some(header_token) if header_token == token => {}
        _ => {
            error!("Invalid or missing x-auth-token header");
            return response
                .status(StatusCode::UNAUTHORIZED)
                .body(Body::from("Unauthorized"))
                .unwrap();
        }
    }

    // For POST requests, extract multipart data
    let (parts, body) = request.into_parts();
    let body_stream = axum::body::Body::new(body);

    // Reconstruct the request for multipart extraction
    let request = Request::from_parts(parts, body_stream);

    // Use Axum's built-in multipart extractor
    match Multipart::from_request(request, &()).await {
        Ok(multipart) => process_multipart(with_permissive_cors(origin), multipart).await,
        Err(err) => {
            error!("Failed to extract multipart: {}", err);
            with_permissive_cors(origin)
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from("Failed to extract multipart data"))
                .unwrap()
        }
    }
}

// Move your existing multipart processing logic to this function
async fn process_multipart(
    response: http::response::Builder,
    mut multipart: Multipart,
) -> Response<Body> {
    info!("Payload received...");
    let mut path = "No path found".to_string();

    while let Some(field_result) = multipart.next_field().await.transpose() {
        match field_result {
            Ok(field) => {
                let name = field.name().unwrap_or_default().to_string();

                match name.as_str() {
                    "filepond" => {
                        let file_name = field.file_name().unwrap_or_default().to_string();
                        if file_name.is_empty() {
                            continue;
                        }

                        info!("Processing: {}", file_name);
                        let content_type = field.content_type().unwrap_or("image/jpeg").to_string();
                        let data = field.bytes().await.unwrap_or_default();

                        let date_from_name = file_name.split('_').next().unwrap_or_default();
                        let date_only = NaiveDate::parse_from_str(date_from_name, "%Y%m%d")
                            .unwrap_or_else(|_| {
                                error!(
                                    "Failed to parse date from {}: {}",
                                    file_name, date_from_name
                                );
                                Local::now().date_naive()
                            });
                        path = format!(
                            "images/{}/{:02}/{}",
                            date_only.year(),
                            date_only.month(),
                            file_name
                        );

                        if let Err(e) =
                            upload_image(path.as_str(), content_type.as_str(), data.to_vec()).await
                        {
                            error!("Failed to upload image: {}", e);
                            return response
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .body(Body::from(format!("Failed to upload image: {}", e)))
                                .unwrap();
                        }
                    }
                    _ => {
                        error!("Unexpected field name: {}", name);
                        return response
                            .status(StatusCode::BAD_REQUEST)
                            .body(Body::from("Unexpected field name"))
                            .unwrap();
                    }
                }
            }
            Err(err) => {
                error!("Failed to read multipart field: {}", err);
                return response
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::from("Failed to read multipart field"))
                    .unwrap();
            }
        }
    }

    response
        .status(StatusCode::OK)
        .header("Content-Type", "text/plain")
        .body(Body::from(path))
        .unwrap()
}

async fn upload_image(path: &str, content_type: &str, image: Vec<u8>) -> Result<(), String> {
    let bucket_name = "kyrremann-plog";
    let region_name = "nl-ams".to_string();
    let endpoint = "https://s3.nl-ams.scw.cloud".to_string();
    let region = Region::Custom {
        region: region_name,
        endpoint,
    };
    let credentials = Credentials::new(None, None, None, None, None)
        .map_err(|e| format!("Failed to create credentials: {}", e))?;
    let bucket = Bucket::new(bucket_name, region, credentials)
        .map_err(|e| format!("Failed to create bucket: {}", e))?;

    bucket
        .put_object_with_content_type(path, &image, content_type)
        .await
        .map_err(|e| {
            error!("Failed to upload {} to S3: {}", path, e);
            format!("Failed to upload {} to S3: {}", path, e)
        })?;

    info!("Uploaded {} successfully", path);
    Ok(())
}
