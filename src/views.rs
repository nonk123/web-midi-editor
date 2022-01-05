use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::{
    events::{Event, InputEvent},
    prelude::*,
};

use crate::{
    project::{MIN_INTERVAL, NOTE_RECT_HEIGHT, WHOLE_NOTE_WIDTH},
    util::{note_name, select_get_value, time_signature_options},
    Model, Msg,
};

pub const PIANO_KEYS_WIDTH: f64 = 50.0;

impl Model {
    pub fn view_no_midi(&self) -> Html {
        html! {
            <p class="error">{ "This app requires MIDI permissions to work" }</p>
        }
    }

    pub fn view_main(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div id="main-view">
                { self.view_top_bar(ctx) }
                { self.view_project_panel(ctx) }
                { self.view_piano_roll(ctx) }
            </div>
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
        let progress_bar_on_mouse_down = ctx
            .link()
            .callback(|event: MouseEvent| Msg::ProgressBarMouseDown(event));

        let progress_bar_on_mouse_up = ctx.link().callback(|_: MouseEvent| Msg::ProgressBarMouseUp);

        let piano_roll_on_mouse_down = ctx
            .link()
            .callback(|event: MouseEvent| Msg::PianoRollMouseDown(event));

        let piano_roll_on_mouse_up = ctx.link().callback(|_: MouseEvent| Msg::PianoRollMouseUp);

        let on_mouse_move = ctx
            .link()
            .callback(|event: MouseEvent| Msg::MouseMove(event));

        let oncontextmenu = |event: MouseEvent| event.prevent_default();

        let width = 10000.0;

        let measure_width = self.project.time_signature.measure_width();
        let interval_width = WHOLE_NOTE_WIDTH * MIN_INTERVAL;

        let piano_view_style = format!(
            "width: {}px; grid-template-columns: {}px auto;",
            width, PIANO_KEYS_WIDTH
        );

        let grid_lines_style = format!(
            r#"
                background-size: {}px {}px;
                background-image:
                    linear-gradient(black 2px, transparent 1px),
                    linear-gradient(90deg, black 2px, transparent 1px);
            "#,
            interval_width, NOTE_RECT_HEIGHT
        );

        let measure_lines_style = format!(
            r#"
                background-size: {}px 100%;
                background-image: linear-gradient(90deg, black 4px, transparent 1px);
            "#,
            measure_width
        );

        html! {
            <div id="piano-wrapper">
                <div id="piano-view" style={ piano_view_style }>
                    <div id="piano-keys" class="v-box-left no-gap">
                        { for self.view_piano_keys(ctx) }
                    </div>
                    <svg id="progress-bar" width="100%" height="100%">
                        { for self.view_measure_numbers(width) }
                    </svg>
                    <div id="progress-bar-clickable-area"
                         onmousedown={ progress_bar_on_mouse_down }
                         onmouseup={ progress_bar_on_mouse_up }
                         onmousemove= { on_mouse_move.clone() }/>
                    <div class="overlay" style={ grid_lines_style }/>
                    <div class="overlay" style={ measure_lines_style }/>
                    <svg id="piano-roll" width="100%" height="100%">
                        { for self.view_notes() }
                        <line ref={ self.progress_line.clone() } y1="0" y2="100%"
                              stroke="white" stroke-width="2"/>
                    </svg>
                    <div ref={ self.piano_roll_area.clone() } id="piano-roll-clickable-area"
                         onmousedown={ piano_roll_on_mouse_down }
                         onmouseup={ piano_roll_on_mouse_up }
                         onmousemove={ on_mouse_move }
                         { oncontextmenu }/>
                </div>
            </div>
        }
    }

    pub fn view_measure_numbers(&self, width: f64) -> Vec<Html> {
        let mut measure_numbers = Vec::new();

        let mut progress = 0.0;
        let mut x = 0.0;

        let mut measure_number = 1;

        let measure_length = self.project.time_signature.measure_length();

        while x <= width {
            if progress % measure_length <= 1e-5 {
                let x = x + PIANO_KEYS_WIDTH;

                measure_numbers.push(html! {
                    <text class="measure-number" x={ x.to_string() } y="50%">
                        { measure_number.to_string() }
                    </text>
                });

                measure_number += 1;
            }

            progress += MIN_INTERVAL;
            x = progress * WHOLE_NOTE_WIDTH;
        }

        measure_numbers
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

                let style = format!("height: {}px;", NOTE_RECT_HEIGHT);

                html! {
                    <button { class } { style } { onclick }>
                        { note_name.to_string() }
                    </button>
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
