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
//! - **One page drawer** ([`crate::tools::page`]) draws a utility's page as it draws a
//!   processing tool's ([`rows`]): labels, groups, settings and actions itself, and each
//!   custom row by the utility's `body`. `UtilityDefinition::check` refuses every row that
//!   edits, so the drawer leaves a utility nothing else. History's tab draws its page with
//!   [`rows`] under a header of its own ([`header`]), which stays at the tab's top.

use std::cell::RefCell;

use apertrust_engine::session::Command;
use apertrust_tools::utility::UtilityDefinition;
use apertrust_tools::{Reset, Row, ToolDefinition, Widget, messages};
use apertrust_ui::components as parts;
use apertrust_ui::{Translator, translator};
use eframe::egui;

use crate::actions::{self, State};
use crate::mirror::Mirror;
use crate::settings::{Settings, ToolSettings};
use crate::tools::{self, Change, Head, Pressed, Rows};

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
/// ↺ carried out; what the page drawer's rows asked for, a collapsible group opened or
/// closed; and what its custom rows asked for.
#[derive(Debug)]
pub struct Drawn<A> {
    pub pressed: Vec<Pressed>,
    pub changes: Vec<Change>,
    pub actions: Vec<A>,
}

/// Draws `tool`'s section, its header from its declaration ([`tools::header`]) as `head`
/// has it, ↺ drawn disabled unless `resettable`, and, while open, its page ([`rows`]).
/// Its own ↺ is carried out here ([`UtilityTool::reset`]).
pub fn section<T: UtilityTool>(
    ui: &mut egui::Ui,
    tool: &mut T,
    cx: &mut Context<'_>,
    head: Head,
    resettable: bool,
) -> Drawn<T::Action> {
    let definition = &tool.definition().tool;
    let [title, reset, name, later] = titles(cx.words, definition);
    let about = about(cx.words, definition);
    let photo = head.photo;
    let (mut changes, mut actions) = (Vec::new(), Vec::new());
    let section = tools::header(
        definition,
        &title,
        &reset,
        [&name, &later],
        resettable,
        about,
        head,
    );
    let out = section.show(ui, |ui| (changes, actions) = rows(ui, tool, cx));
    let mut pressed = tools::pressed(ui, definition, &out, photo);
    if pressed.contains(&Pressed::Reset) {
        tool.reset(cx);
    }
    pressed.retain(|pressed| *pressed != Pressed::Reset);
    Drawn {
        pressed,
        changes,
        actions,
    }
}

/// Draws `tool`'s page with the one page drawer every tool's is drawn with
/// ([`tools::page`]): its rows as its declaration lists them, each custom row by its
/// [`UtilityTool::body`]. Returns what the drawer's rows and the custom rows asked for.
pub fn rows<T: UtilityTool>(
    ui: &mut egui::Ui,
    tool: &mut T,
    cx: &mut Context<'_>,
) -> (Vec<Change>, Vec<T::Action>) {
    let (words, settings) = (cx.words, cx.settings);
    let definition = &tool.definition().tool;
    let page = Page {
        words,
        settings,
        definition,
        drawn: RefCell::new((tool, cx, Vec::new())),
    };
    let mut changes = Vec::new();
    tools::page(ui, &page, 0, &mut changes);
    let (_, _, actions) = page.drawn.into_inner();
    (changes, actions)
}

/// A utility's page as the one page drawer draws it (decision 93): its words, its
/// declaration, its collapsible groups open as the settings keep them, and the utility,
/// whose body draws its custom rows, with what they asked for.
struct Page<'t, 'c, T: UtilityTool> {
    words: &'c Translator,
    settings: &'c Settings,
    definition: &'static ToolDefinition,
    drawn: RefCell<(&'t mut T, &'t mut Context<'c>, Vec<T::Action>)>,
}

impl<T: UtilityTool> Rows for Page<'_, '_, T> {
    fn words(&self) -> &Translator {
        self.words
    }

    fn definition(&self) -> &ToolDefinition {
        self.definition
    }

    fn group_open(&self, group: &str) -> bool {
        let groups = self.settings.edit.groups.get(self.definition.id);
        let kept = groups.and_then(|groups| groups.get(group));
        kept.copied().unwrap_or(tools::GROUP_OPEN)
    }

    fn settings(&self) -> &ToolSettings {
        &self.settings.tool_settings
    }

    /// A custom row, by the utility's body; `UtilityDefinition::check` refuses every other
    /// row the page drawer leaves to a tool's kind, each of them one that edits.
    fn own(&self, ui: &mut egui::Ui, row: &Row, _changes: &mut Vec<Change>) {
        if let Row::Custom { widget, .. } = row {
            let (tool, cx, actions) = &mut *self.drawn.borrow_mut();
            actions.extend(tool.body(*widget, ui, cx));
        }
    }
}

/// Draws the header of the utility `definition` alone, as `head` has it, for a section
/// whose header stays at its tab's top while its page scrolls under it, as History's
/// does: the caller draws the page apart while it is open, with [`rows`]. Returns what it
/// asked for, its own ↺ among it, for the caller to carry out.
pub fn header(
    ui: &mut egui::Ui,
    definition: &ToolDefinition,
    head: Head,
    resettable: bool,
) -> Vec<Pressed> {
    let words = translator(ui.ctx());
    let [title, reset, name, later] = titles(&words, definition);
    let about = about(&words, definition);
    let photo = head.photo;
    let section = tools::header(
        definition,
        &title,
        &reset,
        [&name, &later],
        resettable,
        about,
        head,
    );
    let out = section.show_header(ui);
    tools::pressed(ui, definition, &out, photo)
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

/// The utility `definition`'s title, `section-<id>`, its ↺'s name, and its presets
/// button's name and tooltip where it has one, else empty.
fn titles(words: &Translator, definition: &ToolDefinition) -> [String; 4] {
    let title = words.text(&format!("section-{}", definition.id));
    let reset = tools::reset_name(words, definition, &title);
    // Its presets button's words, where it has one.
    let [name, later] = if definition.presets {
        messages::utility_presets(definition.id).map(|id| words.text(&id))
    } else {
        Default::default()
    };
    [title, reset, name, later]
}

#[cfg(test)]
mod tests {
    use apertrust_tools::utility::UTILITIES;
    use apertrust_tools::{Reset, TOOLS, Tab};
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    use super::*;

    /// Every utility's page holds no row that edits, which leaves the page drawer nothing
    /// of a processing tool's to draw for it; every header follows its declaration: a
    /// processing tool's ↺ its own, with presets, a utility's ↺ a registry action's, its
    /// own or none, with presets where it declares them, as the header draws them.
    #[test]
    fn every_page_edits_nothing_and_every_header_is_one_the_header_draws() {
        for utility in UTILITIES {
            let tool = &utility.tool;
            assert_eq!(utility.check(), Ok(()), "{}", tool.id);
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

    /// A utility whose page is a label, a custom row, then a collapsible group holding
    /// another custom row.
    static SAMPLE: UtilityDefinition = UtilityDefinition {
        tool: ToolDefinition {
            id: "sample",
            tabs: &[Tab {
                id: None,
                rows: &[
                    Row::Label { id: "first" },
                    Row::Custom {
                        widget: Widget::Scopes,
                        controls: &[],
                    },
                    Row::Group {
                        id: "more",
                        rows: &[Row::Custom {
                            widget: Widget::History,
                            controls: &[],
                        }],
                    },
                ],
            }],
            before: &[],
            after: &[],
            reset: Reset::None,
            presets: false,
            menus: &[],
        },
        measures: None,
    };

    /// Draws each custom row as a line naming its widget, and says it drew it.
    struct Sample;

    impl UtilityTool for Sample {
        type Action = Widget;

        fn definition(&self) -> &'static UtilityDefinition {
            &SAMPLE
        }

        fn body(&mut self, widget: Widget, ui: &mut egui::Ui, _: &mut Context<'_>) -> Vec<Widget> {
            ui.label(format!("{widget:?}"));
            vec![widget]
        }
    }

    /// The one page drawer draws a utility's rows in their order, its label and its group
    /// itself, open as the settings keep it, and each custom row by its body.
    #[test]
    fn the_page_drawer_draws_a_utility_s_rows_and_its_custom_rows_by_its_body() {
        struct Scene {
            settings: Settings,
            drawn: (Vec<Change>, Vec<Widget>),
        }
        let mut settings = Settings::default();
        let open = [("more".to_owned(), true)].into_iter().collect();
        settings.edit.groups.insert("sample".to_owned(), open);
        let mut harness = Harness::builder()
            .with_size(egui::vec2(320.0, 300.0))
            .with_context_setup(|ctx| apertrust_ui::install(ctx, apertrust_ui::Theme::default()))
            .build_ui_state(
                |ui, scene: &mut Scene| {
                    let mirror = crate::mirror::tests::mirror();
                    let (words, mut commands) = (translator(ui.ctx()), Vec::new());
                    let mut cx = Context {
                        mirror: &mirror,
                        settings: &scene.settings,
                        words: &words,
                        commands: &mut commands,
                    };
                    scene.drawn = rows(ui, &mut Sample, &mut cx);
                },
                Scene {
                    settings,
                    drawn: Default::default(),
                },
            );
        harness.run();
        let (changes, drawn) = &harness.state().drawn;
        assert_eq!(drawn, &[Widget::Scopes, Widget::History]);
        assert!(changes.is_empty());
        let scopes = harness.get_by_label("Scopes").rect();
        let history = harness.get_by_label("History").rect();
        assert!(scopes.bottom() <= history.top());
        // Closed as the settings keep it, the group hides its custom row.
        harness.state_mut().settings.edit.groups.clear();
        harness.run();
        assert_eq!(harness.state().drawn.1, [Widget::Scopes]);
    }
}
