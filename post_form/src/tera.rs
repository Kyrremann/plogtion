use log::{error, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use tera::Tera;

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct ImageMetadata {
    pub file_name: String,
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

pub fn create_post(upload_form: &UploadForm) -> Result<String, String> {
    let mut tera = Tera::default();
    tera.add_raw_template("post.md", r##"---
title: "{{ form.title }}"
date: "{{ form.date }}"
categories: "{{ form.categories }}"
main:
  image: "{{ form.main.image_url }}"
  alt_text: "{{ form.main.alt_text }}"
  {%- if form.main.caption %}
  caption: "{{ form.main.caption }}"
  {% endif %}
  {%- if form.main.location %}
  location: "{{ form.main.location }}"
  coordinates: "{{ form.main.coordinates }}"
  coordinates_url: "https://www.google.com/maps/place/{{ form.main.coordinates }}"
  {% endif %}
{%- if form.strava %}strava: "{{ form.strava }}"{% endif %}
---

{{ form.main.description }}

{% for key, metadata in form.images %}
{%- if key == form.main.file_name %}{% continue %}{% endif -%}
![{{ metadata.alt_text }}]({{ metadata.image_url }})
{%- if metadata.caption %}
*{%- if metadata.location %}[{{ metadata.location }}](https://www.google.com/maps/place/{{ metadata.coordinates }}): {% endif %}{{ metadata.caption }}*
{% endif %}
{%- if metadata.description %}
{{ metadata.description }}
{% endif %}
{% endfor -%}
"##).map_err(|err| {
        error!("Failed to add template: {}", err);
        "Template initialization failed".to_string()
    })?;

    let mut context = tera::Context::new();
    context.insert("form", upload_form);

    let rendered = tera.render("post.md", &context).map_err(|err| {
        error!("Failed to render template: {}", err);
        "Template rendering failed".to_string()
    })?;

    let file_name_safe_title = create_file_name_safe_title(&upload_form.title);

    let file_name = format!(
        "plog/_posts/{}-{}.md",
        upload_form.date, file_name_safe_title
    );

    info!("Content to be written: {}", rendered.trim_end());

    File::create(&file_name)
        .and_then(|mut file| file.write_all(rendered.trim_end().as_bytes()))
        .map_err(|err| {
            error!("Failed to write rendered content to file: {}", err);
            "File writing failed".to_string()
        })?;

    info!("Post created successfully: {}", file_name);
    Ok(file_name_safe_title)
}

fn create_file_name_safe_title(title: &str) -> String {
    trim_whitespace(&title.replace(|c: char| !c.is_alphanumeric(), " "))
        .to_lowercase()
        .to_string()
}

// From https://stackoverflow.com/a/71864249/502493
pub fn trim_whitespace(s: &str) -> String {
    // second attempt: only allocate a string
    let mut result = String::with_capacity(s.len());
    s.split_whitespace().for_each(|w| {
        if !result.is_empty() {
            result.push('-');
        }
        result.push_str(w);
    });
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_file_name_safe_title() {
        let cases = vec![
            ("Hello World", "hello-world"),
            ("Rust Programming!", "rust-programming"),
            ("Multiple   Spaces", "multiple-spaces"),
            ("Special@#Characters", "special-characters"),
            ("--Already-Safe--", "already-safe"),
            ("Trailing--", "trailing"),
            (
                "Day two, 108km, 864m - Wonderful weather on straight roads",
                "day-two-108km-864m-wonderful-weather-on-straight-roads",
            ),
        ];

        for (input, expected) in cases {
            assert_eq!(create_file_name_safe_title(input), expected);
        }
    }
}
