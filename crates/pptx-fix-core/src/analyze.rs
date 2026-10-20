use crate::zip_io::Zip;

pub const NS_P: &str = "http://schemas.openxmlformats.org/presentationml/2006/main";
pub const NS_A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
pub const NS_R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
pub const NS_A14: &str = "http://schemas.microsoft.com/office/drawing/2010/main";
pub const NS_MC: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: i64,
    pub y: i64,
    pub cx: i64,
    pub cy: i64,
}

#[derive(Debug)]
pub struct EqJob {
    pub rid: String,
    pub rect: Rect,
}

#[derive(Debug)]
pub struct PicJob {
    pub rid: String,
}

#[derive(Debug, Clone)]
pub struct Rel {
    pub id: String,
    pub target: String,
    pub rtype: String,
    pub external: bool,
}

#[derive(Debug)]
pub struct SlidePlan {
    pub slide_part: String,
    pub rels_part: String,
    pub rels: Vec<Rel>,
    pub equations: Vec<EqJob>,
    pub pics: Vec<PicJob>,
    pub snapshots: Vec<Rect>,
    /// oleObj elements surviving the rewrite (non-replaced frames and others)
    pub other_ole: bool,
    /// rid -> replacement media part ("ppt/media/fix_imageN.png")
    pub new_rids: Vec<(String, String)>,
}

#[derive(Debug)]
pub struct Analysis {
    pub slide_w: i64,
    pub slide_h: i64,
    pub slides: Vec<SlidePlan>,
}

pub const FORMULA_DPI: f32 = 600.0;
pub const PAGE_DPI: f32 = 300.0;

pub struct MediaJob {
    /// package part of the source metafile
    pub part: String,
    pub dpi: f32,
}

/// media jobs in python's order: equation media first (formula dpi wins on shared parts)
pub fn media_jobs(a: &Analysis) -> Vec<MediaJob> {
    let mut out: Vec<MediaJob> = Vec::new();
    let mut push = |part: String, dpi: f32, out: &mut Vec<MediaJob>| {
        if let Some(existing) = out.iter_mut().find(|m| m.part == part) {
            if dpi > existing.dpi {
                existing.dpi = dpi;
            }
            return;
        }
        out.push(MediaJob { part, dpi });
    };
    for s in &a.slides {
        let part_of = |rid: &str| -> Option<String> {
            s.rels.iter().find(|r| r.id == rid).map(|r| r.target.clone())
        };
        for eq in &s.equations {
            if let Some(p) = part_of(&eq.rid) {
                push(p, FORMULA_DPI, &mut out);
            }
        }
        for pic in &s.pics {
            if let Some(p) = part_of(&pic.rid) {
                push(p, PAGE_DPI, &mut out);
            }
        }
    }
    out
}

pub fn has_tag(n: roxmltree::Node, ns: &str, local: &str) -> bool {
    n.tag_name().namespace() == Some(ns) && n.tag_name().name() == local
}

fn descendant<'a>(n: roxmltree::Node<'a, 'a>, ns: &str, local: &str) -> Option<roxmltree::Node<'a, 'a>> {
    n.descendants().find(|d| has_tag(*d, ns, local))
}

fn rid_embed(n: roxmltree::Node) -> Option<String> {
    descendant(n, NS_A, "blip")
        .and_then(|b| b.attribute((NS_R, "embed")))
        .map(|s| s.to_string())
}

pub fn is_math_fallback(ac: roxmltree::Node) -> bool {
    let choice_ok = ac.children().any(|c| {
        has_tag(c, NS_MC, "Choice") && descendant(c, NS_A14, "m").is_some()
    });
    let fallback_ok = ac.children().any(|c| {
        has_tag(c, NS_MC, "Fallback") && descendant(c, NS_A, "blip").is_some()
    });
    choice_ok && fallback_ok
}

/// Equation graphicFrame: has an Equation* oleObj. Returns (has_blip, rect).
fn equation_frame(gf: roxmltree::Node) -> Option<(bool, Rect)> {
    let eq = gf.descendants().find(|d| {
        has_tag(*d, NS_P, "oleObj")
            && d.attribute("progId").is_some_and(|p| p.starts_with("Equation"))
    })?;
    let _ = eq;
    let rect = gf.children()
        .find(|c| has_tag(*c, NS_P, "xfrm"))
        .and_then(|x| {
            let off = x.children().find(|c| has_tag(*c, NS_A, "off"))?;
            let ext = x.children().find(|c| has_tag(*c, NS_A, "ext"))?;
            Some(Rect {
                x: off.attribute("x")?.parse().ok()?,
                y: off.attribute("y")?.parse().ok()?,
                cx: ext.attribute("cx")?.parse().ok()?,
                cy: ext.attribute("cy")?.parse().ok()?,
            })
        })?;
    let rid = rid_embed(gf);
    Some((rid.is_some(), rect))
}

fn in_graphic_frame(n: roxmltree::Node) -> bool {
    n.ancestors().any(|a| has_tag(a, NS_P, "graphicFrame"))
}

fn part_ext(part: &str) -> String {
    part.rsplit('.').next().unwrap_or("").to_ascii_lowercase()
}

fn resolve_target(slide_dir: &str, target: &str) -> String {
    // slide_dir like "ppt/slides", target like "../media/image3.wmf"
    if target.starts_with('/') {
        return target.trim_start_matches('/').to_string();
    }
    let mut stack: Vec<&str> = slide_dir.split('/').collect();
    for seg in target.split('/') {
        match seg {
            ".." => { stack.pop(); }
            "." | "" => {}
            s => stack.push(s),
        }
    }
    stack.join("/")
}

fn parse_rels(xml: &[u8]) -> Result<Vec<Rel>, String> {
    let doc = roxmltree::Document::parse(std::str::from_utf8(xml).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for rel in doc.descendants().filter(|n| has_tag(*n, "http://schemas.openxmlformats.org/package/2006/relationships", "Relationship")) {
        out.push(Rel {
            id: rel.attribute("Id").unwrap_or_default().to_string(),
            target: rel.attribute("Target").unwrap_or_default().to_string(),
            rtype: rel.attribute("Type").unwrap_or_default().to_string(),
            external: rel.attribute("TargetMode") == Some("External"),
        });
    }
    Ok(out)
}

fn slide_parts(z: &Zip) -> Vec<String> {
    // order from presentation.xml sldIdLst when parseable, else numeric glob
    let ordered = (|| -> Option<Vec<String>> {
        let pres = z.get("ppt/presentation.xml")?;
        let rels = z.get("ppt/_rels/presentation.xml.rels")?;
        let rels = parse_rels(rels).ok()?;
        let doc = roxmltree::Document::parse(std::str::from_utf8(pres).ok()?).ok()?;
        let mut out = Vec::new();
        for sld in doc.descendants().filter(|n| has_tag(*n, NS_P, "sldId")) {
            let rid = sld.attribute((NS_R, "id"))?;
            let target = rels.iter().find(|r| r.id == rid).map(|r| r.target.clone())?;
            out.push(resolve_target("ppt", &target));
        }
        if out.is_empty() { None } else { Some(out) }
    })();
    ordered.unwrap_or_else(|| {
        let mut names: Vec<String> = z.entries.iter()
            .filter(|(n, _)| n.starts_with("ppt/slides/slide") && n.ends_with(".xml"))
            .map(|(n, _)| n.clone())
            .collect();
        names.sort_by_key(|n| n.split("slide").last().and_then(|s| s.trim_end_matches(".xml").parse::<u32>().ok()).unwrap_or(0));
        names
    })
}

pub fn analyze(z: &Zip) -> Result<Analysis, String> {
    let (slide_w, slide_h) = z.get("ppt/presentation.xml")
        .and_then(|x| std::str::from_utf8(x).ok())
        .and_then(|s| roxmltree::Document::parse(s).ok())
        .and_then(|doc| {
            doc.descendants().find(|n| has_tag(*n, NS_P, "sldSz")).map(|sz| {
                (sz.attribute("cx").and_then(|v| v.parse().ok()).unwrap_or(0),
                 sz.attribute("cy").and_then(|v| v.parse().ok()).unwrap_or(0))
            })
        })
        .unwrap_or((0, 0));

    let mut slides = Vec::new();
    for slide_part in slide_parts(z) {
        let xml = z.get(&slide_part).ok_or_else(|| format!("missing {slide_part}"))?;
        let doc = match roxmltree::Document::parse(std::str::from_utf8(xml).map_err(|e| e.to_string())?) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let rels_part = {
            let base = slide_part.rsplit_once('/').unwrap_or(("", &slide_part));
            format!("{}/_rels/{}.rels", base.0, base.1)
        };
        let slide_dir = slide_part.rsplit_once('/').map(|(d, _)| d).unwrap_or(".");
        let rels = z.get(&rels_part)
            .and_then(|x| std::str::from_utf8(x).ok())
            .and_then(|s| parse_rels(s.as_bytes()).ok())
            .unwrap_or_default()
            .into_iter()
            .map(|r| Rel { target: resolve_target(slide_dir, &r.target), ..r })
            .collect::<Vec<_>>();
        let part_of = |rid: &str| rels.iter().find(|r| r.id == rid).map(|r| r.target.clone());

        let root = doc.root();
        let mut equations = Vec::new();
        let mut pics = Vec::new();
        let mut snapshots = Vec::new();
        let mut other_ole = false;

        for gf in root.descendants().filter(|n| has_tag(*n, NS_P, "graphicFrame")) {
            match equation_frame(gf) {
                Some((has_blip, rect)) if has_blip => {
                    if let Some(rid) = rid_embed(gf) {
                        equations.push(EqJob { rid, rect });
                    }
                }
                Some(_) => other_ole = true,
                None => {
                    if descendant(gf, NS_P, "oleObj").is_some() {
                        other_ole = true;
                    }
                }
            }
        }
        // oleObj outside graphicFrames (shape trees etc.)
        if root.descendants().any(|n| has_tag(n, NS_P, "oleObj") && !in_graphic_frame(n)) {
            other_ole = true;
        }

        for pic in root.descendants().filter(|n| has_tag(*n, NS_P, "pic") && !in_graphic_frame(*n)) {
            let Some(rid) = rid_embed(pic) else { continue };
            let Some(part) = part_of(&rid) else { continue };
            if !matches!(part_ext(&part).as_str(), "wmf" | "emf") {
                continue;
            }
            pics.push(PicJob { rid });
            if part_ext(&part) == "emf" && slide_w > 0 && slide_h > 0 {
                if let Some(ext) = pic.children().find(|c| has_tag(*c, NS_P, "spPr"))
                    .and_then(|sp| sp.children().find(|c| has_tag(*c, NS_A, "xfrm")))
                    .and_then(|x| x.children().find(|c| has_tag(*c, NS_A, "ext"))) {
                    let cx: i64 = ext.attribute("cx").and_then(|v| v.parse().ok()).unwrap_or(0);
                    let cy: i64 = ext.attribute("cy").and_then(|v| v.parse().ok()).unwrap_or(0);
                    if cx >= 9 * slide_w / 10 && cy >= 9 * slide_h / 10 {
                        if let Some(off) = pic.children().find(|c| has_tag(*c, NS_P, "spPr"))
                            .and_then(|sp| sp.children().find(|c| has_tag(*c, NS_A, "xfrm")))
                            .and_then(|x| x.children().find(|c| has_tag(*c, NS_A, "off"))) {
                            snapshots.push(Rect {
                                x: off.attribute("x").and_then(|v| v.parse().ok()).unwrap_or(0),
                                y: off.attribute("y").and_then(|v| v.parse().ok()).unwrap_or(0),
                                cx,
                                cy,
                            });
                        }
                    }
                }
            }
        }

        slides.push(SlidePlan {
            slide_part,
            rels_part,
            rels,
            equations,
            pics,
            snapshots,
            other_ole,
            new_rids: Vec::new(),
        });
    }
    Ok(Analysis { slide_w, slide_h, slides })
}

pub fn rect_in_snapshot(r: &Rect, snapshots: &[Rect]) -> bool {
    snapshots.iter().any(|s| {
        r.x >= s.x && r.y >= s.y && r.x + r.cx <= s.x + s.cx && r.y + r.cy <= s.y + s.cy
    })
}

/// is this AlternateContent a math fallback to hoist (used by both analyze and rewrite)
pub fn math_fallback_count(z: &Zip, slides: &[SlidePlan]) -> usize {
    slides.iter().filter_map(|s| z.get(&s.slide_part)).filter_map(|x| std::str::from_utf8(x).ok())
        .filter_map(|s| roxmltree::Document::parse(s).ok())
        .map(|doc| doc.descendants().filter(|n| has_tag(*n, NS_MC, "AlternateContent") && is_math_fallback(*n)).count())
        .sum()
}
