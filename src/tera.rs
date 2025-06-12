use log::{error, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::sync::OnceLock;
use tera::Tera;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct ImageMetadata {
    pub location: String,
    pub coordinates: String,
    pub description: String,
    pub alt_text: String,
    pub caption: String,
    pub image_url: String,
}

#[derive(Deserialize, Serialize, Default)]
pub struct UploadForm {
    pub title: String,
    pub categories: String,
    pub strava: String,
    pub date: String,
    pub main: ImageMetadata,
    pub images: HashMap<String, ImageMetadata>,
}

// I want the validate function to be a function implemented on the UploadForm struct
impl UploadForm {
    pub fn validate(&self) -> Result<(), String> {
        if self.title.is_empty() {
            return Err("Title cannot be empty".to_string());
        }
        if self.categories.is_empty() {
            return Err("Categories cannot be empty".to_string());
        }
        if self.date.is_empty() {
            return Err("Date cannot be empty".to_string());
        }
        if self.main.image_url.is_empty() {
            return Err("Main image URL cannot be empty".to_string());
        }
        if self.main.description.is_empty() {
            return Err("Main image description cannot be empty".to_string());
        }
        Ok(())
    }
}

static TERA: OnceLock<Tera> = OnceLock::new();

pub fn create_post(upload_form: &UploadForm) -> Result<String, String> {
    // Initialize Tera templates once
    let tera = TERA
        .get_or_init(|| Tera::new("templates/**/*").expect("Failed to initialize Tera templates"));

    let mut context = tera::Context::new();
    context.insert("form", upload_form);

    let rendered = match tera.render("post.md", &context) {
        Ok(content) => content,
        Err(err) => {
            error!("Failed to render template: {}", err);
            return Err("Template rendering failed".to_string());
        }
    };

    let file_name_safe_title = upload_form
        .title
        .replace(|c: char| !c.is_alphanumeric(), "-")
        .to_lowercase()
        .replace("--", "-")
        .trim_matches('-')
        .to_string();

    let file_name = format!("_posts/{}-{}.md", upload_form.date, file_name_safe_title);

    match File::create(&file_name) {
        Ok(mut file) => {
            if let Err(err) = file.write_all(rendered.as_bytes()) {
                error!("Failed to write rendered content to file: {}", err);
                return Err("File writing failed".to_string());
            }
        }
        Err(err) => {
            error!("Failed to create output file: {}", err);
            return Err("File creation failed".to_string());
        }
    }

    info!("Post created successfully: {}", file_name);
    Ok(file_name_safe_title)
}
