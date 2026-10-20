#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Reading,
    Analyzing,
    RenderingMedia,
    Rewriting,
    Done,
}

impl Stage {
    pub fn label(self) -> &'static str {
        match self {
            Stage::Reading => "reading",
            Stage::Analyzing => "analyzing",
            Stage::RenderingMedia => "rendering media",
            Stage::Rewriting => "rewriting",
            Stage::Done => "done",
        }
    }
}

pub trait Progress {
    fn on_stage(&self, _stage: Stage) {}
    fn on_media(&self, _cur: usize, _total: usize, _name: &str) {}
}

pub struct NoProgress;
impl Progress for NoProgress {}
