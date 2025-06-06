use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use tera::Tera;

#[derive(Default, Serialize, Deserialize)]
pub(crate) struct ImageMetadata {
    pub location: String,
    pub description: String,
    pub alt_text: String,
    pub image: String,
}

#[derive(Deserialize, Serialize, Default)]
pub(crate) struct UploadForm {
    pub title: String,
    pub categories: String,
    pub description: String,
    pub date: String,
    pub main_image: String,
    pub images: HashMap<String, ImageMetadata>,
}

pub(crate) fn create_post(upload_form: &UploadForm) -> String {
    let tera = Tera::new("templates/**/*").expect("Failed to initialize Tera templates");
    let mut context = tera::Context::new();
    context.insert("form", upload_form);
    let rendered = tera
        .render("post.md", &context)
        .expect("Failed to render template");

    let file_name_safe_title = upload_form
        .title
        .replace(|c: char| !c.is_alphanumeric(), "-")
        .to_lowercase()
        .replace("--", "-");
    let file_name = format!("_posts/{}-{}.md", upload_form.date, file_name_safe_title);

    let mut file = File::create(file_name).expect("Failed to create output file");
    file.write_all(rendered.as_bytes())
        .expect("Failed to write rendered content to file");

    file_name_safe_title
}
