use crate::{
    project::{Note, Track},
    Model,
};

#[derive(Clone)]
pub enum Action {
    RenameProject(String),
    SetBpm(u32),
    SetTimeSignatureTop(u32),
    SetTimeSignatureBottom(u32),
    CreateTrack(Track),
    DeleteTrack(usize),
    RenameTrack(usize, String),
    SetTrackInstrument(usize, u8),
    CreateNote(usize, Note),
    DeleteNote(usize, usize),
    EditNote(usize, usize, f64, u8, f64),
}

impl Model {
    pub fn perform_action(&mut self, action: Action) {
        let inverse = self.perform_action_impl(action);
        self.undo_stack.push(inverse);
    }

    pub fn perform_action_impl(&mut self, action: Action) -> Action {
        match action {
            Action::RenameProject(new_name) => {
                let old_name = self.project.name.clone();
                self.project.name = new_name;
                Action::RenameProject(old_name)
            }
            Action::SetBpm(new_bpm) => {
                let old_bpm = self.project.bpm;
                self.project.bpm = new_bpm;
                Action::SetBpm(old_bpm)
            }
            Action::SetTimeSignatureTop(top) => {
                let old_top = self.project.time_signature.top;
                self.project.time_signature.top = top;
                Action::SetTimeSignatureTop(old_top)
            }
            Action::SetTimeSignatureBottom(bottom) => {
                let old_bottom = self.project.time_signature.bottom;
                self.project.time_signature.bottom = bottom;
                Action::SetTimeSignatureBottom(old_bottom)
            }
            Action::CreateTrack(track) => {
                let old_len = self.project.tracks.len();
                self.project.tracks.push(track);

                if old_len == 0 {
                    self.selected_track_index = Some(0);
                }

                Action::DeleteTrack(old_len)
            }
            Action::DeleteTrack(index) => {
                if let Some(selected_track_index) = self.selected_track_index {
                    if index == selected_track_index {
                        self.selected_track_index = None;
                    }
                }

                let track = self.project.tracks.remove(index);
                Action::CreateTrack(track)
            }
            Action::RenameTrack(index, new_name) => {
                let track = &mut self.project.tracks[index];
                let old_name = track.name.clone();
                track.name = new_name;
                Action::RenameTrack(index, old_name)
            }
            Action::SetTrackInstrument(track_index, instrument) => {
                let track = &mut self.project.tracks[track_index];
                let old_instrument = track.instrument;
                track.instrument = instrument;
                Action::SetTrackInstrument(track_index, old_instrument)
            }
            Action::CreateNote(track_index, note) => {
                let track = &mut self.project.tracks[track_index];
                let note_index = track.notes.len();
                track.notes.push(note);
                Action::DeleteNote(track_index, note_index)
            }
            Action::DeleteNote(track_index, note_index) => {
                let note = self.project.tracks[track_index].notes.remove(note_index);
                Action::CreateNote(track_index, note)
            }
            Action::EditNote(track_index, note_index, new_offset, new_pitch, new_length) => {
                let note = &mut self.project.tracks[track_index].notes[note_index];

                let old_offset = note.offset;
                let old_pitch = note.pitch;
                let old_length = note.length;

                note.offset = new_offset;
                note.pitch = new_pitch;
                note.length = new_length;

                Action::EditNote(track_index, note_index, old_offset, old_pitch, old_length)
            }
        }
    }

    pub fn undo_last(&mut self) {
        match self.undo_stack.pop() {
            None => {}
            Some(action) => {
                let inverse = self.perform_action_impl(action);
                self.redo_stack.push(inverse);
            }
        }
    }

    pub fn redo_last(&mut self) {
        match self.redo_stack.pop() {
            None => {}
            Some(action) => {
                let inverse = self.perform_action_impl(action);
                self.undo_stack.push(inverse);
            }
        }
    }
}
