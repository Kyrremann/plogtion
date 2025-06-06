use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{GenericImageView, ImageReader};
use log::info;
use s3::creds::Credentials;
use s3::{Bucket, Region};
use std::io::{BufWriter, Cursor};

const MAX_DIMENSION: u32 = 1440;
const JPEG_QUALITY: u8 = 75;

pub(crate) async fn upload_image(
    path: &str,
    content_type: &str,
    resized_image: BufWriter<Vec<u8>>,
) {
    let bucket_name = "kyrremann-plog";
    let region_name = "nl-ams".to_string();
    let endpoint = "https://s3.nl-ams.scw.cloud".to_string();
    let region = Region::Custom {
        region: region_name,
        endpoint,
    };
    let credentials =
        Credentials::new(None, None, None, None, None).expect("Failed to create credentials");
    let bucket = Bucket::new(bucket_name, region, credentials).expect("Failed to create bucket");

    let response = bucket
        .put_object_with_content_type(path, &resized_image.into_inner().unwrap(), content_type)
        .await;

    match response {
        Ok(_) => println!("Uploaded {}", path),
        Err(e) => eprintln!("Failed to upload {} to S3: {}", path, e),
    }
}

pub(crate) async fn resize_with_quality(file_name: &str, bytes: &[u8]) -> BufWriter<Vec<u8>> {
    let src_image = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .expect("Failed to guess image format")
        .decode()
        .expect("Failed to decode image");

    let (src_width, src_height) = src_image.dimensions();
    if src_width <= MAX_DIMENSION && src_height <= MAX_DIMENSION {
        info!(
            "Image {} is already smaller than {} pixels on the longest side, skipping resize.",
            file_name, MAX_DIMENSION
        );
        return BufWriter::new(bytes.to_vec());
    }

    let scale_factor = if src_width > src_height {
        MAX_DIMENSION as f32 / src_width as f32
    } else {
        MAX_DIMENSION as f32 / src_height as f32
    };

    let dst_width = (src_width as f32 * scale_factor) as u32;
    let dst_height = (src_height as f32 * scale_factor) as u32;
    info!(
        "Resizing image {} from {}x{} to {}x{}",
        file_name, src_width, src_height, dst_width, dst_height
    );

    let mut final_image = src_image.resize(dst_width, dst_height, FilterType::Lanczos3);

    if src_width > src_height {
        final_image = final_image.rotate90();
    }

    let mut buf = BufWriter::new(Vec::new());
    let encoder = JpegEncoder::new_with_quality(&mut buf, JPEG_QUALITY);
    final_image
        .write_with_encoder(encoder)
        .expect("Failed to encode image");

    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::{Read, Write};
    use std::path::Path;

    #[tokio::test]
    async fn test_resize_with_quality() {
        // Load the image from disk
        let input_path = "testdata/20250529_104556.jpg";
        let output_path = "testdata/resized_output.jpg";
        let mut file = File::open(input_path).expect("Failed to open input image");
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .expect("Failed to read input image");

        // Call the resize_with_quality function
        let resized_image = resize_with_quality("20250529_124118.jpg", &buffer).await;

        // Save the resized image to disk
        let mut output_file = File::create(output_path).expect("Failed to create output image");
        output_file
            .write_all(&resized_image.into_inner().unwrap())
            .expect("Failed to write resized image to disk");

        // Assert the output file exists
        assert!(
            Path::new(output_path).exists(),
            "Output file was not created"
        );
    }
}
