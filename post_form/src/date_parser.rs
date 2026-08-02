use chrono::{Local, NaiveDate};
use log::error;

pub fn parse_date_from_filename(file_name: &str) -> NaiveDate {
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
