// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//! A sheet in the main window (decision 86; `docs/gui.md`, Sheets, Modality): the host of
//! the design system's `Sheet`, whose content, Import's, the caller opens. One at a time,
//! drawn over the window below the Toolbar in an area of its own, as the customise mode's
//! scrim is, so the panels under it take no pointer.
//!
//! While one is open the shell's state says so ([`Shell::state`](crate::Shell)): the view's
//! keys and the Toolbar's actions rest, the Toolbar drawn at rest rather than greyed, the
//! items that act on the main window or its photo are disabled, as under a dialog's own
//! window, and every action that opens a dialog rests, Export… and Export Again among them;
//! the jobs spinner and View › Jobs stay live. Close Window, the window's close button and
//! the Toolbar's drawn close close the sheet first; Quit asks only where a job runs, as
//! always. The sheet's own keys are the frame's (`apertrust_ui::components::Sheet`).

use std::path::{Path, PathBuf};

use apertrust_engine::session::{Command, LibraryEvent};
use apertrust_ui::components::PanelEdgeAction;
use eframe::egui;

use crate::SendCommand;
use crate::mirror::Mirror;
use crate::settings::Settings;

/// Where the sheet's tool column is: its kept width, and whether it is at the window's
/// start, as the Tools panel is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    pub column: f32,
    pub at_start: bool,
}

/// What a frame of the sheet came to, for the shell: whether it closed, what was done to
/// its column's edge, the settings its tool changed, written at once with no history step,
/// its tool's groups opened or closed, and whether it asks for the folder chooser.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Shown {
    pub closed: bool,
    pub column: Option<PanelEdgeAction>,
    /// Each a text setting by its key in full, as the store keeps it.
    pub settings: Vec<(&'static str, String)>,
    /// Each of its tool's collapsible groups by id, opened or closed, kept in `edit.groups`
    /// under the tool's id.
    pub groups: Vec<(&'static str, bool)>,
    /// The system's folder chooser, whose answer comes back as [`Content::folder_chosen`],
    /// starting at `folder_start` where it is given and there, else its nearest folder.
    pub choose_folder: bool,
    pub folder_start: Option<PathBuf>,
    /// The job it started this frame, by its command's number, as Import's primary button
    /// sends `Import`: quitting waits for it, and its destination opens as it ends.
    pub started: Option<u64>,
}

/// What the shell gives a sheet's content each frame: what the session reported and the
/// settings, which its tool's page is drawn with.
pub struct Given<'a> {
    pub mirror: &'a Mirror,
    pub settings: &'a Settings,
}

/// A sheet's content: draws the design system's `Sheet` into the room it is given, acting
/// on what it reports and sending the session what it asks through `send`; takes in the
/// library's events the mirror hands it, and says what it sends as it closes.
pub trait Content {
    fn show(
        &mut self,
        ui: &mut egui::Ui,
        place: Place,
        given: &Given<'_>,
        send: &mut SendCommand<'_>,
    ) -> Shown;

    /// Takes in one of the library's events, Import's or a thumbnail of no folder open
    /// (`Reaction::Import`): the content keeps those of its own commands' numbers.
    fn receive(&mut self, _event: &LibraryEvent) {}

    /// What it sends as it closes, however it closes.
    fn closing(&mut self) -> Vec<Command> {
        Vec::new()
    }

    /// The folder the system's folder chooser answered with, which it asked for.
    fn folder_chosen(&mut self, _folder: &Path) {}
}

/// Draws `content` over the window below the Toolbar, `toolbar`, at `place`.
pub fn show(
    ctx: &egui::Context,
    toolbar: egui::Rect,
    content: &mut dyn Content,
    place: Place,
    given: &Given<'_>,
    send: &mut SendCommand<'_>,
) -> Shown {
    let window = ctx.content_rect();
    let below = egui::Rect::from_min_max(egui::pos2(window.left(), toolbar.bottom()), window.max);
    egui::Area::new(egui::Id::unique("sheet"))
        .order(egui::Order::Middle)
        .fixed_pos(below.min)
        .interactable(true)
        .show(ctx, |ui| {
            // Taken in the area itself, so the area is as large as the room and the wheel
            // over it reaches the sheet's scroll areas.
            ui.allocate_rect(below, egui::Sense::hover());
            let mut room = ui.new_child(egui::UiBuilder::new().max_rect(below));
            room.set_clip_rect(below);
            content.show(&mut room, place, given, send)
        })
        .inner
}

/// The column's kept width after `action` on its edge: the width dragged to, or
/// `import.column_width`'s default.
pub fn resized(action: PanelEdgeAction) -> f32 {
    let setting = &crate::settings::registry::IMPORT_COLUMN_WIDTH;
    match action {
        PanelEdgeAction::Resize(width) => {
            width.clamp(*setting.range().start(), *setting.range().end())
        }
        PanelEdgeAction::Reset => setting.default(),
    }
}
