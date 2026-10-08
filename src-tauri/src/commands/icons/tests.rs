use super::{data_url_png, decode_icon_resource};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
use std::io::Cursor;

fn one_pixel_png() -> Vec<u8> {
    let mut png = Vec::new();
    DynamicImage::ImageRgba8(RgbaImage::from_pixel(1, 1, Rgba([255, 0, 0, 255])))
        .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .unwrap();
    png
}

#[test]
fn icon_data_urls_from_older_configs_decode() {
    let png = one_pixel_png();
    let data_url = format!("data:image/png;base64,{}", STANDARD.encode(&png));

    assert_eq!(data_url_png(&data_url), Some(png));
}

#[test]
fn invalid_data_urls_do_not_decode() {
    assert_eq!(data_url_png("not-a-data-url"), None);
    assert_eq!(data_url_png("data:image/png;base64,%%%"), None);
}

#[test]
fn decodes_png_icon_resources_wrapped_in_ico() {
    let decoded = decode_icon_resource(&one_pixel_png()).unwrap();

    assert_eq!(decoded.width(), 1);
    assert_eq!(decoded.height(), 1);
}
