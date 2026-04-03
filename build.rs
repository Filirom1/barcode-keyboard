fn main() {
    #[cfg(windows)]
    embed_windows_resource();
}

#[cfg(windows)]
fn embed_windows_resource() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let icon_path = std::path::Path::new(&out_dir).join("barcode.ico");

    let (rgba, w, h) = barcode_rgba(32);
    let image = ico::IconImage::from_rgba_data(w, h, rgba);
    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);
    icon_dir.add_entry(ico::IconDirEntry::encode(&image).unwrap());
    let mut f = std::fs::File::create(&icon_path).unwrap();
    icon_dir.write(&mut f).unwrap();

    winres::WindowsResource::new()
        .set_icon(icon_path.to_str().unwrap())
        .compile()
        .unwrap();
}

/// Generate a simple barcode RGBA image (white background, black vertical bars).
fn barcode_rgba(size: u32) -> (Vec<u8>, u32, u32) {
    let w = size as usize;
    let h = size as usize;
    // 26-column barcode pattern (0=white, 1=black), fits in 32px with 3px quiet zones each side
    const BARS: [u8; 26] = [1,1,0,1,0,1,1,0,1,1,0,1,0,0,1,0,1,1,0,1,0,1,0,1,1,0];
    let bar_top    = h / 8;         // ~4 px
    let bar_bottom = h * 7 / 8;    // ~28 px
    let quiet      = h / 10 + 1;   // ~4 px quiet zone on each side

    let mut rgba = vec![255u8; w * h * 4]; // white, fully opaque
    for y in bar_top..bar_bottom {
        for (i, &bar) in BARS.iter().enumerate() {
            let x = quiet + i;
            if x < w && bar == 1 {
                let base = (y * w + x) * 4;
                rgba[base]   = 0;   // R
                rgba[base+1] = 0;   // G
                rgba[base+2] = 0;   // B
                // alpha stays 255
            }
        }
    }
    (rgba, w as u32, h as u32)
}
