use axum::body::{Body, to_bytes};
use axum::http::{self, Request, StatusCode};
use axum::response::Response;
use log::{error, info};
use s3::creds::Credentials;
use s3::{Bucket, Region};
use std::str;

pub fn with_permissive_cors(origin: String) -> http::response::Builder {
    let response = Response::builder()
        .header(
            "Access-Control-Allow-Headers",
            "content-type, x-auth-token, authorization, origin, accept",
        )
        .header("Access-Control-Allow-Methods", "OPTIONS, DELETE");

    if origin == "http://localhost:4000" || origin == "https://kyrremann.no" {
        return response.header("Access-Control-Allow-Origin", origin);
    }

    return response;
}

pub async fn handle(request: Request<Body>) -> Response<Body> {
    env_logger::try_init().unwrap_or_else(|_| {
        eprintln!("Failed to initialize logger, using default settings");
    });

    let origin = request
        .headers()
        .get("origin")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    // Check if this is an OPTIONS request
    if request.method() == http::Method::OPTIONS {
        return with_permissive_cors(origin.clone())
            .status(StatusCode::OK)
            .body(Body::empty())
            .unwrap();
    }

    let token = std::env::var("TOKEN")
        .map_err(|_| "TOKEN not set".to_string())
        .unwrap();

    request
        .headers()
        .get("x-auth-token")
        .and_then(|v| v.to_str().ok())
        .filter(|&header_token| header_token == token)
        .ok_or_else(|| {
            error!("Invalid or missing x-auth-token header");
            with_permissive_cors(origin.clone())
                .status(StatusCode::UNAUTHORIZED)
                .body(Body::from("Unauthorized"))
                .unwrap();
        })
        .unwrap();

    let bucket_name = "kyrremann-plog";
    let region_name = "nl-ams".to_string();
    let endpoint = "https://s3.nl-ams.scw.cloud".to_string();
    let region = Region::Custom {
        region: region_name,
        endpoint,
    };

    let credentials = Credentials::new(None, None, None, None, None)
        .map_err(|e| {
            error!("Failed to create credentials: {}", e);
        })
        .unwrap();
    let bucket = Bucket::new(bucket_name, region, credentials)
        .map_err(|e| {
            error!("Failed to create bucket: {}", e);
        })
        .unwrap();

    // Extract the body as a string with a limit
    let body_bytes = to_bytes(request.into_body(), 65536) // 64 KB limit
        .await
        .map_err(|e| {
            error!("Failed to read request body: {}", e);
        })
        .unwrap();

    let body_str = str::from_utf8(&body_bytes)
        .map_err(|e| {
            error!("Failed to convert body to string: {}", e);
        })
        .unwrap();

    info!("Request body: {}", body_str);

    let path = body_str.trim();

    let _ = bucket.delete_object(path).await.map_err(|e| {
        error!("Failed to delete {} from S3: {}", path, e);
    });

    info!("Deleted {} successfully", path);

    with_permissive_cors(origin.clone())
        .status(StatusCode::OK)
        .body(Body::empty())
        .unwrap()
}
