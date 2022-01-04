use js_sys::Array;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::MidiOutput;

use crate::Model;

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

    pub fn play_note(&self, note: u8, duration: u32) {
        let note = JsValue::from_f64(note as _);
        let full_velocity = JsValue::from_f64(0x7f as _);

        let note_on_message = Array::of3(&JsValue::from_f64(0x90 as _), &note, &full_velocity);
        let note_off_message = Array::of3(&JsValue::from_f64(0x80 as _), &note, &full_velocity);

        let output = self.selected_output.as_ref().unwrap();

        output.send(&note_on_message).ok();

        output
            .send_with_timestamp(&note_off_message, duration as _)
            .ok();
    }
}
