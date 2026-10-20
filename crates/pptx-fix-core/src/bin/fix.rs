use std::process::ExitCode;

use pptx_fix_core::{fix_pptx, media, EmfConverter};

struct Emf2SvgConv;

impl EmfConverter for Emf2SvgConv {
    fn emf_to_svg(&mut self, emf: &[u8]) -> Result<String, String> {
        let dir = std::env::temp_dir().join(format!("fix-emf-{}", std::process::id()));
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let in_path = dir.join("in.emf");
        let out_path = dir.join("out.svg");
        std::fs::write(&in_path, emf).map_err(|e| e.to_string())?;
        let st = std::process::Command::new("emf2svg-conv")
            .args(["-p", "-i"]).arg(&in_path).arg("-o").arg(&out_path)
            .output().map_err(|e| e.to_string())?;
        if !st.status.success() {
            return Err(format!("emf2svg-conv: {}", String::from_utf8_lossy(&st.stderr)));
        }
        let svg = std::fs::read_to_string(&out_path).map_err(|e| e.to_string())?;
        let _ = std::fs::remove_file(&in_path);
        let _ = std::fs::remove_file(&out_path);
        Ok(svg)
    }
}

struct CliProgress;

impl pptx_fix_core::Progress for CliProgress {
    fn on_stage(&self, stage: pptx_fix_core::Stage) {
        eprintln!("stage: {}", stage.label());
    }
    fn on_media(&self, cur: usize, total: usize, name: &str) {
        eprintln!("media {cur}/{total}: {name}");
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: fix <input.pptx> <output.pptx>");
        return ExitCode::FAILURE;
    }
    let fonts: Vec<Vec<u8>> = match std::env::var("FIX_FONTS_DIR") {
        Ok(d) => {
            let mut v = Vec::new();
            if let Ok(rd) = std::fs::read_dir(&d) {
                for e in rd.flatten() {
                    if let Ok(b) = std::fs::read(e.path()) {
                        v.push(b);
                    }
                }
            }
            v
        }
        Err(_) => Vec::new(),
    };
    let fonts = if fonts.is_empty() {
        let mut db = fontdb::Database::new();
        db.load_system_fonts();
        eprintln!("note: FIX_FONTS_DIR unset, using system fonts");
        media::Fonts::from_system(db)
    } else {
        media::Fonts::from_bytes(&fonts)
    };
    let input = match std::fs::read(&args[1]) {
        Ok(b) => b,
        Err(e) => { eprintln!("read error: {e}"); return ExitCode::FAILURE; }
    };
    let mut emf = Emf2SvgConv;
    match fix_pptx(&input, &fonts, &mut emf, &CliProgress, &media::Options::default(), &std::collections::HashMap::new()) {
        Ok(out) => {
            if let Err(e) = std::fs::write(&args[2], &out.pptx) {
                eprintln!("write error: {e}");
                return ExitCode::FAILURE;
            }
            eprintln!(
                "math fallbacks: {}, replaced: {}, skipped on snapshots: {}, retargeted: {}, rels dropped: {}",
                out.stats.math_fallbacks, out.stats.replaced,
                out.stats.skipped_on_snapshots, out.stats.retargeted, out.dropped_rels
            );
            ExitCode::SUCCESS
        }
        Err(e) => { eprintln!("fix error: {e}"); ExitCode::FAILURE }
    }
}
