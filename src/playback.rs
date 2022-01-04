use std::collections::HashMap;

use gloo_timers::callback::Timeout;
use js_sys::Array;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::MidiOutput;
use yew::prelude::*;

use crate::{project::Note, Model, Msg};

impl Model {
    pub fn get_output_devices(&self) -> Vec<MidiOutput> {
        let mut output_devices = Vec::new();

        let midi_access = match &self.midi_access {
            Some(midi_access) => midi_access,
            None => return output_devices,
        };

        let iter = js_sys::try_iter(midi_access.outputs().as_ref())
            .expect("try_iter(midi_access)")
            .expect("midi_access (view_top_bar)");

        for entry in iter {
            match entry {
                Err(_) => break,
                Ok(entry) => {
                    let array: Array = entry.dyn_into().expect("dyn_into (array)");

                    let output = array
                        .get(1)
                        .dyn_into::<MidiOutput>()
                        .expect("dyn_into::<MidiOutput>");

                    output_devices.push(output);
                }
            }
        }

        output_devices
    }

    pub fn play(&mut self, ctx: &Context<Self>) {
        if self.selected_output.is_none() {
            return;
        }

        let selected_output = self.selected_output.clone().unwrap();

        for pitch in 0..=127 {
            self.stop_midi_note(pitch, None);
        }

        let was_empty = self.playing_notes.is_empty();

        loop {
            match self.playing_notes.pop() {
                None => break,
                Some(timeout) => {
                    timeout.cancel();
                }
            }
        }

        if !was_empty {
            return;
        }

        let whole_note_duration = 240.0 / self.project.bpm;

        let all_notes = self
            .project
            .tracks
            .iter()
            .flat_map(|track| track.notes.clone())
            .collect::<Vec<Note>>();

        let mut notes_by_offset_ms = HashMap::<u32, Vec<Note>>::new();
        let mut note_ends = HashMap::<u32, Vec<Note>>::new();

        for note in all_notes {
            if note.offset >= self.play_offset - 1e-5 {
                let offset_ms =
                    ((note.offset - self.play_offset) * whole_note_duration * 1000.0) as u32;

                if let Some(notes) = notes_by_offset_ms.get_mut(&offset_ms) {
                    notes.push(note.clone());
                } else {
                    notes_by_offset_ms.insert(offset_ms, vec![note.clone()]);
                }

                let note_length_ms = (note.length * whole_note_duration * 1000.0) as u32;
                let stop_timeout_ms = offset_ms + note_length_ms;

                if let Some(notes) = note_ends.get_mut(&stop_timeout_ms) {
                    notes.push(note.clone());
                } else {
                    note_ends.insert(stop_timeout_ms, vec![note.clone()]);
                }
            }
        }

        for (offset_ms, notes) in notes_by_offset_ms {
            let output = selected_output.clone();
            let link = ctx.link().clone();

            let opcode = JsValue::from_f64(0x90 as _);
            let full_velocity = JsValue::from_f64(0x7f as _);

            let notes_clone = notes.clone();
            let progress = notes[0].offset;

            self.playing_notes.push(Timeout::new(offset_ms, move || {
                for note in notes_clone {
                    let pitch = JsValue::from_f64(note.pitch as _);
                    let message = Array::of3(&opcode, &pitch, &full_velocity);
                    output.send(&message).ok();
                }

                link.send_message(Msg::SetPlayProgress(progress));
            }));
        }

        for (offset, notes) in note_ends {
            let output = selected_output.clone();
            let link = ctx.link().clone();

            let opcode = JsValue::from_f64(0x80 as _);
            let full_velocity = JsValue::from_f64(0x7f as _);

            let end_safety = 6;
            let end_timeout = offset.max(end_safety) - end_safety;

            self.playing_notes.push(Timeout::new(end_timeout, move || {
                let progress = notes[0].offset + notes[0].length;

                for note in notes {
                    let pitch = JsValue::from_f64(note.pitch as _);
                    let message = Array::of3(&opcode, &pitch, &full_velocity);
                    output.send(&message).ok();
                }

                link.send_message(Msg::SetPlayProgress(progress));
            }));
        }
    }

    pub fn play_midi_note(&self, pitch: u8, duration: f64) {
        let full_velocity = JsValue::from_f64(0x7f as _);

        let message = Array::of3(
            &JsValue::from_f64(0x90 as _),
            &JsValue::from_f64(pitch as _),
            &full_velocity,
        );

        let output = self.selected_output.as_ref().unwrap();
        output.send(&message).ok();

        self.stop_midi_note(pitch, Some(duration));
    }

    pub fn stop_midi_note(&self, pitch: u8, timeout: Option<f64>) {
        let pitch = JsValue::from_f64(pitch as _);
        let full_velocity = JsValue::from_f64(0x7f as _);

        let message = Array::of3(&JsValue::from_f64(0x80 as _), &pitch, &full_velocity);

        let output = self.selected_output.as_ref().unwrap();

        if let Some(timeout) = timeout {
            output.send_with_timestamp(&message, timeout).ok();
        } else {
            output.send(&message).ok();
        }
    }
}
