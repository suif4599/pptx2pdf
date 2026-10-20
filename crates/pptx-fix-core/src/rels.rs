use std::collections::HashMap;

use crate::analyze::SlidePlan;
use crate::zip_io::Zip;

pub const IMAGE_REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image";

pub fn fix_part_name(part: &str) -> String {
    let base = part.rsplit('/').next().unwrap_or(part);
    format!("ppt/media/fix_{base}.png")
}

fn max_rid(slide: &SlidePlan) -> u32 {
    slide.rels.iter()
        .filter_map(|r| r.id.strip_prefix("rId").and_then(|n| n.parse::<u32>().ok()))
        .max()
        .unwrap_or(0)
}

fn relative_target(slide_dir: &str, part: &str) -> String {
    let sd: Vec<&str> = slide_dir.split('/').collect();
    let pd: Vec<&str> = part.split('/').collect();
    let mut i = 0;
    while i < sd.len() && i < pd.len() && sd[i] == pd[i] {
        i += 1;
    }
    let mut segs: Vec<String> = vec!["..".to_string(); sd.len() - i];
    segs.extend(pd[i..].iter().map(|s| s.to_string()));
    segs.join("/")
}

pub fn update_rels(
    z: &mut Zip,
    slides: &mut Vec<SlidePlan>,
    media_map: &HashMap<String, String>,
) -> Result<usize, String> {
    let mut dropped = 0usize;
    for slide in slides.iter_mut() {
        let mut used: Vec<(String, String)> = Vec::new();
        let mut consider = |rid: &str, slide: &SlidePlan, used: &mut Vec<(String, String)>| {
            if used.iter().any(|(o, _)| o == rid) {
                return;
            }
            if let Some(r) = slide.rels.iter().find(|r| r.id == rid) {
                if media_map.contains_key(&r.target) {
                    used.push((rid.to_string(), r.target.clone()));
                }
            }
        };
        for eq in &slide.equations {
            consider(&eq.rid, slide, &mut used);
        }
        for pic in &slide.pics {
            consider(&pic.rid, slide, &mut used);
        }
        if used.is_empty() && slide.other_ole {
            continue;
        }
        let mut next = max_rid(slide) + 1;
        let mut added: Vec<(String, String)> = Vec::new();
        for (old_rid, part) in &used {
            let new_part = &media_map[part];
            added.push((old_rid.clone(), format!("rId{next}")));
            slide.rels.push(crate::analyze::Rel {
                id: format!("rId{next}"),
                target: new_part.clone(),
                rtype: IMAGE_REL.to_string(),
                external: false,
            });
            next += 1;
        }
        slide.new_rids = added;

        let drop_legacy = !slide.other_ole;
        if drop_legacy {
            let before = slide.rels.len();
            slide.rels.retain(|r| {
                !(r.rtype.ends_with("/oleObject") || r.rtype.ends_with("/vmlDrawing"))
            });
            dropped += before - slide.rels.len();
        }

        let slide_dir = slide.slide_part.rsplit_once('/').map(|(d, _)| d).unwrap_or(".");
        let mut xml = String::from(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">",
        );
        for r in &slide.rels {
            let ext = if r.external { " TargetMode=\"External\"" } else { "" };
            let target = if r.external {
                r.target.clone()
            } else {
                relative_target(slide_dir, &r.target)
            };
            xml.push_str(&format!(
                "\n<Relationship Id=\"{}\" Type=\"{}\" Target=\"{}\"{ext}/>",
                r.id, r.rtype, xml_escape(&target)
            ));
        }
        xml.push_str("\n</Relationships>");
        z.set(&slide.rels_part.clone(), xml.into_bytes());
    }
    Ok(dropped)
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

pub fn ensure_png_content_type(z: &mut Zip) {
    if let Some(ct) = z.get("[Content_Types].xml") {
        let s = String::from_utf8_lossy(ct);
        if s.contains("Extension=\"png\"") {
            return;
        }
        if let Some(pos) = s.find('>') {
            let inserted = format!(
                "{}<Default Extension=\"png\" ContentType=\"image/png\"/>{}",
                &s[..pos + 1],
                &s[pos + 1..]
            );
            z.set("[Content_Types].xml", inserted.into_bytes());
        }
    }
}
