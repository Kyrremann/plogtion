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

    let origin = match request
        .headers()
        .get("origin")
        .and_then(|v| v.to_str().ok())
    {
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

    match request
        .headers()
        .get("x-auth-token")
        .and_then(|v| v.to_str().ok())
    {
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
                        let content_type = field
                            .content_type()
                            .unwrap_or_else(|| {
                                if file_name.to_lowercase().ends_with(".mp4") {
                                    "video/mp4"
                                } else {
                                    "image/jpeg"
                                }
                            })
                            .to_string();
                        let data = field.bytes().await.unwrap_or_default();

                        let date_only = parse_date_from_filename(&file_name);
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

fn parse_date_from_filename(file_name: &str) -> NaiveDate {
    let stem = file_name
        .rfind('.')
        .map(|i| &file_name[..i])
        .unwrap_or(file_name);
    let segments: Vec<&str> = stem.split('_').collect();

    // Strategy 1: first segment is exactly 8 digits (e.g. 20250815_182919.jpg)
    if let Some(seg) = segments.first() {
        if seg.len() == 8 {
            if let Ok(date) = NaiveDate::parse_from_str(seg, "%Y%m%d") {
                return date;
            }
        }
    }

    // Strategy 2: scan all segments for the first one whose leading 8 chars parse as %Y%m%d
    // Covers IMG_*, VID_*, PRO_VID_*, DJI_* etc. regardless of how many prefix segments there are
    for seg in &segments {
        if seg.len() >= 8 {
            if let Ok(date) = NaiveDate::parse_from_str(&seg[..8], "%Y%m%d") {
                return date;
            }
        }
    }

    error!("Failed to parse date from filename: {}", file_name);
    Local::now().date_naive()
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn test_plain_date_prefix() {
        // 20250815_182919.jpg
        assert_eq!(
            parse_date_from_filename("20250815_182919.jpg"),
            date(2025, 8, 15)
        );
    }

    #[test]
    fn test_img_prefix() {
        // IMG_20250915_055422_00_266.jpg
        assert_eq!(
            parse_date_from_filename("IMG_20250915_055422_00_266.jpg"),
            date(2025, 9, 15)
        );
    }

    #[test]
    fn test_img_prefix_pureshot() {
        // IMG_20250915_055422_00_266_PureShot.jpg
        assert_eq!(
            parse_date_from_filename("IMG_20250915_055422_00_266_PureShot.jpg"),
            date(2025, 9, 15)
        );
    }

    #[test]
    fn test_dji_prefix() {
        // DJI_20250910153138_0155_D.JPG
        assert_eq!(
            parse_date_from_filename("DJI_20250910153138_0155_D.JPG"),
            date(2025, 9, 10)
        );
    }

    #[test]
    fn test_plain_date_mp4() {
        // 20260420_205613.mp4
        assert_eq!(
            parse_date_from_filename("20260420_205613.mp4"),
            date(2026, 4, 20)
        );
    }

    #[test]
    fn test_vid_prefix() {
        // VID_20250824_115547_00_092.mp4
        assert_eq!(
            parse_date_from_filename("VID_20250824_115547_00_092.mp4"),
            date(2025, 8, 24)
        );
    }

    #[test]
    fn test_pro_vid_prefix() {
        // PRO_VID_20250903_152413_00_138.mp4
        assert_eq!(
            parse_date_from_filename("PRO_VID_20250903_152413_00_138.mp4"),
            date(2025, 9, 3)
        );
    }

    #[test]
    fn test_dji_mp4_prefix() {
        // DJI_20250910110913_0137_D.MP4
        assert_eq!(
            parse_date_from_filename("DJI_20250910110913_0137_D.MP4"),
            date(2025, 9, 10)
        );
    }

    #[test]
    fn test_unknown_falls_back_to_today() {
        let result = parse_date_from_filename("unknown.jpg");
        assert_eq!(result, Local::now().date_naive());
    }
}
