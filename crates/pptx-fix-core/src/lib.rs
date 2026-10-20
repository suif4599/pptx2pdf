mod analyze;
mod emf_seam;
pub mod media;
mod progress;
mod rels;
mod rewrite;
mod zip_io;

use std::collections::HashMap;

pub use emf_seam::EmfConverter;
pub use progress::{NoProgress, Progress, Stage};
pub use rewrite::Stats;

pub struct Output {
    pub pptx: Vec<u8>,
    pub stats: Stats,
    pub dropped_rels: usize,
}

pub fn fix_pptx(
    pptx: &[u8],
    fonts: &media::Fonts,
    emf: &mut dyn EmfConverter,
    prog: &dyn Progress,
    opts: &media::Options,
    font_map: &HashMap<String, String>,
) -> Result<Output, String> {
    prog.on_stage(Stage::Reading);
    let mut z = zip_io::Zip::read(pptx)?;

    if !font_map.is_empty() {
        for (name, data) in &mut z.entries {
            if name.starts_with("ppt/") && name.ends_with(".xml") {
                if let Some(remapped) = rewrite::remap_fonts(data, font_map) {
                    *data = remapped;
                }
            }
        }
    }

    prog.on_stage(Stage::Analyzing);
    let mut analysis = analyze::analyze(&z)?;
    let jobs = analyze::media_jobs(&analysis);

    let rendered = media::render_media(&z, &jobs, emf, fonts, opts, prog)?;
    let media_map: HashMap<String, String> = rendered.iter()
        .map(|(part, _)| (part.clone(), rels::fix_part_name(part)))
        .collect();
    let new_media: Vec<(String, Vec<u8>)> = rendered.iter()
        .map(|(part, png)| (rels::fix_part_name(part), png.clone()))
        .collect();

    let dropped = rels::update_rels(&mut z, &mut analysis.slides, &media_map)?;

    let mut stats = Stats::default();
    for slide in &analysis.slides {
        let xml = z.get(&slide.slide_part).ok_or("missing slide")?.to_vec();
        let out = rewrite::rewrite_slide(&xml, slide, &mut stats)?;
        z.set(&slide.slide_part, out);
    }

    for (name, data) in new_media {
        z.set(&name, data);
    }
    rels::ensure_png_content_type(&mut z);

    prog.on_stage(Stage::Done);
    let pptx = z.write()?;
    Ok(Output { pptx, stats, dropped_rels: dropped })
}
