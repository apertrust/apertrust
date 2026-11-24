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

//! Create a Style… (decision 106; `docs/gui.md`, Creating a style): the active photo's
//! edit, or chosen tools of it, saved as `<user data>/styles/<Name>.apstyle`.
//!
//! - **Two doors,** History › ⋯ › Create a Style… (`create-style`) and Edit › Save Edit as
//!   Style… (`save-style`), each enabled while the photo open takes edits and its edit
//!   touches a tool ([`can_create`]).
//! - **The dialog** ([`StyleDialog`]), the host's, a sheet on the Mac: Name and Description,
//!   as Save as Preset…'s (`presets::name_rows`), then a check box for each tool the edit
//!   touches, in pipeline order ([`lines`]), checked where the tool makes a look,
//!   `ProcessingDefinition::in_styles`, as darktable's create-style dialog checks the
//!   modules flagged `IOP_FLAGS_INCLUDE_IN_STYLES` (`src/gui/styles_dialog.c`, 810 to 820),
//!   each other one saying in its tooltip that it usually belongs to one photo.
//! - **Create** writes one entry per tool checked, its values now as a preset saves them
//!   ([`create`]), and the Viewer says "Style "…" created" ([`CREATED`]). No history step:
//!   no photo changes.

use std::path::{Path, PathBuf};

use apertrust_engine::session::{ToolState, tool_entry};
use apertrust_library::presets::{self as files, Kind, PresetError, Stored};
use apertrust_tools::{ProcessingTool, messages};
use apertrust_ui::components::Checkbox;
use apertrust_ui::i18n::Args;
use apertrust_ui::{Translator, theme, tr};
use eframe::egui::{self, Label, RichText};

use crate::dialog_host::Output;
use crate::mirror::Mirror;
use crate::presets::{SheetContent, current, description_rows, name_rows, refusal};

/// The message id of the status message a style created gives, its name as `name`.
pub const CREATED: &str = "style-created";
/// The message id of the status message a style not written gives, its name as `name`
/// and why as `error`.
pub const NOT_CREATED: &str = "style-not-created";

/// The messages that say why a name cannot be taken: empty, and another style's.
const REFUSED: (&str, &str) = ("styles-name-empty", "styles-name-taken");

/// Whether the edit holds `state`'s tool (decision 106): a value off its default or a
/// switched tool on (`ToolState::edited`), or a mark. A switched tool off holds its marks
/// alone, as the edit keeps it.
pub fn touches(state: &ToolState) -> bool {
    state.edited || state.controls.iter().any(|control| control.mark.is_some())
}

/// Whether Create a Style… can run on `mirror`'s photo: open, live, taking edits, and its
/// edit touching a tool.
pub fn can_create(mirror: &Mirror) -> bool {
    mirror.takes_edits() && mirror.live() && mirror.values.tools.iter().any(touches)
}

/// One tool the dialog lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub tool: &'static str,
    /// Its title, translated as the dialog opened.
    pub title: String,
    pub checked: bool,
    /// Its `ProcessingDefinition::in_styles`: checked at first, and no tooltip.
    pub in_styles: bool,
}

/// The tools of `states` the edit touches ([`touches`]), among `tools`, in pipeline order
/// (`layout::hidden_order`), each titled in `words` and checked where its `in_styles` is.
pub fn lines(tools: &[&dyn ProcessingTool], states: &[ToolState], words: &Translator) -> Vec<Line> {
    let mut lines: Vec<Line> = (states.iter().filter(|state| touches(state)))
        .filter_map(|state| {
            let tool = tools
                .iter()
                .find(|t| t.definition().tool.id == state.tool)?;
            let in_styles = tool.definition().in_styles;
            Some(Line {
                tool: state.tool,
                title: words.text(&messages::title(state.tool)),
                checked: in_styles,
                in_styles,
            })
        })
        .collect();
    lines.sort_by_key(|line| crate::layout::hidden_order(line.tool));
    lines
}

/// The styles' names in `folder`, trimmed, which a new style cannot take; a file that
/// cannot be read is left out with a warning under `-d settings`.
pub fn taken(folder: &Path) -> Vec<String> {
    let listed = files::list(folder, Kind::Style);
    for (path, error) in &listed.failed {
        let path = path.display();
        tracing::warn!(target: "settings", "style {path} not read: {error}");
    }
    (listed.files.into_iter())
        .map(|stored| stored.name.trim().to_owned())
        .collect()
}

/// What the dialog asked of the app as Create was pressed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asked {
    /// Trimmed, never empty.
    pub name: String,
    pub description: Option<String>,
    /// The tools checked, in the dialog's order.
    pub tools: Vec<&'static str>,
}

/// Writes the style `asked` into `folder`: one entry per tool it names, of `tools`, with
/// its values now, `states`, as a preset saves them (`presets::current`), a switched tool
/// off holding no `enabled` and no value; `build` recorded in it. Refuses a name a style in
/// the folder already holds, as the dialog does, should one have come since it opened.
/// Returns its path.
pub fn create(
    folder: &Path,
    tools: &[&dyn ProcessingTool],
    states: &[ToolState],
    asked: &Asked,
    build: &str,
) -> Result<PathBuf, PresetError> {
    let name = asked.name.trim();
    if taken(folder).iter().any(|taken| taken == name) {
        return Err(PresetError::NameTaken(name.to_owned()));
    }
    let mut stored = Stored::new(Kind::Style, name);
    stored.description = asked.description.clone();
    for id in &asked.tools {
        let tool = tools.iter().find(|t| t.definition().tool.id == *id);
        let state = states.iter().find(|state| state.tool == *id);
        if let (Some(tool), Some(state)) = (tool, state) {
            let definition = tool.definition();
            let values = current(definition, state);
            stored.entries.push(tool_entry(definition, &values));
        }
    }
    files::write(folder, &stored, build)
}

/// What the Viewer says once the style `name` is written, or, with `error`, why it was
/// not, in `words`.
pub fn said(words: &Translator, name: &str, error: Option<&PresetError>) -> String {
    let mut args = Args::new();
    args.set("name", name);
    match error {
        None => words.text_with(CREATED, Some(&args)),
        Some(error) => {
            args.set("error", error.to_string());
            words.text_with(NOT_CREATED, Some(&args))
        }
    }
}

/// Create a Style…, a [`Sheet`](crate::presets::Sheet)'s content: Name, its first focus, empty, Description,
/// optional, and a check box for each tool the edit touches, above Create. An empty name
/// or one `taken` says why under the field, and no tool checked under the tools, each
/// disabling Create.
#[derive(Debug)]
pub struct StyleDialog {
    /// The names it cannot take, trimmed.
    taken: Vec<String>,
    lines: Vec<Line>,
    name: String,
    description: String,
    /// Whether the name has had the focus, given as the dialog first draws.
    focused: bool,
}

impl StyleDialog {
    /// The dialog over the tools `lines` ([`lines`]), the names `taken` refused
    /// ([`taken`]).
    pub fn new(taken: Vec<String>, lines: Vec<Line>) -> Self {
        StyleDialog {
            taken,
            lines,
            name: String::new(),
            description: String::new(),
            focused: false,
        }
    }

    /// Whether Create can be pressed: a name it can take, and a tool checked.
    fn able(&self, words: &Translator) -> bool {
        let refused = refusal(&self.name, &self.taken, REFUSED, &Args::new(), words);
        refused.is_none() && self.lines.iter().any(|line| line.checked)
    }

    /// What Create asks.
    fn asked(&self) -> Asked {
        let description = self.description.trim();
        Asked {
            name: self.name.trim().to_owned(),
            description: (!description.is_empty()).then(|| description.to_owned()),
            tools: (self.lines.iter())
                .filter(|line| line.checked)
                .map(|line| line.tool)
                .collect(),
        }
    }
}

impl SheetContent for StyleDialog {
    fn id(&self) -> &'static str {
        "style-create"
    }

    fn title(&self, ctx: &egui::Context) -> String {
        tr(ctx, "styles-create-title")
    }

    fn rows(&mut self, ui: &mut egui::Ui, words: &Translator) -> bool {
        let t = theme(ui.ctx());
        let taken = &self.taken;
        let refused = |name: &str| refusal(name, taken, REFUSED, &Args::new(), words);
        let enter = name_rows(ui, &mut self.name, &mut self.focused, refused);
        description_rows(ui, &mut self.description);
        ui.add(Label::new(words.text("styles-tools")).selectable(false));
        let one_photo = words.text("styles-tool-one-photo");
        for line in &mut self.lines {
            let Line {
                title,
                checked,
                in_styles,
                ..
            } = line;
            let response = Checkbox::new(checked, title).show(ui);
            if !*in_styles {
                ui.ctx().accesskit_node_builder(response.id, |node| {
                    node.set_description(one_photo.as_str());
                });
                drop(response.on_hover_text(&one_photo));
            }
        }
        if !self.lines.iter().any(|line| line.checked) {
            let why = RichText::new(words.text("styles-no-tool"))
                .size(t.text.small)
                .color(t.colors.error);
            ui.add(Label::new(why).selectable(false));
        }
        enter
    }

    fn button(&self, words: &Translator) -> (String, bool) {
        (words.text("styles-create-button"), self.able(words))
    }

    fn output(&self) -> Output {
        Output::Style(self.asked())
    }
}

#[cfg(test)]
mod tests;
