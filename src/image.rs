use image::codecs::jpeg::JpegEncoder;
use image::{GenericImageView, ImageReader};
use s3::creds::Credentials;
use s3::{Bucket, Region};
use std::io::{BufWriter, Cursor};

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
        .unwrap()
        .decode()
        .unwrap();

    // // Correct orientation based on EXIF metadata
    // if let Ok(exif_reader) = exif::Reader::new().read_from_container(&mut Cursor::new(bytes)) {
    //     if let Some(orientation) = exif_reader.get_field(exif::Tag::Orientation, exif::In::PRIMARY)
    //     {
    //         src_image = match orientation.value.get_uint(0) {
    //             Some(3) => src_image.rotate180(),
    //             Some(6) => src_image.rotate90(),
    //             Some(8) => src_image.rotate270(),
    //             _ => src_image, // No rotation needed
    //         };
    //     }
    // }

    // Resize logic
    let (src_width, src_height) = src_image.dimensions();
    if src_width <= 1440 && src_height <= 1440 {
        println!(
            "Image {} is already smaller than 1440 pixels on the longest side, skipping resize.",
            file_name
        );
        return BufWriter::new(bytes.to_vec());
    }

    let scale_factor = if src_width > src_height {
        1440.0 / src_width as f32
    } else {
        1440.0 / src_height as f32
    };

    let dst_width = (src_width as f32 * scale_factor) as u32;
    let dst_height = (src_height as f32 * scale_factor) as u32;
    println!(
        "Resizing image {} from {}x{} to {}x{}",
        file_name, src_width, src_height, dst_width, dst_height
    );

    let final_image =
        src_image.resize(dst_width, dst_height, image::imageops::FilterType::Lanczos3);

    let mut buf = BufWriter::new(Vec::new());
    let encoder = JpegEncoder::new_with_quality(&mut buf, 75);
    final_image.write_with_encoder(encoder).unwrap();

    buf
}
