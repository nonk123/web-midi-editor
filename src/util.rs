use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlSelectElement};
use yew::prelude::*;

use crate::project::{MIN_INTERVAL, NOTE_RECT_HEIGHT, WHOLE_NOTE_WIDTH};

pub fn time_signature_options(values: &[u32]) -> Vec<Html> {
    values
        .iter()
        .map(|x| {
            html! {
                <option value={ x.to_string() } selected={ x == &4 }>
                    { x.to_string() }
                </option>
            }
        })
        .collect()
}

pub fn select_get_value(event: Event) -> Option<String> {
    let target = event.target();
    let select = target.and_then(|target| target.dyn_into::<HtmlSelectElement>().ok());

    select.and_then(|select| Some(select.value()))
}

pub fn note_name(midi_note: u8) -> String {
    let notes = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];

    format!(
        "{}{}",
        notes[midi_note as usize % 12],
        (midi_note / 12) as i8 - 1
    )
}

pub fn snap(x: f64, precision: f64) -> f64 {
    let f_mod = x % precision;

    if f_mod >= precision / 2.0 {
        x - f_mod + precision
    } else {
        x - f_mod
    }
}

pub fn relative_mouse_pos(event: &MouseEvent) -> (f64, f64) {
    event
        .target_dyn_into::<Element>()
        .map(|target| {
            let rect = target.get_bounding_client_rect();

            let mouse_x = event.client_x() as f64 - rect.left();
            let mouse_y = event.client_y() as f64 - rect.top();

            (mouse_x, mouse_y)
        })
        .unwrap_or((0.0, 0.0))
}

pub fn mouse_x_to_interval(mouse_x: f64) -> f64 {
    snap(mouse_x / WHOLE_NOTE_WIDTH, MIN_INTERVAL)
}

pub fn mouse_y_to_pitch(mouse_y: f64) -> u8 {
    let pitch = 127.0 - mouse_y / NOTE_RECT_HEIGHT;
    pitch.clamp(0.0, 127.0).ceil() as u8
}
