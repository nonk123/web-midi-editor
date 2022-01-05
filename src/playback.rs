use gloo_timers::callback::Interval;
use js_sys::Array;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::MidiOutput;
use yew::prelude::*;

use crate::{
    project::{Note, MIN_INTERVAL},
    Model, Msg,
};

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
        let output = match self.selected_output.clone() {
            Some(output) => output,
            None => return,
        };

        for pitch in 0..=127 {
            self.stop_midi_note(pitch, None);
        }

        if self.tick_interval.is_some() {
            self.tick_interval.take().unwrap().cancel();
            ctx.link().send_message(Msg::SetPlayProgress(0.0));
            return;
        };

        let whole_note_duration = 240.0 / self.project.bpm;
        let min_interval_duration = whole_note_duration * MIN_INTERVAL;
        let tick_interval = (min_interval_duration * 1000.0) as u32;

        let link = ctx.link().clone();

        let mut local_offset = self.play_offset;

        let all_notes = self
            .project
            .tracks
            .iter()
            .flat_map(|track| track.notes.clone())
            .collect::<Vec<Note>>();

        self.tick_interval = Some(Interval::new(tick_interval, move || {
            let full_velocity = JsValue::from_f64(0x7f as _);

            let epsilon = 1e-5;

            for note in &all_notes {
                let start_offset = note.offset;
                let end_offset = start_offset + note.length;

                let opcode = {
                    if (local_offset - start_offset).abs() <= epsilon {
                        0x90
                    } else if (local_offset - end_offset).abs() <= epsilon {
                        0x80
                    } else {
                        continue;
                    }
                };

                let opcode = JsValue::from_f64(opcode as _);
                let pitch = JsValue::from_f64(note.pitch as _);

                let message = Array::of3(&opcode, &pitch, &full_velocity);

                output.send(&message).ok();
            }

            link.send_message(Msg::IncrementPlayProgress);
            local_offset += MIN_INTERVAL;
        }));
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
