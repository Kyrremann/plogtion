use chrono::NaiveDate;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use tera::Tera;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ImageMetadata {
    pub file_name: String,
    pub location: String,
    pub coordinates: String,
    pub description: String,
    pub alt_text: String,
    pub caption: String,
    pub image_url: String,
    pub is_video: bool,
    pub date: NaiveDate,
}

impl Default for ImageMetadata {
    fn default() -> Self {
        Self {
            file_name: String::new(),
            location: String::new(),
            coordinates: String::new(),
            description: String::new(),
            alt_text: String::new(),
            caption: String::new(),
            image_url: String::new(),
            is_video: false,
            date: NaiveDate::from_ymd_opt(1970, 1, 1).unwrap(),
        }
    }
}

#[derive(Deserialize, Serialize, Default)]
pub struct UploadForm {
    pub title: String,
    pub categories: String,
    pub strava: String,
    pub date: String,
    pub feature: ImageMetadata,
    pub images: Vec<ImageMetadata>,
}

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
        if self.feature.image_url.is_empty() {
            return Err("Missing featured image".to_string());
        }
        Ok(())
    }
}

pub fn create_post(upload_form: &UploadForm) -> Result<String, String> {
    let (file_name_safe_title, rendered) = render(upload_form)?;

    let file_name = format!(
        "plog/_posts/{}-{}.md",
        upload_form.date, file_name_safe_title
    );

    File::create(&file_name)
        .and_then(|mut file| file.write_all(rendered.trim_end().as_bytes()))
        .map_err(|err| {
            error!("Failed to write rendered content to file: {err}");
            "File writing failed".to_string()
        })?;

    info!("Post created successfully: {file_name}");
    Ok(file_name_safe_title)
}

fn render(upload_form: &UploadForm) -> Result<(String, String), String> {
    let mut tera = Tera::default();
    tera.add_raw_template("post.md", r##"---
title: "{{ form.title }}"
date: "{{ form.date }}"
categories: "{{ form.categories }}"
feature:
  image: "{{ form.feature.image_url }}"
{% if form.strava %}strava: "{{ form.strava }}"{% endif %}
---

{% for metadata in form.images %}
{%- if metadata.is_video %}
<video width="640" preload="auto" controls src="{{ metadata.image_url }}" type="video/mp4">
	Your browser does not support HTML5 video.
</video>
{% if metadata.caption %}
*{{ metadata.caption }}*
{% endif %}
{% else %}
![{{ metadata.alt_text }}]({{ metadata.image_url }})
{%- if metadata.caption %}
*{%- if metadata.location %}[{{ metadata.location }}](https://www.google.com/maps/place/{{ metadata.coordinates }}): {% endif %}{{ metadata.caption }}*
{% endif %}
{%- if metadata.description %}
{{ metadata.description }}
{%- endif %}
{%- endif %}
{% endfor -%}
"##).map_err(|err| {
        error!("Failed to add template: {err}");
        "Template initialization failed".to_string()
    })?;

    let mut context = tera::Context::new();
    context.insert("form", upload_form);

    let rendered = tera.render("post.md", &context).map_err(|err| {
        error!("Failed to render template: {err}");
        "Template rendering failed".to_string()
    })?;

    let file_name_safe_title = create_file_name_safe_title(&upload_form.title);
    Ok((file_name_safe_title, rendered))
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
    use pretty_assertions::assert_eq;

    #[test]
    fn test_render_post() {
        let upload_form = UploadForm {
            title: "Test Post".to_string(),
            categories: "test, example".to_string(),
            strava: "123456789".to_string(),
            date: "2023-10-01".to_string(),
            feature: ImageMetadata {
                image_url: "https://example.com/image.jpg".to_string(),
                ..Default::default()
            },
            images: vec![
                ImageMetadata {
                    file_name: "20230101_120000.jpg".to_string(),
                    image_url: "https://example.com/image1.jpg".to_string(),
                    date: NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
                    caption: "hello world".to_string(),
                    description: "some story about my life, as an intro".to_string(),
                    ..Default::default()
                },
                ImageMetadata {
                    file_name: "20230102_120000.jpg".to_string(),
                    image_url: "https://example.com/image2.jpg".to_string(),
                    date: NaiveDate::from_ymd_opt(2023, 1, 2).unwrap(),
                    caption: "what a great image".to_string(),
                    description: "this is what I did later".to_string(),
                    ..Default::default()
                },
            ],
        };

        let result = render(&upload_form);
        assert!(result.is_ok());
        let (file_name_safe_title, rendered) = result.unwrap();
        assert_eq!(file_name_safe_title, "test-post");
        println!("{}", rendered);
        assert_eq!(
            rendered,
            r##"---
title: "Test Post"
date: "2023-10-01"
categories: "test, example"
feature:
  image: "https://example.com/image.jpg"
strava: "123456789"
---


![](https://example.com/image1.jpg)
*hello world*

some story about my life, as an intro

![](https://example.com/image2.jpg)
*what a great image*

this is what I did later
"##
        );
    }

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
