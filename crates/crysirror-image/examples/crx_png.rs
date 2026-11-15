//! Diagnostic: decode Circus CRX files to PNG. `crx_png OUT_DIR FILE...`
fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let out = std::path::PathBuf::from(args.next().expect("OUT_DIR"));
    for path in args {
        let image = crysirror_image::crx::decode(&std::fs::read(&path)?)?;
        let s = &image.surface;
        let name = std::path::Path::new(&path).file_stem().unwrap().to_string_lossy().to_string();
        let file = std::fs::File::create(out.join(format!("{name}.png")))?;
        let mut enc = png::Encoder::new(std::io::BufWriter::new(file), s.width, s.height);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header()?.write_image_data(&s.rgba)?;
        println!("{name} {}x{} offset {:?}", s.width, s.height, image.offset);
    }
    Ok(())
}
