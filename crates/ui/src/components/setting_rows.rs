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

//! The setting rows the Tools panel lacks, which the Import sheet's tool and the Export
//! tool share (`docs/gui.md`, The rows the tool panel lacks; decision 86), each built from
//! what the panel draws already.
//!
//! - **The warning button** ([`WarningButton`]): the 12-point warning sign, 18 × 18 with
//!   no plate, in `warning`, or `error` where the problem stops the job. Its tooltip says
//!   the problem; a click, or Return or Space while it is focused, opens its menu of the
//!   fixes the caller gives, the one chosen reported. Screen readers hear a menu button
//!   named by the problem, and the problem once, assertively, as it appears.
//! - **The path row, Import's form** ([`PathRow`]): the label on a line of its own, then
//!   [`PathField`] taking the content's width less what follows it, 4 points apart: the
//!   warning button where there is a problem, the ⌄ of choices where the row has them and
//!   the folder button where it has one. The field is one line: a long pattern scrolls
//!   inside it. Alt+↓ on the focused field opens the ⌄; Return in it ends the typing, or
//!   takes the variable highlighted in its list, and presses nothing. The ⌄'s menu holds
//!   radio items, each its name with its pattern under it, and disabled ones with their
//!   reasons.
//! - **The text field row** ([`TextFieldRow`]): one 24-point line, the label at its start
//!   and a 180-point [`TextField`] at its end, the label cut first, never the field; the
//!   warning button before the field where there is a problem. Return in it ends the
//!   typing and is reported, so a sheet can press its primary button.
//! - **A problem** draws the field's text in `error` where it stops the job, with no
//!   outline round the field ([`TextField::problem`]); the field's accessible description
//!   says it.
//! - **Disabled,** a row is drawn disabled, its reason its tooltip, never hidden, and its
//!   warning button is not drawn.

use egui::accesskit::Live;
use egui::{
    Align, FontId, Id, Key, Layout, Modifiers, Rect, Response, Role, Sense, Ui, UiBuilder,
    WidgetInfo, pos2, vec2,
};

use super::{IconButton, PathField, TextField, Variable, focus_ring, pull_down, truncated};
use crate::popup::{self, Entry, Item, Request};
use crate::{Icon, Theme, theme};

/// The text field row's field, in points (`docs/gui.md`, The rows the tool panel lacks):
/// as wide as Settings' text fields.
pub const TEXT_FIELD_WIDTH: f32 = 180.0;

/// A fix a warning button's menu offers for its problem.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fix<'a> {
    /// What the menu answers with, chosen.
    pub id: &'a str,
    /// Its words, translated: "Use $(MONTH)", "Choose a folder…".
    pub text: &'a str,
}

/// A problem in a row, which its warning button names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Problem<'a> {
    /// What is wrong, translated: the button's tooltip and name.
    pub text: &'a str,
    /// Whether it stops the job: drawn in `error`, the field's text too, else in `warning`.
    pub stops: bool,
    /// The fixes its menu offers.
    pub fixes: &'a [Fix<'a>],
}

impl<'a> Problem<'a> {
    /// A problem that warns, stopping nothing, with no fix offered.
    pub fn new(text: &'a str) -> Self {
        Problem {
            text,
            stops: false,
            fixes: &[],
        }
    }

    /// Whether it stops the job.
    pub fn stops(mut self, stops: bool) -> Self {
        self.stops = stops;
        self
    }

    /// The fixes its warning button's menu offers.
    pub fn fixes(mut self, fixes: &'a [Fix<'a>]) -> Self {
        self.fixes = fixes;
        self
    }
}

/// The warning button: a row's problem, and the menu of its fixes.
pub struct WarningButton<'a> {
    id: Id,
    problem: Problem<'a>,
    enabled: bool,
}

impl<'a> WarningButton<'a> {
    /// `id` must be stable across frames.
    pub fn new(id: Id, problem: Problem<'a>) -> Self {
        WarningButton {
            id,
            problem,
            enabled: true,
        }
    }

    /// A disabled button is not drawn: a row drawn disabled says why in its tooltip.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Its side, in points: a field high.
    pub fn side(t: &Theme) -> f32 {
        t.sizes.field
    }

    /// Draws it in the next free space of `ui`; returns the fix chosen this frame, by its
    /// id.
    pub fn show(self, ui: &mut Ui) -> Option<String> {
        if !self.enabled || !ui.is_enabled() {
            return None;
        }
        let t = theme(ui.ctx());
        let side = Self::side(&t);
        let (rect, _) = ui.allocate_exact_size(vec2(side, side), Sense::hover());
        let response = ui.interact(rect, self.id, Sense::click());
        let problem = self.problem;
        let colour = if problem.stops {
            t.colors.error
        } else {
            t.colors.warning
        };
        if ui.is_rect_visible(rect) {
            let size = Icon::Warning.size();
            let sign = Rect::from_center_size(rect.center(), vec2(size, size));
            Icon::Warning.paint(ui.painter(), sign, colour);
            if response.has_focus() {
                focus_ring(ui.painter(), rect, &t);
            }
        }
        response.widget_info(|| WidgetInfo::labeled(Role::Button, true, problem.text));
        // Said once, assertively, as it appears: a live region that takes no room, holding
        // the problem while it stands.
        let said = ui.interact(
            Rect::from_min_size(rect.min, egui::Vec2::ZERO),
            self.id.with("said"),
            Sense::hover(),
        );
        said.widget_info(|| WidgetInfo::labeled(Role::Status, true, problem.text));
        (ui.ctx()).accesskit_node_builder(said.id, |node| node.set_live(Live::Assertive));
        let response = response.on_hover_text(problem.text);
        pull_down(ui, &response, problem.text, || {
            (problem.fixes.iter())
                .map(|fix| Item::new(fix.id, fix.text).into())
                .collect()
        })
    }
}

/// One of a path row's choices in its ⌄'s menu: a pattern by its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PathChoice<'a> {
    /// What the menu answers with, chosen.
    pub id: &'a str,
    /// Its name, translated.
    pub name: &'a str,
    /// What it puts in the field, shown under its name; none for one that puts nothing.
    pub pattern: Option<&'a str>,
    /// Whether it is the one the field holds.
    pub chosen: bool,
    /// Why it cannot be chosen, translated: drawn disabled, the reason its tooltip.
    pub disabled: Option<&'a str>,
}

/// What a frame of a path row did.
pub struct PathRowOutput {
    /// The field's.
    pub response: Response,
    /// The whole path, once the person changed it, by typing or by choosing a variable.
    pub text: Option<String>,
    /// Whether the field's list of variables was open as the frame began
    /// ([`PathOutput::list_open`](super::PathOutput)).
    pub list_open: bool,
    /// The fix chosen in the warning button's menu, by its id.
    pub fix: Option<String>,
    /// The choice chosen in the ⌄'s menu, by its id.
    pub choice: Option<String>,
    /// The folder button was pressed.
    pub folder: bool,
}

/// A path setting's row in the Import tool's form: its label on a line of its own, its
/// field on the next, with the row's buttons after it.
pub struct PathRow<'a> {
    id: Id,
    label: &'a str,
    text: &'a str,
    variables: &'a [Variable<'a>],
    problem: Option<Problem<'a>>,
    choices: Option<(&'a str, &'a [PathChoice<'a>])>,
    /// Why its ⌄ is drawn disabled, where it is.
    choices_later: Option<&'a str>,
    folder: Option<&'a str>,
    hint: Option<&'a str>,
    tip: Option<&'a str>,
    disabled: Option<&'a str>,
}

impl<'a> PathRow<'a> {
    /// `id` must be stable across frames, the field's own; `label` is the row's words,
    /// translated, the field's name to screen readers too.
    pub fn new(id: Id, label: &'a str, text: &'a str, variables: &'a [Variable<'a>]) -> Self {
        PathRow {
            id,
            label,
            text,
            variables,
            problem: None,
            choices: None,
            choices_later: None,
            folder: None,
            hint: None,
            tip: None,
            disabled: None,
        }
    }

    /// The row's problem, if it has one: its warning button after the field, and where
    /// it stops the job, the field's text in `error`.
    pub fn problem(mut self, problem: Option<Problem<'a>>) -> Self {
        self.problem = problem;
        self
    }

    /// Its ⌄ of choices, named `name` (translated), opening a menu of `choices`.
    pub fn choices(mut self, name: &'a str, choices: &'a [PathChoice<'a>]) -> Self {
        self.choices = Some((name, choices));
        self
    }

    /// Its ⌄, named `name`, drawn disabled with nothing behind it yet, `why` its tooltip,
    /// translated, as the Import tool's Folder's until its choices come.
    pub fn choices_later(mut self, name: &'a str, why: &'a str) -> Self {
        self.choices = Some((name, &[]));
        self.choices_later = Some(why);
        self
    }

    /// What the field shows while it is empty, translated.
    pub fn hint(mut self, hint: &'a str) -> Self {
        self.hint = Some(hint);
        self
    }

    /// Its folder button (decision 81), named `name`, translated: "Choose a folder".
    pub fn folder(mut self, name: &'a str) -> Self {
        self.folder = Some(name);
        self
    }

    /// What the session says of the row, translated: the field's tooltip and its
    /// accessible description.
    pub fn tip(mut self, tip: &'a str) -> Self {
        self.tip = Some(tip);
        self
    }

    /// Drawn disabled, taking no typing and no presses, `why` its tooltip.
    pub fn disabled(mut self, why: &'a str) -> Self {
        self.disabled = Some(why);
        self
    }

    pub fn show(self, ui: &mut Ui) -> PathRowOutput {
        let t = theme(ui.ctx());
        let enabled = self.disabled.is_none() && ui.is_enabled();
        let label = label_line(ui, &t, self.label, enabled, self.disabled);
        drop(label);

        // Alt+↓ on the focused field opens its ⌄, before the field sees the keys.
        let focused = ui.memory(|m| m.has_focus(self.id));
        let open_choices = enabled
            && focused
            && self.choices.is_some()
            && self.choices_later.is_none()
            && ui.input_mut(|i| i.consume_key(Modifiers::ALT, Key::ArrowDown));
        let problem = self.problem.filter(|_| enabled);
        let buttons = usize::from(problem.is_some())
            + usize::from(self.choices.is_some())
            + usize::from(self.folder.is_some());
        let mut fix = None;
        let mut choice = None;
        let mut folder = false;
        let id = self.id;
        let later = self.choices_later;
        let hint = self.hint;
        let output = ui
            .scope(|ui| {
                ui.spacing_mut().item_spacing.x = t.spacing.xs;
                let side = t.sizes.field;
                let room = buttons as f32 * (side + t.spacing.xs);
                let width = (ui.available_width() - room).max(t.sizes.value_column);
                let mut field = PathField::new(id, self.label, self.text, self.variables);
                if let Some(hint) = hint {
                    field = field.hint(hint);
                }
                field
                    .width(width)
                    .problem(problem.is_some_and(|p| p.stops))
                    .enabled(enabled)
                    .beside(|ui| {
                        if let Some(problem) = problem {
                            fix = WarningButton::new(id.with("warning"), problem).show(ui);
                        }
                        match (self.choices, later) {
                            (Some((name, _)), Some(why)) => {
                                IconButton::new(Icon::ChevronDown, name)
                                    .size(side)
                                    .tip(why)
                                    .enabled(false)
                                    .show(ui);
                            }
                            (Some((name, choices)), None) => {
                                choice =
                                    choices_button(ui, &t, name, choices, enabled, open_choices);
                            }
                            (None, _) => {}
                        }
                        if let Some(name) = self.folder {
                            folder = IconButton::new(Icon::ChooseFolder, name)
                                .size(side)
                                .enabled(enabled)
                                .show(ui)
                                .clicked();
                        }
                    })
                    .show(ui)
            })
            .inner;
        let response = described(ui, output.response, problem, self.tip, self.disabled);
        PathRowOutput {
            response,
            text: output.text,
            list_open: output.list_open,
            fix,
            choice,
            folder,
        }
    }
}

/// The ⌄ of a path row's choices, named `name`: an icon button a field high with no plate,
/// opening the menu of `choices`, as it does at once where `asked`, by Alt+↓ on the field.
/// Returns the choice chosen, by its id.
fn choices_button(
    ui: &mut Ui,
    t: &Theme,
    name: &str,
    choices: &[PathChoice],
    enabled: bool,
    asked: bool,
) -> Option<String> {
    let response = IconButton::new(Icon::ChevronDown, name)
        .size(t.sizes.field)
        .enabled(enabled)
        .show(ui);
    let entries = || -> Vec<Entry> {
        (choices.iter())
            .map(|choice| {
                let mut item = Item::new(choice.id, choice.name).radio(choice.chosen);
                if let Some(pattern) = choice.pattern {
                    item = item.detail(pattern);
                }
                if let Some(why) = choice.disabled {
                    item = item.enabled(false).tip(why);
                }
                item.into()
            })
            .collect()
    };
    let ctx = ui.ctx().clone();
    let id = popup::button_menu(response.id);
    if asked && !popup::is_open(&ctx, id) {
        let request = Request::pull_down(response.rect, name, entries()).opened_by(response.id);
        popup::open(&ctx, id, request);
    }
    pull_down(ui, &response, name, entries)
}

/// What a frame of a text field row did.
pub struct TextRowOutput {
    /// The field's.
    pub response: Response,
    /// The whole text, once the person changed it.
    pub text: Option<String>,
    /// Return ended the typing this frame: a sheet presses its primary button.
    pub returned: bool,
    /// The fix chosen in the warning button's menu, by its id.
    pub fix: Option<String>,
}

/// A text setting's row: its label at the start of one line and its field at the end.
pub struct TextFieldRow<'a> {
    id: Id,
    label: &'a str,
    text: &'a str,
    hint: Option<&'a str>,
    problem: Option<Problem<'a>>,
    tip: Option<&'a str>,
    disabled: Option<&'a str>,
}

impl<'a> TextFieldRow<'a> {
    /// `id` must be stable across frames, the field's own; `label` is the row's words,
    /// translated, the field's name to screen readers too.
    pub fn new(id: Id, label: &'a str, text: &'a str) -> Self {
        TextFieldRow {
            id,
            label,
            text,
            hint: None,
            problem: None,
            tip: None,
            disabled: None,
        }
    }

    /// What the field shows while it is empty, translated.
    pub fn hint(mut self, hint: &'a str) -> Self {
        self.hint = Some(hint);
        self
    }

    /// The row's problem, if it has one: its warning button before the field, and where
    /// it stops the job, the field's text in `error`.
    pub fn problem(mut self, problem: Option<Problem<'a>>) -> Self {
        self.problem = problem;
        self
    }

    /// What the session says of the row, translated: the field's tooltip and its
    /// accessible description.
    pub fn tip(mut self, tip: &'a str) -> Self {
        self.tip = Some(tip);
        self
    }

    /// Drawn disabled, taking no typing, `why` its tooltip.
    pub fn disabled(mut self, why: &'a str) -> Self {
        self.disabled = Some(why);
        self
    }

    pub fn show(self, ui: &mut Ui) -> TextRowOutput {
        let t = theme(ui.ctx());
        let enabled = self.disabled.is_none() && ui.is_enabled();
        let problem = self.problem.filter(|_| enabled);
        let (line, _) = ui.allocate_exact_size(
            vec2(ui.available_width(), t.sizes.panel_line),
            Sense::hover(),
        );
        // The field's end on the content's end, never cut; the warning button before it.
        let field = TEXT_FIELD_WIDTH.min(line.width());
        let warning = problem.map_or(0.0, |_| WarningButton::side(&t) + t.spacing.xs);
        let ends = Rect::from_min_max(pos2(line.right() - field - warning, line.top()), line.max);
        let start = Rect::from_min_max(
            line.min,
            pos2((ends.left() - t.spacing.m).max(line.left()), line.bottom()),
        );
        let mut words = ui.new_child(UiBuilder::new().max_rect(start));
        label_in(&mut words, &t, start, self.label, enabled, self.disabled);
        let mut end = ui.new_child(
            UiBuilder::new()
                .max_rect(ends)
                .layout(Layout::left_to_right(Align::Center)),
        );
        end.spacing_mut().item_spacing.x = t.spacing.xs;
        let fix =
            problem.and_then(|p| WarningButton::new(self.id.with("warning"), p).show(&mut end));
        let mut text_field = TextField::new(self.id, self.label, self.text)
            .width(field)
            .problem(problem.is_some_and(|p| p.stops))
            .enabled(enabled);
        if let Some(hint) = self.hint {
            text_field = text_field.hint(hint);
        }
        let (response, text) = text_field.show(&mut end);
        let returned = response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        let response = described(ui, response, problem, self.tip, self.disabled);
        TextRowOutput {
            response,
            text,
            returned,
            fix,
        }
    }
}

/// A row's label on a line of its own, in the body text, cut at its end with … and whole
/// in its tooltip; disabled, its reason the tooltip.
fn label_line(
    ui: &mut Ui,
    t: &Theme,
    label: &str,
    enabled: bool,
    disabled: Option<&str>,
) -> Response {
    let (line, _) = ui.allocate_exact_size(
        vec2(ui.available_width(), t.sizes.panel_line),
        Sense::hover(),
    );
    label_in(ui, t, line, label, enabled, disabled)
}

/// `label` in `rect`, as [`label_line`] draws it.
fn label_in(
    ui: &mut Ui,
    t: &Theme,
    rect: Rect,
    label: &str,
    enabled: bool,
    disabled: Option<&str>,
) -> Response {
    let colour = if enabled {
        t.colors.text
    } else {
        t.colors.text_disabled
    };
    let font = FontId::proportional(t.text.body);
    let galley = (ui.painter()).layout_job(truncated(label, font, rect.width(), colour));
    let cut = galley.elided;
    let at = pos2(rect.left(), rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(at, galley, colour);
    let response = ui.interact(
        rect,
        ui.scope_id().with(("row label", label)),
        Sense::hover(),
    );
    let tip: Vec<&str> = [cut.then_some(label), disabled]
        .into_iter()
        .flatten()
        .collect();
    if tip.is_empty() {
        response
    } else {
        let tip = tip.join("\n");
        response.on_hover_text(&tip)
    }
}

/// The field's `response`, its accessible description and tooltip: the session's words
/// for the row, then its problem; a disabled row's reason in their place.
fn described(
    ui: &Ui,
    response: Response,
    problem: Option<Problem>,
    tip: Option<&str>,
    disabled: Option<&str>,
) -> Response {
    let lines: Vec<&str> = match disabled {
        Some(why) => vec![why],
        None => [tip, problem.map(|p| p.text)]
            .into_iter()
            .flatten()
            .collect(),
    };
    if lines.is_empty() {
        return response;
    }
    let said = lines.join("\n");
    ui.ctx()
        .accesskit_node_builder(response.id, |node| node.set_description(said.clone()));
    response.on_hover_text(&said).on_disabled_hover_text(&said)
}
