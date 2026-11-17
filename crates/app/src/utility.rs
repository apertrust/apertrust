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

//! The utility tools' behaviour (decision 93; `docs/gui.md`, One tool model): darktable's
//! `lib_api.h` to [`apertrust_tools::utility`]'s declarations, as the trait
//! `ProcessingTool` is `iop_api.h`'s non-GUI half to a processing tool's.
//!
//! - **[`UtilityTool`]** is what a utility does: its page's custom rows, each drawn by its
//!   [`UtilityTool::body`] with the row's widget, and its own ↺, where its declaration says
//!   so. Its egui code stays in its module (`history`, `scopes`, `library` for Browse).
//! - **One [`Context`] for every utility:** the mirror, the settings, the words and the
//!   frame's commands for the session.
//! - **The shell's part,** its section open or closed and the peek keys, is the header's
//!   ([`crate::tools::Head`]), drawn from the declaration by [`crate::tools::header`] as
//!   every tool's is, so neither `body` nor the context holds it.
//! - **Until step 93.4** each utility's page is one custom row, drawn whole by today's code;
//!   [`page`] draws the custom rows, and a test holds every page to them.

use apertrust_engine::session::Command;
use apertrust_tools::utility::UtilityDefinition;
use apertrust_tools::{Reset, Row, ToolDefinition, Widget, messages};
use apertrust_ui::components as parts;
use apertrust_ui::{Translator, translator};
use eframe::egui;

use crate::actions::{self, State};
use crate::mirror::Mirror;
use crate::settings::Settings;
use crate::tools::{self, Head, Pressed};

/// What every utility is given each frame (decision 93): what the session reported, the
/// settings, the words, and the commands the frame sends the session once the side panel is
/// drawn.
pub struct Context<'a> {
    pub mirror: &'a Mirror,
    pub settings: &'a Settings,
    pub words: &'a Translator,
    pub commands: &'a mut Vec<Command>,
}

/// A utility tool's behaviour (decision 93), darktable's `lib_api.h`: its declaration, its
/// page's custom rows, and its own ↺.
pub trait UtilityTool {
    /// What its page's widgets ask the app for, beyond the session's commands.
    type Action;

    fn definition(&self) -> &'static UtilityDefinition;

    /// Draws the custom row of its page showing `widget`, one of its own, called once per
    /// custom row each frame, and returns what it asked for.
    fn body(
        &mut self,
        widget: Widget,
        ui: &mut egui::Ui,
        cx: &mut Context<'_>,
    ) -> Vec<Self::Action>;

    /// Its header's ↺, where its declaration's `reset` is its own (`Reset::Own`).
    fn reset(&mut self, _cx: &mut Context<'_>) {}
}

/// What a utility's section did this frame: what its header asked the app for, its own
/// ↺ carried out, and what its page asked for.
#[derive(Debug)]
pub struct Drawn<A> {
    pub pressed: Vec<Pressed>,
    pub actions: Vec<A>,
}

/// Draws `tool`'s section, its header from its declaration ([`tools::header`]) as `head`
/// has it, ↺ drawn disabled unless `resettable`, and, while open, its page ([`page`]).
/// Its own ↺ is carried out here ([`UtilityTool::reset`]).
pub fn section<T: UtilityTool>(
    ui: &mut egui::Ui,
    tool: &mut T,
    cx: &mut Context<'_>,
    head: Head,
    resettable: bool,
) -> Drawn<T::Action> {
    let definition = &tool.definition().tool;
    let (title, reset) = titles(cx.words, definition);
    let about = about(cx.words, definition);
    let photo = head.photo;
    let mut actions = Vec::new();
    let section = tools::header(definition, &title, &reset, resettable, about, head);
    let out = section.show(ui, |ui| actions = page(ui, tool, cx));
    let mut pressed = tools::pressed(ui, definition, &out, photo);
    if pressed.contains(&Pressed::Reset) {
        tool.reset(cx);
    }
    pressed.retain(|pressed| *pressed != Pressed::Reset);
    Drawn { pressed, actions }
}

/// Draws the header of the utility `definition` alone, as `head` has it, for a section
/// whose header stays at its tab's top while its page scrolls under it, as History's
/// does: the caller draws the page apart while it is open. Returns what it asked for, its
/// own ↺ among it, for the caller to carry out.
pub fn header(
    ui: &mut egui::Ui,
    definition: &ToolDefinition,
    head: Head,
    resettable: bool,
) -> Vec<Pressed> {
    let words = translator(ui.ctx());
    let (title, reset) = titles(&words, definition);
    let about = about(&words, definition);
    let photo = head.photo;
    let section = tools::header(definition, &title, &reset, resettable, about, head);
    let out = section.show_header(ui);
    tools::pressed(ui, definition, &out, photo)
}

/// Draws `tool`'s page: each custom row by its body, in the page's order, the rows before
/// its tab's, its one tab's, then those after. Until step 93.4 a utility's page holds custom
/// rows alone.
pub fn page<T: UtilityTool>(
    ui: &mut egui::Ui,
    tool: &mut T,
    cx: &mut Context<'_>,
) -> Vec<T::Action> {
    let definition = &tool.definition().tool;
    let tab = definition.tabs.first().map_or(&[][..], |tab| tab.rows);
    let mut actions = Vec::new();
    for rows in [definition.before, tab, definition.after] {
        for row in rows {
            if let Row::Custom { widget, .. } = row {
                actions.extend(tool.body(*widget, ui, cx));
            }
        }
    }
    actions
}

/// Whether the utility `definition`'s ↺ can run in `state`: as its registry action can,
/// History's Reset Edit; a utility's own always; none where it has none.
pub fn resettable(definition: &ToolDefinition, state: &State) -> bool {
    match definition.reset {
        Reset::Action(id) => actions::named(id).is_some_and(|action| action.enabled(state)),
        Reset::Own => true,
        Reset::Tool | Reset::None => false,
    }
}

/// What the utility `definition`'s header's tooltip says, where its words hold its
/// sentence: that it edits nothing, and what it measures. Browse, whose sentence is still a
/// design question, says nothing, as before.
fn about(words: &Translator, definition: &ToolDefinition) -> Option<parts::About> {
    let id = definition.id;
    (words.has(&messages::description(id))).then(|| tools::about(words, id, None))
}

/// The utility `definition`'s title, `section-<id>`, and its ↺'s name.
fn titles(words: &Translator, definition: &ToolDefinition) -> (String, String) {
    let title = words.text(&format!("section-{}", definition.id));
    let reset = tools::reset_name(words, definition, &title);
    (title, reset)
}

#[cfg(test)]
mod tests {
    use apertrust_tools::utility::UTILITIES;
    use apertrust_tools::{Reset, Row, TOOLS};

    /// Until step 93.4 every utility's page is custom rows alone, each one of its own
    /// widgets, which its body draws; every header follows its declaration: a processing
    /// tool's ↺ its own, with presets, a utility's no presets and ↺ a registry action's,
    /// its own or none, as the header draws them.
    #[test]
    fn every_page_is_custom_rows_and_every_header_one_the_header_draws() {
        for utility in UTILITIES {
            let tool = &utility.tool;
            let tab = tool.tabs.iter().flat_map(|tab| tab.rows);
            for row in tool.before.iter().chain(tab).chain(tool.after) {
                assert!(matches!(row, Row::Custom { .. }), "{}: {row:?}", tool.id);
            }
            assert!(tool.tabs.len() <= 1 && tool.tabs.iter().all(|tab| tab.id.is_none()));
            assert!(!tool.presets, "{}", tool.id);
            assert_ne!(tool.reset, Reset::Tool, "{}", tool.id);
            if let Reset::Action(id) = tool.reset {
                assert!(crate::actions::named(id).is_some(), "{}: {id}", tool.id);
            }
        }
        for tool in TOOLS {
            let tool = &tool.definition().tool;
            assert_eq!(
                (tool.reset, tool.presets),
                (Reset::Tool, true),
                "{}",
                tool.id
            );
        }
    }
}
