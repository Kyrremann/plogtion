use log::{error, info};
use s3::creds::Credentials;
use s3::{Bucket, Region};

pub(crate) async fn upload_image(
    path: &str,
    content_type: &str,
    image: Vec<u8>,
) -> Result<(), String> {
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
