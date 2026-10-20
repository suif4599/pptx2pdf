use std::collections::HashMap;

use quick_xml::events::Event;
use quick_xml::{Reader, Writer};

use crate::analyze::{self, Rect, SlidePlan, NS_A, NS_MC, NS_P, NS_R};

#[derive(Default, Debug)]
pub struct Stats {
    pub math_fallbacks: usize,
    pub replaced: usize,
    pub skipped_on_snapshots: usize,
    pub retargeted: usize,
}

struct NsMap {
    prefixes: Vec<(String, String)>,
}

impl NsMap {
    fn from_start(e: &quick_xml::events::BytesStart) -> Self {
        let mut prefixes = Vec::new();
        for a in e.attributes().with_checks(true).flatten() {
            let name = std::str::from_utf8(a.key.as_ref()).unwrap_or("");
            if let Some(pfx) = name.strip_prefix("xmlns:") {
                prefixes.push((pfx.to_string(), String::from_utf8_lossy(&a.value).to_string()));
            }
        }
        NsMap { prefixes }
    }

    fn uri(&self, prefix: &str) -> Option<&str> {
        self.prefixes.iter().find(|(p, _)| p == prefix).map(|(_, u)| u.as_str())
    }

    fn prefix(&self, ns: &str) -> Option<&str> {
        self.prefixes.iter().find(|(_, u)| u == ns).map(|(p, _)| p.as_str())
    }

    fn is_tag(&self, qname: &str, ns: &str, local: &str) -> bool {
        let (pfx, lcl) = qname.split_once(':').unwrap_or(("", qname));
        lcl == local && self.uri(pfx) == Some(ns)
    }

    fn merge_start(&mut self, e: &quick_xml::events::BytesStart) {
        for a in e.attributes().with_checks(true).flatten() {
            let name = std::str::from_utf8(a.key.as_ref()).unwrap_or("");
            if let Some(pfx) = name.strip_prefix("xmlns:") {
                if self.uri(pfx).is_none() {
                    self.prefixes.push((pfx.to_string(), String::from_utf8_lossy(&a.value).to_string()));
                }
            }
        }
    }

    fn wrap(&self, fragment: &str) -> String {
        let decl: String = self.prefixes.iter()
            .map(|(p, u)| format!(" xmlns:{p}=\"{u}\""))
            .collect();
        format!("<w{decl}>{fragment}</w>")
    }
}

fn with_fragment<T>(map: &NsMap, fragment: &str, f: impl FnOnce(roxmltree::Node) -> T) -> Option<T> {
    let wrapped = map.wrap(fragment);
    let doc = roxmltree::Document::parse(&wrapped).ok()?;
    let root = doc.root_element();
    let node = root.children().find(|c| c.is_element())?;
    Some(f(node))
}

fn blip_rid(map: &NsMap, node: roxmltree::Node) -> Option<String> {
    let blip = node.descendants().find(|d| analyze::has_tag(*d, NS_A, "blip"))?;
    if let Some(v) = blip.attribute((NS_R, "embed")) {
        return Some(v.to_string());
    }
    let pfx = map.prefix(NS_R)?;
    blip.attribute(format!("{pfx}:embed").as_str()).map(|s| s.to_string())
}

fn is_equation_frame(node: roxmltree::Node) -> bool {
    node.descendants().any(|d| {
        analyze::has_tag(d, NS_P, "oleObj")
            && d.attribute("progId").is_some_and(|p| p.starts_with("Equation"))
    })
}

fn frame_rect(gf: roxmltree::Node) -> Rect {
    let pick = |local: &str, attr: &str| -> i64 {
        gf.children().find(|c| analyze::has_tag(*c, NS_P, "xfrm"))
            .and_then(|x| x.children().find(|c| analyze::has_tag(*c, NS_A, local)))
            .and_then(|n| n.attribute(attr))
            .and_then(|v| v.parse().ok())
            .unwrap_or(0)
    };
    Rect {
        x: pick("off", "x"),
        y: pick("off", "y"),
        cx: pick("ext", "cx"),
        cy: pick("ext", "cy"),
    }
}

fn pic_xml(rid: &str, r: &Rect, id: u32, name: &str) -> String {
    format!(
        "<p:pic><p:nvPicPr><p:cNvPr id=\"{id}\" name=\"{name}\"/><p:cNvPicPr><a:picLocks noChangeAspect=\"1\"/></p:cNvPicPr><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed=\"{rid}\"/><a:stretch><a:fillRect/></a:stretch></p:blipFill><p:spPr><a:xfrm><a:off x=\"{}\" y=\"{}\"/><a:ext cx=\"{}\" cy=\"{}\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></p:spPr></p:pic>",
        r.x, r.y, r.cx, r.cy
    )
}

fn buffer_subtree(reader: &mut Reader<&[u8]>, start: Event) -> Result<String, String> {
    let mut buf = Vec::new();
    let mut w = Writer::new(&mut buf);
    w.write_event(start).map_err(|e| e.to_string())?;
    let mut depth = 1usize;
    loop {
        let mut ev = Vec::new();
        let e = reader.read_event_into(&mut ev).map_err(|e| e.to_string())?;
        match &e {
            Event::Start(_) => depth += 1,
            Event::End(_) => depth -= 1,
            Event::Eof => return Err("unexpected eof".into()),
            _ => {}
        }
        w.write_event(e).map_err(|e| e.to_string())?;
        if depth == 0 {
            return Ok(String::from_utf8_lossy(&buf).to_string());
        }
    }
}

fn fallback_inner(map: &NsMap, frag: &str) -> Option<String> {
    let pfx = map.prefix(NS_MC)?;
    let open = format!("<{pfx}:Fallback");
    let start = frag.find(&open).map(|i| i + open.len())?;
    let gt = frag[start..].find('>').map(|i| start + i + 1)?;
    let close = format!("</{pfx}:Fallback>");
    let end = frag[gt..].rfind(&close).map(|i| gt + i)?;
    Some(frag[gt..end].to_string())
}

fn push_event(out: &mut Vec<u8>, e: Event) -> Result<(), String> {
    let mut w = Writer::new(out);
    w.write_event(e).map_err(|e| e.to_string())
}

pub fn unwrap_math_fallbacks(xml: &[u8]) -> Result<(Vec<u8>, usize), String> {
    let mut reader = Reader::from_reader(xml);
    let mut out = Vec::new();
    let mut nsmap = NsMap { prefixes: vec![
        ("p".into(), NS_P.into()),
        ("a".into(), NS_A.into()),
        ("r".into(), NS_R.into()),
        ("mc".into(), NS_MC.into()),
    ] };
    let mut count = 0usize;
    let mut first = true;
    loop {
        let mut ev = Vec::new();
        let e = reader.read_event_into(&mut ev).map_err(|e| e.to_string())?;
        if let Event::Start(s) = &e {
            if first {
                nsmap = NsMap::from_start(s);
                first = false;
            } else {
                nsmap.merge_start(s);
            }
            let name = String::from_utf8_lossy(s.name().as_ref()).to_string();
            if nsmap.is_tag(&name, NS_MC, "AlternateContent") {
                let frag = buffer_subtree(&mut reader, e.clone())?;
                let is_math = with_fragment(&nsmap, &frag, analyze::is_math_fallback).unwrap_or(false);
                if is_math {
                    if let Some(inner) = fallback_inner(&nsmap, &frag) {
                        out.extend_from_slice(inner.as_bytes());
                        count += 1;
                        continue;
                    }
                }
                out.extend_from_slice(frag.as_bytes());
                continue;
            }
        }
        if let Event::Eof = e {
            break;
        }
        push_event(&mut out, e)?;
    }
    Ok((out, count))
}

fn replace_and_retarget(xml: &[u8], plan: &SlidePlan, stats: &mut Stats) -> Result<Vec<u8>, String> {
    let mut reader = Reader::from_reader(xml);
    let mut out = Vec::new();
    let mut nsmap = NsMap { prefixes: vec![
        ("p".into(), NS_P.into()),
        ("a".into(), NS_A.into()),
        ("r".into(), NS_R.into()),
        ("mc".into(), NS_MC.into()),
    ] };
    let mut first = true;
    let mut pic_id = 9000u32;
    let new_rid = |old: &str| -> Option<String> {
        plan.new_rids.iter().find(|(o, _)| o == old).map(|(_, n)| n.clone())
    };
    loop {
        let mut ev = Vec::new();
        let e = reader.read_event_into(&mut ev).map_err(|e| e.to_string())?;
        if let Event::Start(s) = &e {
            if first {
                nsmap = NsMap::from_start(s);
                first = false;
            } else {
                nsmap.merge_start(s);
            }
            let name = String::from_utf8_lossy(s.name().as_ref()).to_string();
            if nsmap.is_tag(&name, NS_P, "graphicFrame") {
                let frag = buffer_subtree(&mut reader, e.clone())?;
                let is_eq = with_fragment(&nsmap, &frag, is_equation_frame).unwrap_or(false);
                let mut handled = false;
                if is_eq {
                    let rid = with_fragment(&nsmap, &frag, |n| blip_rid(&nsmap, n)).flatten();
                    if let Some(rid) = rid {
                        if let Some(new) = new_rid(&rid) {
                            with_fragment(&nsmap, &frag, |node| {
                                let rect = frame_rect(node);
                                if analyze::rect_in_snapshot(&rect, &plan.snapshots) {
                                    stats.skipped_on_snapshots += 1;
                                } else {
                                    let part = plan.rels.iter().find(|r| r.id == rid)
                                        .map(|r| r.target.clone()).unwrap_or_default();
                                    let pname = part.rsplit('/').next().unwrap_or("image");
                                    out.extend_from_slice(
                                        pic_xml(&new, &rect, pic_id, &format!("fix-{pname}")).as_bytes(),
                                    );
                                    pic_id += 1;
                                    stats.replaced += 1;
                                }
                            });
                            handled = true;
                        }
                    }
                }
                if !handled {
                    out.extend_from_slice(frag.as_bytes());
                }
                continue;
            }
            if nsmap.is_tag(&name, NS_P, "pic") {
                let frag = buffer_subtree(&mut reader, e.clone())?;
                let rid = with_fragment(&nsmap, &frag, |n| blip_rid(&nsmap, n)).flatten();
                let mut handled = false;
                if let Some(rid) = rid {
                    if let Some(new) = new_rid(&rid) {
                        let needle = format!("embed=\"{rid}\"");
                        if let Some(pos) = frag.find(&needle) {
                            let end = pos + needle.len();
                            let replaced = format!("{}embed=\"{new}\"{}", &frag[..pos], &frag[end..]);
                            out.extend_from_slice(replaced.as_bytes());
                            stats.retargeted += 1;
                            handled = true;
                        }
                    }
                }
                if !handled {
                    out.extend_from_slice(frag.as_bytes());
                }
                continue;
            }
        }
        if let Event::Eof = e {
            break;
        }
        push_event(&mut out, e)?;
    }
    Ok(out)
}

pub fn rewrite_slide(xml: &[u8], plan: &SlidePlan, stats: &mut Stats) -> Result<Vec<u8>, String> {
    let (pass_a, n) = unwrap_math_fallbacks(xml)?;
    stats.math_fallbacks += n;
    replace_and_retarget(&pass_a, plan, stats)
}

pub fn remap_fonts(xml: &[u8], map: &HashMap<String, String>) -> Option<Vec<u8>> {
    let mut s = std::str::from_utf8(xml).ok()?.to_string();
    let mut changed = false;
    for (from, to) in map {
        let needle = format!("typeface=\"{from}\"");
        if s.contains(&needle) {
            s = s.replace(&needle, &format!("typeface=\"{to}\""));
            changed = true;
        }
    }
    changed.then(|| s.into_bytes())
}
