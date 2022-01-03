use js_sys::Array;
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use web_sys::{HtmlElement, HtmlInputElement, HtmlSelectElement, MidiAccess, MidiOutput};
use yew::{
    events::{Event, InputEvent, MouseEvent},
    prelude::*,
};

mod project;
mod util;

use project::{
    Note, Project, TimeSignature, Track, MIN_DIVISION, MIN_INTERVAL, NOTE_EDGE_WIDTH,
    NOTE_RECT_HEIGHT, WHOLE_NOTE_WIDTH,
};
use util::{note_name, relative_mouse_pos, select_get_value, snap, time_signature_options};

enum Msg {
    MidiAccessGranted(MidiAccess),
    MidiAccessRefused,
    SetOutputDevice(MidiOutput),
    SelectTrack(usize),
    DeselectTrack,
    CreateTrack,
    DeleteSelectedTrack,
    RenameSelectedTrack(String),
    SetSelectedTrackInstrument(u8),
    SetProjectName(String),
    SetBpm(u32),
    SetTimeSignatureTop(u32),
    SetTimeSignatureBottom(u32),
    PianoRollMouseDown(MouseEvent),
    PianoRollMouseUp,
    PianoRollMouseMove(MouseEvent),
    PlaySingleNote(u8),
}

struct Model {
    midi_access: Option<MidiAccess>,
    selected_output: Option<MidiOutput>,
    project: Project,
    selected_track_index: Option<usize>,
    note_operation: Option<NoteOperation>,
    piano_roll_area: NodeRef,
    last_placed_note_length: f64,
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

            link.send_message(Msg::MidiAccessGranted(midi_access));
        }) as Box<dyn FnMut(JsValue)>);

        let link = ctx.link().clone();

        let fail = Closure::wrap(Box::new(move |_error: JsValue| {
            link.send_message(Msg::MidiAccessRefused);
        }) as Box<dyn FnMut(JsValue)>);

        let _ = navigator
            .request_midi_access()
            .expect("request_midi_access")
            .then2(&success, &fail);

        let project = Project {
            name: "Untitled".to_string(),
            time_signature: TimeSignature { top: 4, bottom: 4 },
            bpm: 120,
            tracks: Vec::new(),
        };

        Self {
            midi_access: None,
            selected_output: None,
            project,
            selected_track_index: None,
            note_operation: None,
            piano_roll_area: NodeRef::default(),
            last_placed_note_length: 1.0 / 8.0,
            _success_closure: success,
            _fail_closure: fail,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::MidiAccessGranted(midi_access) => {
                self.midi_access = Some(midi_access);

                self.selected_output = {
                    let output_devices = self.get_output_devices();

                    if output_devices.is_empty() {
                        None
                    } else {
                        Some(output_devices[0].clone())
                    }
                };

                ctx.link().send_message(Msg::CreateTrack);

                true
            }
            Msg::MidiAccessRefused => {
                self.midi_access = None;
                self.selected_output = None;

                true
            }
            Msg::SetOutputDevice(output) => {
                self.selected_output = Some(output);
                self.play_note(60, 1000);

                true
            }
            Msg::SelectTrack(index) => {
                self.selected_track_index = Some(index);
                true
            }
            Msg::DeselectTrack => {
                self.selected_track_index = None;
                true
            }
            Msg::CreateTrack => {
                let len = self.project.tracks.len();

                self.project.tracks.push(Track {
                    name: format!("Track {}", len + 1),
                    notes: Vec::new(),
                    instrument: 0,
                });

                ctx.link().send_message(Msg::SelectTrack(len));

                true
            }
            Msg::DeleteSelectedTrack => {
                if let Some(index) = self.selected_track_index {
                    self.project.tracks.remove(index);

                    if self.project.tracks.len() >= 1 {
                        let index = if index == 0 { 0 } else { index - 1 };
                        ctx.link().send_message(Msg::SelectTrack(index));
                    } else {
                        ctx.link().send_message(Msg::DeselectTrack);
                    }

                    true
                } else {
                    false
                }
            }
            Msg::RenameSelectedTrack(name) => {
                self.act_on_selected_track_mut(|track| track.name = name.to_string());
                true
            }
            Msg::SetSelectedTrackInstrument(instrument) => {
                self.act_on_selected_track_mut(|track| track.instrument = instrument);
                true
            }
            Msg::SetProjectName(name) => {
                self.project.name = name.to_string();
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
            Msg::PianoRollMouseDown(event) => {
                if self.note_operation.is_some() {
                    false
                } else if let Some(index) = self.selected_track_index {
                    let track = &mut self.project.tracks[index];
                    let (mouse_x, mouse_y) = relative_mouse_pos(&event);

                    match event.buttons() {
                        1 => {
                            let existing_note_index = track.get_note_at_position(mouse_x, mouse_y);

                            if let Some(index) = existing_note_index {
                                let note = &track.notes[index];

                                self.note_operation = Some(NoteOperation {
                                    note_index: index,
                                    type_: {
                                        if mouse_x <= note.screen_x() + NOTE_EDGE_WIDTH {
                                            NoteOperationType::DragLeftEdge
                                        } else if mouse_x >= note.right_edge() - NOTE_EDGE_WIDTH {
                                            NoteOperationType::DragRightEdge
                                        } else {
                                            NoteOperationType::Move
                                        }
                                    },
                                })
                            } else {
                                let len = track.notes.len();

                                let pitch = 127.0 - mouse_y / NOTE_RECT_HEIGHT;

                                track.notes.push(Note {
                                    pitch: pitch.clamp(0.0, 127.0).ceil() as u8,
                                    velocity: 127,
                                    offset: snap(mouse_x / WHOLE_NOTE_WIDTH, 1.0 / 16.0),
                                    length: self.last_placed_note_length,
                                });

                                self.note_operation = Some(NoteOperation {
                                    note_index: len,
                                    type_: NoteOperationType::Move,
                                });
                            }

                            true
                        }
                        2 => track.remove_note_at_position(mouse_x, mouse_y).is_some(),
                        _ => false,
                    }
                } else {
                    false
                }
            }
            Msg::PianoRollMouseUp => {
                self.note_operation = None;
                true
            }
            Msg::PianoRollMouseMove(event) => {
                let (mouse_x, mouse_y) = relative_mouse_pos(&event);

                match event.buttons() {
                    1 => {
                        if let Some(note_operation) = &self.note_operation {
                            if let Some(index) = self.selected_track_index {
                                let track = &mut self.project.tracks[index];
                                let note = &mut track.notes[note_operation.note_index];

                                let offset = snap(mouse_x / WHOLE_NOTE_WIDTH, MIN_INTERVAL);

                                let pitch = 127
                                    - (mouse_y / NOTE_RECT_HEIGHT - 0.5).round().clamp(0.0, 127.0)
                                        as u8;

                                match note_operation.type_ {
                                    NoteOperationType::Move => {
                                        note.offset = offset;
                                        note.pitch = pitch;
                                    }
                                    NoteOperationType::DragLeftEdge => {
                                        let offset =
                                            offset.min(note.offset + note.length - MIN_INTERVAL);

                                        note.length += note.offset - offset;
                                        note.offset = offset;
                                    }
                                    NoteOperationType::DragRightEdge => {
                                        let offset =
                                            offset.max(note.offset - note.length + MIN_INTERVAL);

                                        note.length = offset - note.offset + MIN_INTERVAL;
                                    }
                                }

                                if note.length < 1e-4 {
                                    note.length = MIN_INTERVAL;
                                }

                                self.last_placed_note_length = note.length;

                                true
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    }
                    2 => {
                        self.act_on_selected_track_mut(|track| {
                            track.remove_note_at_position(mouse_x, mouse_y)
                        });

                        true
                    }
                    _ => {
                        if self.note_operation.is_none() {
                            self.act_on_selected_track(|track| {
                                self.piano_roll_area
                                    .cast::<HtmlElement>()
                                    .map(|piano_roll_area| {
                                        let mut cursor = "auto";

                                        for note in &track.notes {
                                            if mouse_x < note.screen_x()
                                                || mouse_x > note.right_edge()
                                                || mouse_y < note.screen_y()
                                                || mouse_y > note.bottom_edge()
                                            {
                                                continue;
                                            }

                                            if mouse_x <= note.screen_x() + NOTE_EDGE_WIDTH
                                                || mouse_x >= note.right_edge() - NOTE_EDGE_WIDTH
                                            {
                                                cursor = "ew-resize";
                                                break;
                                            } else {
                                                cursor = "move";
                                                break;
                                            }
                                        }

                                        piano_roll_area.style().set_property("cursor", cursor).ok();
                                        false
                                    });
                            });
                        }

                        false
                    }
                }
            }
            Msg::PlaySingleNote(pitch) => {
                self.play_note(pitch, 1000);
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        if self.midi_access.is_some() {
            html! {
                <>
                    { self.view_piano_roll(ctx) }
                    { self.view_top_bar(ctx) }
                    { self.view_track_panel(ctx) }
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
            <p class="error">{ "This app requires MIDI permissions to work" }</p>
        }
    }

    fn view_top_bar(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div id="top-bar" class="frame dark">
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
                <span>{ "BPM" }</span>
                <input type="number" value={ self.project.bpm.to_string() }
                       min="1" max="5000" size="5" { oninput }/>
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
                <span>{ "Time Signature" }</span>
                <div class="h-box">
                    <select required=true onchange={ on_top_change }>
                        { for top_values }
                    </select>
                    <span>{ "/" }</span>
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
                <span>{ "MIDI Output" }</span>
                <select required=true { onchange }>
                    { for device_options }
                </select>
            </div>
        }
    }

    fn view_track_panel(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div id="project-panel" class="v-box frame dark">
                { self.view_project_info(ctx) }
                { self.view_track_select(ctx) }
                { self.view_track_info(ctx) }
            </div>
        }
    }

    fn view_project_info(&self, ctx: &Context<Self>) -> Html {
        let change_project_name = ctx.link().batch_callback(|event: InputEvent| {
            event
                .target_dyn_into::<HtmlInputElement>()
                .and_then(|input| Some(Msg::SetProjectName(input.value())))
        });

        html! {
            <div class="v-box-left frame full-width">
                <div class="full-width">
                    <span>{ "Project: " }</span>
                    <input value={ self.project.name.to_string() } oninput={ change_project_name }/>
                </div>
            </div>
        }
    }

    fn view_track_select(&self, ctx: &Context<Self>) -> Html {
        let tracks = self
            .project
            .tracks
            .iter()
            .enumerate()
            .map(|(index, track)| {
                html! {
                    <option value={ index.to_string() }>{ track.name.to_string() }</option>
                }
            });

        let on_select = ctx.link().batch_callback(|event: Event| {
            select_get_value(event).and_then(|value| {
                value
                    .parse()
                    .ok()
                    .and_then(|index| Some(Msg::SelectTrack(index)))
                    .or_else(|| Some(Msg::DeselectTrack))
            })
        });

        let create = ctx.link().callback(|_| Msg::CreateTrack);
        let delete = ctx.link().callback(|_| Msg::DeleteSelectedTrack);

        let tracks = if self.project.tracks.is_empty() {
            html! {
                <span>{ "No tracks" }</span>
            }
        } else {
            html! {
                <select onchange={ on_select }>
                    { for tracks }
                </select>
            }
        };

        html! {
            <div class="v-box-left frame full-width">
                { tracks }
                <div class="h-box full-width">
                    <button onclick={ create }>{ "Create" }</button>
                    <button onclick={ delete }>{ "Delete" }</button>
                </div>
            </div>
        }
    }

    fn view_track_info(&self, ctx: &Context<Self>) -> Html {
        let body = self
            .act_on_selected_track(|track| {
                let on_track_name_input = ctx.link().batch_callback(|event: InputEvent| {
                    event
                        .target_dyn_into::<HtmlInputElement>()
                        .and_then(|input| Some(Msg::RenameSelectedTrack(input.value())))
                });

                let on_track_instrument_input = ctx.link().batch_callback(|event: InputEvent| {
                    event
                        .target_dyn_into::<HtmlInputElement>()
                        .and_then(|input| {
                            input.value().parse().ok().and_then(|mut instrument: i64| {
                                // Subtract one because we input the instrument number, not index.
                                instrument -= 1;

                                if instrument >= 0 && instrument <= 127 {
                                    Some(Msg::SetSelectedTrackInstrument(instrument as _))
                                } else {
                                    None
                                }
                            })
                        })
                });

                html! {
                    <>
                        <div class="h-box full-width">
                            <span>{ "Name: "}</span>
                            <input value={ track.name.to_string() } oninput={ on_track_name_input }/>
                        </div>
                        <div class="h-box full-width">
                            <span>{ "Instrument: "}</span>
                            <input type="number" value={ (track.instrument + 1).to_string() }
                                   min="1" max="128" oninput={ on_track_instrument_input }
                                   size="3"/>
                        </div>
                    </>
                }
            })
            .unwrap_or_else(|| {
                html! {
                    <span>{ "No track selected" }</span>
                }
            });

        html! {
            <div class="v-box-left frame full-width">
                { body }
            </div>
        }
    }

    fn view_piano_roll(&self, ctx: &Context<Self>) -> Html {
        let onmousedown = ctx
            .link()
            .callback(|event: MouseEvent| Msg::PianoRollMouseDown(event));

        let onmouseup = ctx.link().callback(|_: MouseEvent| Msg::PianoRollMouseUp);

        let onmousemove = ctx
            .link()
            .callback(|event: MouseEvent| Msg::PianoRollMouseMove(event));

        let oncontextmenu = |event: MouseEvent| event.prevent_default();

        let width = 10000.0;

        html! {
            <div id="piano-view" class="h-box no-gap" style={ format!("width: {}px;", width) }>
                <svg id="note-lines" width="100%" height="100%">
                    { for self.view_note_lines() }
                </svg>
                <svg id="measure-lines" width="100%" height="100%">
                    { for self.view_measure_lines(width) }
                </svg>
                <div id="piano-keys" class="v-box-left no-gap">
                    { for self.view_piano_keys(ctx) }
                </div>
                <svg id="piano-notes" width="100%" height="100%">
                    { for self.view_notes() }
                </svg>
                <div ref={ self.piano_roll_area.clone() } id="clickable-area"
                     { onmousedown } { onmouseup } { onmousemove }
                     { oncontextmenu }/>
            </div>
        }
    }

    fn view_note_lines(&self) -> Vec<Html> {
        (0..128)
            .map(|pitch| {
                let x1 = "0";
                let y1 = ((pitch as f64 * NOTE_RECT_HEIGHT + 1.0) as u32).to_string();

                let x2 = "100%";
                let y2 = y1.to_string();

                html! {
                    <line { x1 } { y1 } { x2 } { y2 } stroke="black" stroke-width="1"/>
                }
            })
            .collect()
    }

    fn view_measure_lines(&self, width: f64) -> Vec<Html> {
        let mut measure_lines = Vec::new();

        let division_width = WHOLE_NOTE_WIDTH / MIN_DIVISION as f64;

        let mut measure_progress = 0;
        let mut x = 0.0;

        while x <= width {
            let time_sig = &self.project.time_signature;

            let stroke_width = {
                if measure_progress % (time_sig.top * MIN_DIVISION / time_sig.bottom) == 0 {
                    "4"
                } else {
                    "1"
                }
            };

            {
                let x = x.round() as i32;

                measure_lines.push(html! {
                    <line x1={ x.to_string() } x2={ x.to_string() } y1="0" y2="100%"
                          stroke="black" stroke-width={ stroke_width }/>
                });
            }

            x += division_width;
            measure_progress += 1;
        }

        measure_lines
    }

    fn view_piano_keys(&self, ctx: &Context<Self>) -> Vec<Html> {
        (0..=127)
            .map(|pitch| {
                // Start from the top.
                let pitch = 127 - pitch;

                let onclick = ctx
                    .link()
                    .callback(move |_: MouseEvent| Msg::PlaySingleNote(pitch));

                let note_name = note_name(pitch);

                let class = if note_name.contains('#') {
                    "black-key"
                } else {
                    "white-key"
                };

                html! {
                    <button { class } { onclick }>{ note_name.to_string() }</button>
                }
            })
            .collect()
    }

    fn view_notes(&self) -> Vec<Html> {
        self.act_on_selected_track(|track| {
            track
                .notes
                .iter()
                .map(|note| {
                    let x = note.screen_x().to_string();
                    let y = note.screen_y().to_string();
                    let width = note.screen_width().to_string();
                    let height = note.screen_height().to_string();

                    html! {
                        <rect { x } { y } { width } { height } rx="3" ry="3"
                              stroke="black" stroke-width="2" fill="green"/>
                    }
                })
                .collect()
        })
        .unwrap_or(Vec::new())
    }

    fn act_on_selected_track<R>(&self, action: impl Fn(&Track) -> R) -> Option<R> {
        if let Some(index) = self.selected_track_index {
            Some(action(&self.project.tracks[index]))
        } else {
            None
        }
    }

    fn act_on_selected_track_mut<R>(&mut self, action: impl Fn(&mut Track) -> R) -> Option<R> {
        if let Some(index) = self.selected_track_index {
            Some(action(&mut self.project.tracks[index]))
        } else {
            None
        }
    }

    fn play_note(&self, note: u8, duration: u32) {
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

struct NoteOperation {
    note_index: usize,
    type_: NoteOperationType,
}

enum NoteOperationType {
    DragLeftEdge,
    DragRightEdge,
    Move,
}

fn main() {
    yew::start_app::<Model>();
}
