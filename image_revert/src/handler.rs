use axum::body::{Body, to_bytes};
use axum::http::Request;
use log::{error, info};
use s3::creds::Credentials;
use s3::{Bucket, Region};
use std::str;

pub async fn handle(req: Request<Body>) {
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
    let body_bytes = to_bytes(req.into_body(), 65536) // 64 KB limit
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
}
