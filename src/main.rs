use action::Action;
use gloo_timers::callback::Interval;
use js_sys::{Array, Uint8Array};
use midi::export_midi;
use views::PIANO_KEYS_WIDTH;
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use web_sys::{Blob, HtmlAnchorElement, HtmlElement, MidiAccess, MidiOutput, SvgLineElement, Url};
use yew::{events::MouseEvent, prelude::*};

mod action;
mod midi;
mod playback;
mod project;
mod util;
mod views;

use project::{
    Note, Project, TimeSignature, Track, MIN_INTERVAL, NOTE_EDGE_WIDTH, NOTE_RECT_HEIGHT,
    WHOLE_NOTE_WIDTH,
};
use util::{mouse_x_to_interval, mouse_y_to_pitch, relative_mouse_pos, snap};

pub enum Msg {
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
    SetBpm(f64),
    SetTimeSignatureTop(u32),
    SetTimeSignatureBottom(u32),
    ProgressBarMouseDown(MouseEvent),
    ProgressBarMouseUp,
    PianoRollMouseDown(MouseEvent),
    PianoRollMouseUp,
    MouseMove(MouseEvent),
    TogglePlayback,
    SetPlayProgress(f64),
    IncrementPlayProgress,
    PlayMidiNote(u8, u8),
    ExportMidi,
    Undo,
    Redo,
}

pub struct Model {
    midi_access: Option<MidiAccess>,
    selected_output: Option<MidiOutput>,
    project: Project,
    selected_track_index: Option<usize>,
    mouse_operation: MouseOperation,
    piano_roll_area: NodeRef,
    last_placed_note_length: f64,
    undo_stack: Vec<Action>,
    redo_stack: Vec<Action>,
    play_offset: f64,
    play_progress: f64,
    progress_line: NodeRef,
    tick_interval: Option<Interval>,
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
            bpm: 120.0,
            tracks: Vec::new(),
        };

        Self {
            midi_access: None,
            selected_output: None,
            project,
            selected_track_index: None,
            mouse_operation: MouseOperation::None,
            piano_roll_area: NodeRef::default(),
            last_placed_note_length: 1.0 / 8.0,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            play_offset: 0.0,
            play_progress: 0.0,
            progress_line: NodeRef::default(),
            tick_interval: None,
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
                self.play_midi_note(0, 60, 1000.0);

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

                self.perform_action(Action::CreateTrack(Track {
                    name: format!("Track {}", len + 1),
                    notes: Vec::new(),
                    instrument: 0,
                }));

                true
            }
            Msg::DeleteSelectedTrack => {
                if let Some(index) = self.selected_track_index {
                    self.perform_action(Action::DeleteTrack(index));

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
                if let Some(index) = self.selected_track_index {
                    self.perform_action(Action::RenameTrack(index, name))
                }

                true
            }
            Msg::SetSelectedTrackInstrument(instrument) => {
                if let Some(index) = self.selected_track_index {
                    self.perform_action(Action::SetTrackInstrument(index, instrument));
                }

                true
            }
            Msg::SetProjectName(name) => {
                self.perform_action(Action::RenameProject(name));
                true
            }
            Msg::SetBpm(bpm) => {
                self.perform_action(Action::SetBpm(bpm));
                true
            }
            Msg::SetTimeSignatureTop(top) => {
                self.perform_action(Action::SetTimeSignatureTop(top));
                true
            }
            Msg::SetTimeSignatureBottom(bottom) => {
                self.perform_action(Action::SetTimeSignatureBottom(bottom));
                true
            }
            Msg::ProgressBarMouseDown(event) => {
                if let MouseOperation::None = self.mouse_operation {
                    self.mouse_operation = MouseOperation::DragProgressBar;

                    let (mouse_x, _) = relative_mouse_pos(&event);
                    self.set_play_offset_from_mouse_x(mouse_x);

                    return true;
                }

                false
            }
            Msg::ProgressBarMouseUp => {
                self.mouse_operation = MouseOperation::None;
                false
            }
            Msg::PianoRollMouseDown(event) => {
                match self.mouse_operation {
                    MouseOperation::None => {}
                    _ => return false,
                };

                let track_index = match self.selected_track_index {
                    None => return false,
                    Some(index) => index,
                };

                let track = &mut self.project.tracks[track_index];

                let (mouse_x, mouse_y) = relative_mouse_pos(&event);

                match event.buttons() {
                    1 => {
                        let existing_note_index = track.get_note_at_position(mouse_x, mouse_y);

                        if let Some(note_index) = existing_note_index {
                            let note = &track.notes[note_index];

                            self.mouse_operation = MouseOperation::NoteOperation {
                                note_index,
                                type_: {
                                    if mouse_x <= note.screen_x() + NOTE_EDGE_WIDTH {
                                        NoteOperationType::DragLeftEdge(note.offset, note.length)
                                    } else if mouse_x >= note.right_edge() - NOTE_EDGE_WIDTH {
                                        NoteOperationType::DragRightEdge(note.length)
                                    } else {
                                        let grab_offset = mouse_x_to_interval(mouse_x);

                                        NoteOperationType::Move(
                                            grab_offset - note.offset,
                                            note.offset,
                                            note.pitch,
                                        )
                                    }
                                },
                            };
                        } else {
                            let len = track.notes.len();

                            track.notes.push(Note {
                                pitch: mouse_y_to_pitch(mouse_y),
                                velocity: 127,
                                offset: mouse_x_to_interval(mouse_x),
                                length: self.last_placed_note_length,
                            });

                            self.mouse_operation = MouseOperation::NoteOperation {
                                note_index: len,
                                type_: NoteOperationType::CreateAndMove,
                            };
                        }

                        true
                    }
                    2 => {
                        if let Some(note_index) = track.get_note_at_position(mouse_x, mouse_y) {
                            self.perform_action(Action::DeleteNote(track_index, note_index));
                            true
                        } else {
                            false
                        }
                    }
                    _ => false,
                }
            }
            Msg::PianoRollMouseUp => {
                let selected_track_index = match self.selected_track_index {
                    None => return false,
                    Some(index) => index,
                };

                let result = match self.mouse_operation.clone() {
                    MouseOperation::None | MouseOperation::DragProgressBar => false,
                    MouseOperation::NoteOperation { note_index, type_ } => {
                        let track = &mut self.project.tracks[selected_track_index];
                        let note = &mut track.notes[note_index];

                        let new_offset = note.offset;
                        let new_pitch = note.pitch;
                        let new_length = note.length;

                        let mut create_note_instead = None;

                        match type_ {
                            NoteOperationType::DragLeftEdge(offset, length) => {
                                note.offset = offset;
                                note.length = length;
                            }
                            NoteOperationType::DragRightEdge(length) => {
                                note.length = length;
                            }
                            NoteOperationType::Move(_, offset, pitch) => {
                                note.offset = offset;
                                note.pitch = pitch;
                            }
                            NoteOperationType::CreateAndMove => {
                                let mut note = track.notes.remove(note_index);
                                note.offset = new_offset;
                                note.pitch = new_pitch;
                                create_note_instead = Some(note);
                            }
                        }

                        if let Some(note) = create_note_instead {
                            self.perform_action(Action::CreateNote(selected_track_index, note));
                        } else {
                            self.perform_action(Action::EditNote(
                                selected_track_index,
                                note_index,
                                new_offset,
                                new_pitch,
                                new_length,
                            ));
                        }

                        true
                    }
                };

                self.mouse_operation = MouseOperation::None;

                result
            }
            Msg::MouseMove(event) => {
                let (mouse_x, mouse_y) = relative_mouse_pos(&event);

                match self.mouse_operation.clone() {
                    MouseOperation::DragProgressBar => {
                        self.set_play_offset_from_mouse_x(mouse_x);
                        true
                    }
                    MouseOperation::NoteOperation { note_index, type_ } => {
                        let index = match self.selected_track_index {
                            Some(index) => index,
                            None => return false,
                        };

                        let track = &mut self.project.tracks[index];
                        let note = &mut track.notes[note_index];

                        let offset = mouse_x_to_interval(mouse_x);

                        let pitch = 127
                            - (mouse_y / NOTE_RECT_HEIGHT - 0.5).round().clamp(0.0, 127.0) as u8;

                        match type_ {
                            NoteOperationType::Move(grab_offset, _, _) => {
                                note.offset = offset - grab_offset;
                                note.pitch = pitch;
                            }
                            NoteOperationType::CreateAndMove => {
                                note.offset = offset;
                                note.pitch = pitch;
                            }
                            NoteOperationType::DragLeftEdge(_, _) => {
                                let offset = offset.min(note.offset + note.length);

                                note.length += note.offset - offset;
                                note.offset = offset;
                            }
                            NoteOperationType::DragRightEdge(_) => {
                                let offset = offset.max(note.offset - note.length);
                                note.length = offset - note.offset;
                            }
                        }

                        if note.length < 1e-4 {
                            note.length = MIN_INTERVAL;
                        }

                        self.last_placed_note_length = note.length;

                        true
                    }
                    MouseOperation::None => {
                        if event.buttons() == 2 {
                            let selected_track_index = match self.selected_track_index {
                                Some(index) => index,
                                None => return false,
                            };

                            let track = &mut self.project.tracks[selected_track_index];
                            let note_index = track.get_note_at_position(mouse_x, mouse_y);

                            if let Some(note_index) = note_index {
                                self.perform_action(Action::DeleteNote(
                                    selected_track_index,
                                    note_index,
                                ));

                                return true;
                            }
                        }

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
                                });
                        });

                        false
                    }
                }
            }
            Msg::TogglePlayback => {
                self.play(ctx);
                false
            }
            Msg::SetPlayProgress(progress) => {
                self.play_progress = progress;
                true
            }
            Msg::IncrementPlayProgress => {
                self.play_progress += MIN_INTERVAL;

                if self.play_offset + self.play_progress - 1e-4 >= self.project.length() {
                    self.tick_interval.take().map(|interval| interval.cancel());
                    self.play_progress = 0.0;
                }

                true
            }
            Msg::PlayMidiNote(instrument, pitch) => {
                self.play_midi_note(instrument, pitch, 1000.0);
                false
            }
            Msg::ExportMidi => {
                let project_name = self.project.name.to_owned();

                web_sys::window()
                    .and_then(|window| window.document())
                    .map(|document| {
                        document
                            .create_element("a")
                            .ok()
                            .and_then(|anchor| anchor.dyn_into::<HtmlAnchorElement>().ok())
                            .map(|anchor| {
                                let midi_data = export_midi(&self.project);

                                let array = Uint8Array::new_with_length(midi_data.len() as u32);

                                for (i, byte) in midi_data.iter().enumerate() {
                                    array.set_index(i as u32, *byte);
                                }

                                Blob::new_with_u8_array_sequence(&Array::of1(&array))
                                    .and_then(|blob| Url::create_object_url_with_blob(&blob))
                                    .map(|href| {
                                        document.body().map(|body| {
                                            body.append_child(&anchor).unwrap();

                                            anchor.set_href(&href);
                                            anchor.set_download(&format!("{}.mid", project_name));
                                            anchor.click();

                                            Url::revoke_object_url(&href).ok();

                                            body.remove_child(&anchor).unwrap();
                                        });
                                    })
                                    .ok();
                            });
                    });

                false
            }
            Msg::Undo => {
                self.undo_last();
                true
            }
            Msg::Redo => {
                self.redo_last();
                true
            }
        }
    }

    fn rendered(&mut self, _ctx: &Context<Self>, _first_render: bool) {
        self.progress_line
            .cast::<SvgLineElement>()
            .map(|progress_line| {
                let x = (self.play_offset + self.play_progress) * WHOLE_NOTE_WIDTH;

                for animated_length in [progress_line.x1(), progress_line.x2()] {
                    animated_length.base_val().set_value(x as f32).ok();
                }
            });
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        if self.midi_access.is_some() {
            self.view_main(ctx)
        } else {
            self.view_no_midi()
        }
    }
}

impl Model {
    pub fn act_on_selected_track<R>(&self, action: impl Fn(&Track) -> R) -> Option<R> {
        if let Some(index) = self.selected_track_index {
            Some(action(&self.project.tracks[index]))
        } else {
            None
        }
    }

    fn set_play_offset_from_mouse_x(&mut self, mouse_x: f64) {
        let offset = (mouse_x - PIANO_KEYS_WIDTH) / WHOLE_NOTE_WIDTH;
        self.play_offset = snap(offset, MIN_INTERVAL);
    }
}

#[derive(Clone)]
enum MouseOperation {
    None,
    DragProgressBar,
    NoteOperation {
        note_index: usize,
        type_: NoteOperationType,
    },
}

#[derive(Clone)]
enum NoteOperationType {
    DragLeftEdge(f64, f64),
    DragRightEdge(f64),
    Move(f64, f64, u8),
    CreateAndMove,
}

fn main() {
    yew::start_app::<Model>();
}
