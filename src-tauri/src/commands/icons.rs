use std::io::Cursor;
use std::path::Path;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use editpe::Image as PeImage;

const PNG_SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

/// Extracts an executable's embedded icon and returns it as a ready-to-use
/// `data:image/png;base64,...` URI. Returns `None` whenever the file can't be
/// parsed or has no icon resource — this is a cosmetic nicety for the game
/// library, never a reason to fail adding or updating a game.
pub fn extract_icon_data_url(exe_path: &Path) -> Option<String> {
    let pe_image = PeImage::parse_file(exe_path).ok()?;
    let resources = pe_image.resource_directory()?;
    let icon_bytes = resources.get_main_icon().ok()??;
    let decoded = decode_icon_resource(icon_bytes)?;

    let mut png_bytes = Vec::new();
    decoded
        .write_to(&mut Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .ok()?;

    Some(format!(
        "data:image/png;base64,{}",
        STANDARD.encode(png_bytes)
    ))
}

/// `editpe::get_main_icon` hands back a single icon entry's raw image bytes —
/// a PNG for large modern icons, or a headerless DIB (a BMP stripped of its
/// 14-byte file header, with the AND transparency mask stacked underneath the
/// color data) for classic ones — not a self-describing `.ico` file. `image`'s
/// decoders need an actual file, so this rebuilds the minimal single-entry
/// ICO container (6-byte ICONDIR + 16-byte ICONDIRENTRY) that `editpe`
/// stripped off, then hands it to `image`'s own ICO decoder, which already
/// knows how to tell PNG and DIB-with-AND-mask entries apart and merge the
/// mask into an alpha channel.
fn decode_icon_resource(data: &[u8]) -> Option<image::DynamicImage> {
    let (width, height) = if data.starts_with(&PNG_SIGNATURE) {
        // The PNG IHDR chunk is always first: 8-byte signature + 4-byte
        // length + 4-byte "IHDR" + 4-byte width + 4-byte height, big-endian.
        if data.len() < 24 {
            return None;
        }
        let width = u32::from_be_bytes(data[16..20].try_into().ok()?);
        let height = u32::from_be_bytes(data[20..24].try_into().ok()?);
        (width, height)
    } else {
        // BITMAPINFOHEADER: width at offset 4, height at offset 8. The
        // height covers the XOR color data and the AND mask stacked on top
        // of each other, so the real icon height is half of it (mirroring
        // what `BmpDecoder::read_metadata_in_ico_format` does internally).
        if data.len() < 40 {
            return None;
        }
        let width = i32::from_le_bytes(data[4..8].try_into().ok()?);
        let raw_height = i32::from_le_bytes(data[8..12].try_into().ok()?);
        if width < 0 {
            return None;
        }
        (width as u32, raw_height.unsigned_abs() / 2)
    };
    if width == 0 || height == 0 || width > 256 || height > 256 {
        return None;
    }

    let mut ico = Vec::with_capacity(22 + data.len());
    ico.extend_from_slice(&[0, 0, 1, 0, 1, 0]); // ICONDIR: reserved=0, type=1 (icon), count=1
    ico.push(if width == 256 { 0 } else { width as u8 });
    ico.push(if height == 256 { 0 } else { height as u8 });
    ico.extend_from_slice(&[0, 0]); // color_count, reserved
    ico.extend_from_slice(&[1, 0]); // planes = 1
    ico.extend_from_slice(&[0, 0]); // bpp = unspecified
    ico.extend_from_slice(&(data.len() as u32).to_le_bytes()); // image_length
    ico.extend_from_slice(&22u32.to_le_bytes()); // image_offset, right after this header
    ico.extend_from_slice(data);

    image::load_from_memory_with_format(&ico, image::ImageFormat::Ico).ok()
}
