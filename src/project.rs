pub const WHOLE_NOTE_WIDTH: f64 = 320.0;
pub const NOTE_RECT_HEIGHT: f64 = 30.0;
pub const NOTE_EDGE_WIDTH: f64 = 6.0;

pub const MIN_DIVISION: u32 = 16;
pub const MIN_INTERVAL: f64 = 1.0 / MIN_DIVISION as f64;

#[derive(Clone)]
pub struct Project {
    pub name: String,
    pub time_signature: TimeSignature,
    pub bpm: u32,
    pub tracks: Vec<Track>,
}

#[derive(Clone)]
pub struct TimeSignature {
    pub top: u32,
    pub bottom: u32,
}

#[derive(Clone)]
pub struct Track {
    pub name: String,
    pub notes: Vec<Note>,
    pub instrument: u8,
}

impl Track {
    pub fn get_note_at_position(&self, x: f64, y: f64) -> Option<usize> {
        let mut result = None;

        for (index, note) in self.notes.iter().enumerate() {
            let epsilon = 1e-3;

            let note_x = note.screen_x();
            let note_y = note.screen_y();
            let note_w = note.screen_width();
            let note_h = note.screen_height();

            let x = x - note_x;
            let y = y - note_y;

            if x >= -epsilon && y >= -epsilon && x < note_w + epsilon && y < note_h + epsilon {
                result = Some(index);
            }
        }

        result
    }
}

#[derive(Clone)]
pub struct Note {
    pub pitch: u8,
    pub velocity: u8,
    /// Offset in whole notes.
    pub offset: f64,
    /// Length in whole notes.
    pub length: f64,
}

impl Note {
    pub fn screen_x(&self) -> f64 {
        self.offset * WHOLE_NOTE_WIDTH
    }

    pub fn screen_y(&self) -> f64 {
        (127 - self.pitch) as f64 * NOTE_RECT_HEIGHT
    }

    pub fn screen_width(&self) -> f64 {
        self.length * WHOLE_NOTE_WIDTH
    }

    pub fn screen_height(&self) -> f64 {
        NOTE_RECT_HEIGHT
    }

    pub fn right_edge(&self) -> f64 {
        self.screen_x() + self.screen_width()
    }

    pub fn bottom_edge(&self) -> f64 {
        self.screen_y() + self.screen_height()
    }
}
