
use axum::extract::Multipart;
use axum::response::Html;
use image::codecs::jpeg::JpegEncoder;
use image::{GenericImageView, ImageReader};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Cursor, Write};

#[derive(Deserialize)]
pub struct ImageMetadata {
    location: String,
    description: String,
}

#[derive(Deserialize)]
pub struct UploadForm {
    title: String,
    description: String,
    date: String,
    geo_location: HashMap<String, ImageMetadata>,
}

pub async fn upload(mut multipart: Multipart) -> Html<String> {
    let mut form = UploadForm {
        title: String::new(),
        description: String::new(),
        date: String::new(),
        geo_location: HashMap::new(),
    };

    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();

        if name != "images" {
            let text = field.text().await.unwrap();

            if name == "title" {
                form.title = text.clone();
            } else if name == "description" {
                form.description = text.clone()
            } else if name == "date" {
                form.date = text.clone();
            } else if name.ends_with("_description") {
                let key = name.split("_description").next().unwrap();

                if let Some(value) = form.geo_location.get(key) {
                    form.geo_location.insert(
                        key.to_string(),
                        ImageMetadata {
                            description: text.clone(),
                            location: value.location.clone(),
                        },
                    );
                } else {
                    form.geo_location.insert(
                        key.to_string(),
                        ImageMetadata {
                            description: text.clone(),
                            location: String::new(),
                        },
                    );
                }
            } else if name.ends_with("_location") {
                let key = name.split("_location").next().unwrap();

                if let Some(value) = form.geo_location.get(key) {
                    form.geo_location.insert(
                        key.to_string(),
                        ImageMetadata {
                            location: text.clone(),
                            description: value.description.clone(),
                        },
                    );
                } else {
                    form.geo_location.insert(
                        key.to_string(),
                        ImageMetadata {
                            location: text.clone(),
                            description: String::new(),
                        },
                    );
                }
            }
            println!("Field: {} = {}", name, &text);
        } else {
            let file_name = field.file_name().unwrap().to_string();
            let content_type = field.content_type().unwrap().to_string();
            let data = field.bytes().await.unwrap();

            let mut file = File::create(format!("images/{file_name}")).unwrap();
            file.write_all(&data).unwrap();

            println!(
                "Size of {file_name} ({content_type}) is {} bytes",
                data.len()
            );

            resize_with_quality(&file_name, &data).await;
        }
    }

    println!(
        "Title: {}, Description: {}, Date: {}, Locations: {}",
        form.title,
        form.description,
        form.date,
        form.geo_location.len(),
    );

    for (key, value) in form.geo_location.iter() {
        println!(
            "Key: {}, Description: {}, Location: {}",
            key, value.description, value.location
        );
    }

    Html("Form and multipart data processed successfully!".to_string())
}

async fn resize_with_quality(file_name: &str, bytes: &[u8]) {
    let src_image = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .unwrap()
        .decode()
        .unwrap();

    // I want the longest side to be 1440 pixels and the other side to be scaled proportionally.
    let (src_width, src_height) = src_image.dimensions();
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

    let mut file: File = File::create(format!("images/resized_{}", file_name)).unwrap();
    file.write_all(&buf.into_inner().unwrap()).unwrap();
}
