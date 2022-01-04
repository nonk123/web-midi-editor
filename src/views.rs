use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::{
    events::{Event, InputEvent},
    prelude::*,
};

use crate::{
    project::{MIN_DIVISION, MIN_INTERVAL, NOTE_RECT_HEIGHT, WHOLE_NOTE_WIDTH},
    util::{note_name, select_get_value, time_signature_options},
    Model, Msg,
};

impl Model {
    pub fn view_no_midi(&self) -> Html {
        html! {
            <p class="error">{ "This app requires MIDI permissions to work" }</p>
        }
    }

    pub fn view_top_bar(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div id="top-bar" class="h-box frame dark">
                { self.view_controls(ctx) }
                { self.view_bpm(ctx) }
                { self.view_time_signature(ctx) }
                { self.view_output_selection(ctx) }
            </div>
        }
    }

    pub fn view_controls(&self, ctx: &Context<Self>) -> Html {
        let toggle = ctx.link().callback(|_| Msg::TogglePlayback);
        let undo = ctx.link().callback(|_| Msg::Undo);
        let redo = ctx.link().callback(|_| Msg::Redo);

        html! {
            <div class="h-box frame">
                <button onclick={ toggle }>{ "Play/Stop" }</button>
                <button onclick={ undo }>{ "Undo" }</button>
                <button onclick={ redo }>{ "Redo" }</button>
            </div>
        }
    }

    pub fn view_bpm(&self, ctx: &Context<Self>) -> Html {
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

    pub fn view_time_signature(&self, ctx: &Context<Self>) -> Html {
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

    pub fn view_output_selection(&self, ctx: &Context<Self>) -> Html {
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

    pub fn view_project_panel(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div id="project-panel" class="v-box frame dark">
                { self.view_project_info(ctx) }
                { self.view_track_select(ctx) }
                { self.view_track_info(ctx) }
            </div>
        }
    }

    pub fn view_project_info(&self, ctx: &Context<Self>) -> Html {
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

    pub fn view_track_select(&self, ctx: &Context<Self>) -> Html {
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

    pub fn view_track_info(&self, ctx: &Context<Self>) -> Html {
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

    pub fn view_piano_roll(&self, ctx: &Context<Self>) -> Html {
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
            <div id="piano-wrapper">
                <div id="piano-view" style={ format!("width: {}px", width) }>
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
            </div>
        }
    }

    pub fn view_note_lines(&self) -> Vec<Html> {
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

    pub fn view_measure_lines(&self, width: f64) -> Vec<Html> {
        let mut measure_lines = Vec::new();

        let division_width = WHOLE_NOTE_WIDTH * MIN_INTERVAL as f64;

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

    pub fn view_piano_keys(&self, ctx: &Context<Self>) -> Vec<Html> {
        (0..=127)
            .map(|pitch| {
                // Start from the top.
                let pitch = 127 - pitch;

                let onclick = ctx
                    .link()
                    .callback(move |_: MouseEvent| Msg::PlayMidiNote(pitch));

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

    pub fn view_notes(&self) -> Vec<Html> {
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
}
