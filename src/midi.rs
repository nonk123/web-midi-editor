use js_sys::Array;
use wasm_bindgen::JsValue;

use crate::project::Project;

pub struct MidiMessage {
    // Offset in whole notes.
    pub offset: f64,
    pub type_: MidiMessageType,
}

pub enum MidiMessageType {
    ChangeInstrument(u8),
    NoteOn(u8, u8),
    NoteOff(u8, u8),
}

impl MidiMessageType {
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Self::ChangeInstrument(instrument) => vec![0xC0, *instrument],
            Self::NoteOn(pitch, velocity) => vec![0x90, *pitch, *velocity],
            Self::NoteOff(pitch, velocity) => vec![0x80, *pitch, *velocity],
        }
    }

    pub fn to_array(&self) -> Array {
        let bytes = self.to_bytes();

        let array = Array::new_with_length(bytes.len() as _);

        for (i, byte) in bytes.iter().enumerate() {
            let byte = JsValue::from_f64(*byte as _);
            array.set(i as _, byte);
        }

        array
    }
}

impl Project {
    pub fn to_midi(&self) -> Vec<MidiMessage> {
        let mut messages = Vec::new();

        let full_velocity = 0x7f;

        for track in &self.tracks {
            for note in &track.notes {
                messages.push(MidiMessage {
                    offset: note.offset,
                    type_: MidiMessageType::ChangeInstrument(track.instrument),
                });

                messages.push(MidiMessage {
                    offset: note.offset,
                    type_: MidiMessageType::NoteOn(note.pitch, full_velocity),
                });

                messages.push(MidiMessage {
                    offset: note.offset + note.length,
                    type_: MidiMessageType::NoteOff(note.pitch, full_velocity),
                });
            }
        }

        messages.sort_by(|a, b| a.offset.partial_cmp(&b.offset).unwrap());

        messages
    }
}

fn to_varlen(value: u32) -> Vec<u8> {
    let mut bytes = vec![(value & 0x7f) as u8];

    let mut number: u32 = value >> 7;

    while number != 0 {
        bytes.insert(0, (number & 0x7f | 0x80) as u8);
        number >>= 7;
    }

    bytes
}

pub fn export_midi(project: &Project) -> Vec<u8> {
    let messages = project.to_midi();

    let mut bytes = Vec::new();

    for byte in "MThd".bytes() {
        bytes.push(byte);
    }

    for byte in 6u32.to_be_bytes() {
        bytes.push(byte);
    }

    for byte in 0u16.to_be_bytes() {
        bytes.push(byte);
    }

    for byte in 1u16.to_be_bytes() {
        bytes.push(byte);
    }

    let ticks_per_quarter_note = 1024u16;
    let delta_multiplier = (480.0 / project.bpm * ticks_per_quarter_note as f64).round() as u32;

    for byte in ticks_per_quarter_note.to_be_bytes() {
        bytes.push(byte);
    }

    for byte in "MTrk".bytes() {
        bytes.push(byte);
    }

    let mut track_bytes = Vec::new();

    let mut last_offset = 0.0;

    for message in messages {
        let delta = message.offset - last_offset;
        last_offset = message.offset;

        let delta = delta * delta_multiplier as f64;
        track_bytes.append(&mut to_varlen(delta as _));

        track_bytes.append(&mut message.type_.to_bytes());
    }

    for byte in (track_bytes.len() as u32).to_be_bytes() {
        bytes.push(byte);
    }

    track_bytes.push(0);
    track_bytes.push(0xFF);
    track_bytes.push(0x2F);
    track_bytes.push(0);

    bytes.append(&mut track_bytes);

    bytes
}
