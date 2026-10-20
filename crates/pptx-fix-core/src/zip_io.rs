use std::io::{Read, Write};

pub struct Zip {
    /// entry name -> contents, in archive order
    pub entries: Vec<(String, Vec<u8>)>,
}

impl Zip {
    pub fn read(data: &[u8]) -> Result<Self, String> {
        let mut r = zip::ZipArchive::new(std::io::Cursor::new(data))
            .map_err(|e| format!("open pptx: {e}"))?;
        let mut entries = Vec::with_capacity(r.len());
        for i in 0..r.len() {
            let mut f = r.by_index(i).map_err(|e| format!("entry {i}: {e}"))?;
            if f.is_dir() {
                continue;
            }
            let mut buf = Vec::with_capacity(f.size() as usize);
            f.read_to_end(&mut buf).map_err(|e| format!("read {}: {e}", f.name()))?;
            entries.push((f.name().to_string(), buf));
        }
        Ok(Zip { entries })
    }

    pub fn get(&self, name: &str) -> Option<&[u8]> {
        self.entries.iter().find(|(n, _)| n == name).map(|(_, b)| b.as_slice())
    }

    pub fn set(&mut self, name: &str, data: Vec<u8>) {
        if let Some(e) = self.entries.iter_mut().find(|(n, _)| n == name) {
            e.1 = data;
        } else {
            self.entries.push((name.to_string(), data));
        }
    }

    pub fn write(&self) -> Result<Vec<u8>, String> {
        let mut w = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let opts: zip::write::SimpleFileOptions = Default::default();
        for (name, data) in &self.entries {
            w.start_file(name.as_str(), opts).map_err(|e| format!("{name}: {e}"))?;
            w.write_all(data).map_err(|e| format!("{name}: {e}"))?;
        }
        let cursor = w.finish().map_err(|e| e.to_string())?;
        Ok(cursor.into_inner())
    }
}
