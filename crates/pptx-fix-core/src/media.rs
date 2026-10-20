use std::sync::Arc;

use crate::analyze::MediaJob;
use crate::{EmfConverter, Progress, Stage};

pub struct Options {
    pub max_px: u32,
}

impl Default for Options {
    fn default() -> Self {
        Options { max_px: 8000 }
    }
}

pub struct Fonts {
    db: Arc<fontdb::Database>,
}

impl Fonts {
    pub fn from_bytes(fonts: &[Vec<u8>]) -> Self {
        let mut db = fontdb::Database::new();
        for f in fonts {
            db.load_font_data(f.clone());
        }
        Fonts { db: Arc::new(db) }
    }

    pub fn from_system(db: fontdb::Database) -> Self {
        Fonts { db: Arc::new(db) }
    }
}

pub fn rasterize(svg: &str, dpi: f32, fonts: &Fonts, opts: &Options) -> Result<Vec<u8>, String> {
    let uopts = usvg::Options {
        fontdb: fonts.db.clone(),
        ..Default::default()
    };
    let tree = usvg::Tree::from_str(svg, &uopts)
        .map_err(|e| format!("svg parse: {e}"))?;
    let size = tree.size();
    let nat_w = size.width();
    let nat_h = size.height();
    if nat_w <= 0.0 || nat_h <= 0.0 {
        return Err("empty svg".into());
    }
    let mut scale = dpi / 96.0;
    let longest = nat_w.max(nat_h);
    let cap = opts.max_px as f32 / longest;
    if cap < scale {
        scale = cap.max(72.0 / 96.0);
    }
    let w = (nat_w * scale).round().max(1.0) as u32;
    let h = (nat_h * scale).round().max(1.0) as u32;
    let mut pixmap = tiny_skia::Pixmap::new(w, h).ok_or("pixmap alloc")?;
    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    pixmap.encode_png().map_err(|e| e.to_string())
}

pub fn render_media(
    z: &crate::zip_io::Zip,
    jobs: &[MediaJob],
    emf: &mut dyn EmfConverter,
    fonts: &Fonts,
    opts: &Options,
    prog: &dyn Progress,
) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut out = Vec::new();
    for (i, job) in jobs.iter().enumerate() {
        let name = job.part.rsplit('/').next().unwrap_or(&job.part);
        prog.on_media(i + 1, jobs.len(), name);
        let blob = z.get(&job.part).ok_or_else(|| format!("missing {}", job.part))?;
        let svg = if job.part.to_ascii_lowercase().ends_with(".emf") {
            emf.emf_to_svg(blob)?
        } else {
            let r = metafile::to_svg(blob, metafile::RenderOptions::default())
                .map_err(|e| format!("wmf render {name}: {e:?}"))?;
            r.svg
        };
        let png = rasterize(&svg, job.dpi, fonts, opts)?;
        out.push((job.part.clone(), png));
    }
    prog.on_stage(Stage::Rewriting);
    Ok(out)
}
