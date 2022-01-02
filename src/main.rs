use js_sys::Array;
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use web_sys::{HtmlInputElement, HtmlSelectElement, MidiAccess, MidiOutput};
use yew::{
    events::{Event, InputEvent},
    prelude::*,
};

mod project;
mod util;

use crate::{
    project::{Project, TimeSignature},
    util::{select_get_value, time_signature_options},
};

enum Msg {
    GrantMidiAccess(MidiAccess),
    RefuseMidiAccess,
    SetOutputDevice(MidiOutput),
    SetBpm(u32),
    SetTimeSignatureTop(u32),
    SetTimeSignatureBottom(u32),
}

struct Model {
    midi_access: Option<MidiAccess>,
    selected_output: Option<MidiOutput>,
    project: Project,
    _success_closure: Closure<dyn FnMut(JsValue)>,
    _fail_closure: Closure<dyn FnMut(JsValue)>,
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let window = web_sys::window().expect("window unavailable");
        let navigator = window.navigator();

        let link = ctx.link().clone();

        let success = Closure::wrap(Box::new(move |midi_access: JsValue| {
            let midi_access = midi_access
                .dyn_into::<MidiAccess>()
                .expect("dyn_into::<MidiAccess>");

            link.send_message(Msg::GrantMidiAccess(midi_access));
        }) as Box<dyn FnMut(JsValue)>);

        let link = ctx.link().clone();

        let fail = Closure::wrap(Box::new(move |_error: JsValue| {
            link.send_message(Msg::RefuseMidiAccess);
        }) as Box<dyn FnMut(JsValue)>);

        navigator
            .request_midi_access()
            .expect("request_midi_access")
            .then2(&success, &fail);

        let project = Project {
            name: "My Project".to_string(),
            time_signature: TimeSignature { top: 4, bottom: 4 },
            bpm: 120,
            tracks: Vec::new(),
        };

        Self {
            midi_access: None,
            selected_output: None,
            project,
            _success_closure: success,
            _fail_closure: fail,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::GrantMidiAccess(midi_access) => {
                self.midi_access = Some(midi_access);

                self.selected_output = {
                    let output_devices = self.get_output_devices();

                    if output_devices.is_empty() {
                        None
                    } else {
                        Some(output_devices[0].clone())
                    }
                };

                true
            }
            Msg::RefuseMidiAccess => {
                self.midi_access = None;
                self.selected_output = None;

                true
            }
            Msg::SetOutputDevice(output) => {
                self.selected_output = Some(output);
                self.play_note(60, 1000);

                true
            }
            Msg::SetBpm(bpm) => {
                self.project.bpm = bpm;

                true
            }
            Msg::SetTimeSignatureTop(top) => {
                self.project.time_signature.top = top;

                true
            }
            Msg::SetTimeSignatureBottom(bottom) => {
                self.project.time_signature.bottom = bottom;

                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        if self.midi_access.is_some() {
            html! {
                <>
                    { self.view_top_bar(ctx) }
                    <div id="main-view">{ "Hello there!" }</div>
                </>
            }
        } else {
            self.view_no_midi()
        }
    }
}

impl Model {
    fn get_output_devices(&self) -> Vec<MidiOutput> {
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

    fn view_no_midi(&self) -> Html {
        html! {
            <p class="error">
                { "This app requires MIDI permissions to work" }
            </p>
        }
    }

    fn view_top_bar(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div id="top-bar">
                <div class="h-box">
                    { self.view_bpm(ctx) }
                    { self.view_time_signature(ctx) }
                    { self.view_output_selection(ctx) }
                </div>
            </div>
        }
    }

    fn view_bpm(&self, ctx: &Context<Self>) -> Html {
        let oninput = ctx.link().batch_callback(|event: InputEvent| {
            event
                .target_dyn_into::<HtmlInputElement>()
                .and_then(|input| {
                    let parse = input.value().parse().ok();
                    parse.and_then(|bpm| Some(Msg::SetBpm(bpm)))
                })
        });

        html! {
            <div class="v-box frame">
                { "BPM" }
                <input type={ "number" } value={ self.project.bpm.to_string() }
                       min={ "1" } max={ "5000" } size={ "5" } { oninput }/>
            </div>
        }
    }

    fn view_time_signature(&self, ctx: &Context<Self>) -> Html {
        let top_values = time_signature_options(&[2u32, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);

        let on_top_change = ctx.link().batch_callback(|event: Event| {
            select_get_value(event)
                .and_then(|top| Some(Msg::SetTimeSignatureTop(top.parse().unwrap())))
        });

        let bottom_values = time_signature_options(&[4u32, 8, 16]);

        let on_bottom_change = ctx.link().batch_callback(|event: Event| {
            select_get_value(event)
                .and_then(|bottom| Some(Msg::SetTimeSignatureBottom(bottom.parse().unwrap())))
        });

        html! {
            <div class="v-box frame">
                { "Time Signature" }
                <div class="h-box">
                    <select required=true onchange={ on_top_change }>
                        { for top_values }
                    </select>
                    { "/" }
                    <select required=true onchange={ on_bottom_change }>
                        { for bottom_values }
                    </select>
                </div>
            </div>
        }
    }

    fn view_output_selection(&self, ctx: &Context<Self>) -> Html {
        let output_devices = self.get_output_devices();

        let device_options = output_devices.iter().map(|output| {
            let selected = {
                let selected_output_name = self
                    .selected_output
                    .as_ref()
                    .and_then(|selected_output| selected_output.name());

                output.name() == selected_output_name
            };

            html! {
                <option value={ output.name() } { selected }>
                    { output.name().unwrap_or("No Name".to_string()) }
                </option>
            }
        });

        let output_devices_copy = output_devices.clone();

        let onchange = ctx.link().batch_callback(move |event: Event| {
            let target = event.target();

            let select = target.and_then(|target| target.dyn_into::<HtmlSelectElement>().ok());

            let output_device_name = match select {
                Some(select) => select.value().to_string(),
                None => return None,
            };

            let mut output_devices = output_devices_copy.clone();

            output_devices.retain(|output| {
                let name = output.name();

                if let Some(name) = name {
                    name.to_string() == output_device_name
                } else {
                    false
                }
            });

            if output_devices.is_empty() {
                None
            } else {
                let output_device = output_devices[0].clone();
                Some(Msg::SetOutputDevice(output_device))
            }
        });

        html! {
            <div class="v-box frame">
                { "MIDI Output" }
                <select required=true { onchange }>
                    { for device_options }
                </select>
            </div>
        }
    }

    fn play_note(&self, note: u8, duration: u32) {
        let note = JsValue::from_f64(note as f64);
        let full_velocity = JsValue::from_f64(0x7f as f64);

        let note_on_message = Array::of3(&JsValue::from_f64(0x90 as f64), &note, &full_velocity);
        let note_off_message = Array::of3(&JsValue::from_f64(0x80 as f64), &note, &full_velocity);

        let output = self.selected_output.as_ref().unwrap();

        output.send(&note_on_message).ok();

        output
            .send_with_timestamp(&note_off_message, duration as f64)
            .ok();
    }
}

fn main() {
    yew::start_app::<Model>();
}
