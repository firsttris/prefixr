use super::{data_url_png, decode_icon_resource, png_data_url};
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
fn png_data_urls_round_trip() {
    let png = one_pixel_png();
    let data_url = png_data_url(&png);

    assert!(data_url.starts_with("data:image/png;base64,"));
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
