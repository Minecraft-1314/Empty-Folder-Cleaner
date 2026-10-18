use std::path::Path;

use tao::window::Icon;

const CANDIDATES: [&str; 4] = ["app-256.png", "app-64.png", "app-48.png", "app-32.png"];

pub fn load_icon(directory: &Path) -> Option<Icon> {
    CANDIDATES
        .iter()
        .find_map(|name| decode_png(&directory.join("Ui").join("Icon").join(name)))
}

fn decode_png(path: &Path) -> Option<Icon> {
    let file = std::fs::File::open(path).ok()?;
    let mut decoder = png::Decoder::new(file);
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let mut buffer = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buffer).ok()?;
    let bytes = &buffer[..info.buffer_size()];
    let rgba = match info.color_type {
        png::ColorType::Rgba => bytes.to_vec(),
        png::ColorType::Rgb => expand(bytes, 3),
        png::ColorType::Grayscale => expand(bytes, 1),
        png::ColorType::GrayscaleAlpha => return None,
        png::ColorType::Indexed => return None,
    };
    Icon::from_rgba(rgba, info.width, info.height).ok()
}

fn expand(bytes: &[u8], channels: usize) -> Vec<u8> {
    let mut result = Vec::with_capacity(bytes.len() / channels * 4);
    for chunk in bytes.chunks_exact(channels) {
        let (red, green, blue) = if channels == 1 {
            (chunk[0], chunk[0], chunk[0])
        } else {
            (chunk[0], chunk[1], chunk[2])
        };
        result.extend_from_slice(&[red, green, blue, 255]);
    }
    result
}
