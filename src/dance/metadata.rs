use crate::dance::Beat;

#[derive(Debug, Clone)]
pub struct Metadata {
    pub title: String,
    pub bpm: f64,
    pub beats_per_bar: u8,
    pub stage_size: StageSize,
}

impl Metadata {
    pub fn beat_to_time(&self, beat: &Beat) -> f64 {
        let beat_length = 60.0 / self.bpm;
        let bar_beats = self.beats_per_bar as f64 * beat.0 as f64;

        (bar_beats + beat.1) * beat_length
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StageSize {
    pub width: u64,
    pub depth: u64,
}
//
// #[derive(Debug, Clone, Copy)]
// pub struct TimeSignature(pub u8, pub u8);
