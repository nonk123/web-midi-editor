pub struct Project {
    pub name: String,
    pub time_signature: TimeSignature,
    pub bpm: u32,
    pub tracks: Vec<Track>,
}

pub struct TimeSignature {
    pub top: u32,
    pub bottom: u32,
}

pub struct Track {
    pub name: String,
    pub notes: Vec<Note>,
    pub instrument: u8,
}

pub struct Note {
    pub pitch: u8,
    pub velocity: u8,
    pub offset: u32,
    pub length: u32,
}
