use std::io::Cursor;
use std::path::Path;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use editpe::Image as PeImage;

/// Extracts an executable's embedded icon and returns it as a ready-to-use
/// `data:image/png;base64,...` URI. Returns `None` whenever the file can't be
/// parsed or has no icon resource — this is a cosmetic nicety for the game
/// library, never a reason to fail adding or updating a game.
pub fn extract_icon_data_url(exe_path: &Path) -> Option<String> {
    let pe_image = PeImage::parse_file(exe_path).ok()?;
    let resources = pe_image.resource_directory()?;
    let icon_bytes = resources.get_main_icon().ok()??;
    let decoded = image::load_from_memory(icon_bytes).ok()?;

    let mut png_bytes = Vec::new();
    decoded
        .write_to(&mut Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .ok()?;

    Some(format!(
        "data:image/png;base64,{}",
        STANDARD.encode(png_bytes)
    ))
}
