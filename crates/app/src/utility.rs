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

use apertrust_engine::session::{Command, ControlValue};
use apertrust_tools::utility::UtilityDefinition;
use apertrust_tools::{Reset, Row, SettingRow, ToolDefinition, Widget, messages};
use apertrust_ui::components as parts;
use apertrust_ui::{Translator, translator};
use eframe::egui;

use crate::actions::{self, State};
use crate::mirror::Mirror;
use crate::settings::{Settings, ToolSettings};
use crate::tools::{self, Change, Head, Note, Pressed, Rows};

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

    /// The value its setting row `row` shows, one naming its key in full, where it keeps
    /// it: its settings are the settings store's, never the tools' (decision 93). None
    /// for one not kept yet, drawn disabled at its default.
    fn setting(&self, _row: &SettingRow) -> Option<ControlValue> {
        None
    }

    /// Its setting `key`, named in full, set to `value`, as the app routes a change of its
    /// setting row to it (decision 93).
    fn set(&mut self, _key: &str, _value: &ControlValue) {}

    /// Whether `--conf` gives its setting `key` for this run (decision 43): its row is
    /// then drawn disabled at the value given, saying so, and a change of it is refused.
    fn given(&self, _key: &str) -> bool {
        false
    }

    /// The options of its choice setting `row` that cannot be chosen now, by index, and
    /// why, in `words`.
    fn disabled(&self, _row: &SettingRow, _words: &Translator) -> Option<(Vec<usize>, String)> {
        None
    }

    /// What `row`'s tooltip says now, in `words`, in place of its promise, and its
    /// description.
    fn tip(&self, _row: &Row, _words: &Translator) -> Option<String> {
        None
    }

    /// The note under `row`, in `words`.
    fn note(&self, _row: &Row, _words: &Translator) -> Option<Note> {
        None
    }

    /// The fix of the note under `row` was pressed.
    fn fix(&mut self, _row: &Row) {}

    /// How many settings its custom row `widget` shows off their defaults, which a
    /// collapsible group holding it counts in its "N set".
    fn set_in(&self, _widget: Widget) -> usize {
        0
    }

    /// The spacing between its page's rows and their parts, where it is not the style's:
    /// Export's rows stack with no gap between them, as its custom rows have them.
    fn spacing(&self, _ui: &egui::Ui) -> Option<egui::Vec2> {
        None
    }
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
    let utility = tool.definition();
    if let Some(spacing) = tool.spacing(ui) {
        ui.spacing_mut().item_spacing = spacing;
    }
    let page = Page {
        words,
        settings,
        utility,
        drawn: RefCell::new((tool, cx, Vec::new())),
    };
    let mut changes = Vec::new();
    tools::page(ui, &page, 0, &mut changes);
    let (_, _, actions) = page.drawn.into_inner();
    (changes, actions)
}

/// Why a utility's row is disabled where `--conf` gives its setting for the run, in
/// `words`, as the Settings dialog says it of such a row (decision 43).
pub fn overridden(words: &Translator) -> String {
    let mut args = apertrust_ui::i18n::Args::new();
    args.set("flag", apertrust_base::settings::CONF_FLAG);
    words.text_with("settings-overridden", Some(&args))
}

/// A utility's page as the one page drawer draws it (decision 93): its words, its
/// declaration, its collapsible groups open as the settings keep them, its settings as it
/// keeps them, and the utility, whose body draws its custom rows, with what they asked
/// for.
struct Page<'t, 'c, T: UtilityTool> {
    words: &'c Translator,
    settings: &'c Settings,
    utility: &'static UtilityDefinition,
    drawn: RefCell<(&'t mut T, &'t mut Context<'c>, Vec<T::Action>)>,
}

impl<T: UtilityTool> Page<'_, '_, T> {
    /// How many settings `rows` show now off their defaults, those inside them included:
    /// its setting rows', and its custom rows' as the utility counts them.
    fn count(&self, rows: &[Row]) -> usize {
        let mut set = 0;
        for row in rows {
            match row {
                Row::WhenSetting { setting, is, rows } => {
                    if tools::follows_setting(self, setting, is) {
                        set += self.count(rows);
                    }
                }
                Row::Setting(setting) => {
                    let shown = self.setting(setting).map(|(value, _)| value);
                    set += usize::from(shown.is_some_and(|value| value != setting.default()));
                }
                Row::Custom { widget, .. } => set += self.drawn.borrow().0.set_in(*widget),
                row => set += self.count(row.rows()),
            }
        }
        set
    }
}

impl<T: UtilityTool> Rows for Page<'_, '_, T> {
    fn words(&self) -> &Translator {
        self.words
    }

    fn definition(&self) -> &ToolDefinition {
        &self.utility.tool
    }

    /// As the settings keep it, else open where its declaration says it opens at first.
    fn group_open(&self, group: &str) -> bool {
        let groups = self.settings.edit.groups.get(self.utility.tool.id);
        let kept = groups.and_then(|groups| groups.get(group));
        kept.copied().unwrap_or(self.utility.opens(group))
    }

    fn settings(&self) -> &ToolSettings {
        &self.settings.tool_settings
    }

    fn set_in(&self, rows: &[Row]) -> usize {
        self.count(rows)
    }

    /// A custom row, by the utility's body; `UtilityDefinition::check` refuses every other
    /// row the page drawer leaves to a tool's kind, each of them one that edits.
    fn own(&self, ui: &mut egui::Ui, row: &Row, _changes: &mut Vec<Change>) {
        if let Row::Custom { widget, .. } = row {
            let (tool, cx, actions) = &mut *self.drawn.borrow_mut();
            actions.extend(tool.body(*widget, ui, cx));
        }
    }

    fn title(&self, id: &str, collapsible: bool) -> String {
        self.words.text(&self.utility.title(id, collapsible))
    }

    /// A setting naming its key in full as the utility keeps it, live but where `--conf`
    /// gives it for the run, drawn disabled at that value as the Settings dialog draws such
    /// a row (decision 43), a change of it refused as the app routes it too (`main.rs`'s
    /// `utility_setting`); any other as a tool's.
    fn setting(&self, row: &SettingRow) -> Option<(ControlValue, bool)> {
        match row.key {
            None => tools::tool_setting(&self.settings.tool_settings, self.utility.tool.id, row),
            Some(key) => {
                let tool = &self.drawn.borrow().0;
                Some((tool.setting(row)?, !tool.given(key)))
            }
        }
    }

    fn disabled(&self, row: &SettingRow) -> Option<(Vec<usize>, String)> {
        self.drawn.borrow().0.disabled(row, self.words)
    }

    /// The utility's, but for a setting `--conf` gives for the run, which says so.
    fn tip(&self, row: &Row) -> Option<String> {
        let tool = &self.drawn.borrow().0;
        if let Row::Setting(SettingRow { key: Some(key), .. }) = row
            && tool.given(key)
        {
            return Some(overridden(self.words));
        }
        tool.tip(row, self.words)
    }

    fn note(&self, row: &Row) -> Option<Note> {
        self.drawn.borrow().0.note(row, self.words)
    }

    fn fixed(&self, row: &Row) {
        self.drawn.borrow_mut().0.fix(row);
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
    use apertrust_tools::{ControlKind, Reset, TOOLS, Tab, Track, Word};
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
        titles: &[],
        open: &[],
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

    /// Declared words: On conflict's, with options named alike in every language.
    const CONFLICT: apertrust_tools::SettingWords = apertrust_tools::SettingWords {
        label: "export-on-conflict",
        full: None,
        promise: None,
        options: &[
            Word::Message("export-conflict-unique"),
            Word::Message("export-conflict-overwrite"),
            Word::Name("Skip it"),
            Word::Name("Keep both"),
        ],
    };
    const UPSCALE: apertrust_tools::SettingWords = apertrust_tools::SettingWords {
        label: "export-upscale",
        full: None,
        promise: None,
        options: &[
            Word::Message("export-option-no"),
            Word::Message("export-option-yes"),
        ],
    };

    /// A utility whose settings it keeps itself under keys in full, with words of its
    /// own: a label and a group titled by it, the group open at first, holding a choice, a
    /// boolean of No and Yes and a custom row with a note under it.
    static KEEPER: UtilityDefinition = UtilityDefinition {
        tool: ToolDefinition {
            id: "keeper",
            tabs: &[Tab {
                id: None,
                rows: &[
                    Row::Label { id: "first" },
                    Row::Group {
                        id: "more",
                        rows: &[
                            Row::Setting(SettingRow {
                                id: "conflict",
                                kind: ControlKind::Choice {
                                    options: &["unique", "overwrite", "skip", "keep"],
                                    default: 0,
                                    unported: &["keep"],
                                },
                                decimals: 0,
                                track: Track::Plain,
                                key: Some("keeper.conflict"),
                                words: Some(&CONFLICT),
                            }),
                            Row::Setting(SettingRow {
                                id: "upscale",
                                kind: ControlKind::Bool { default: false },
                                decimals: 0,
                                track: Track::Plain,
                                key: Some("keeper.upscale"),
                                words: Some(&UPSCALE),
                            }),
                            Row::Custom {
                                widget: Widget::Scopes,
                                controls: &[],
                            },
                        ],
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
        titles: &[
            ("first", "export-storage-options"),
            ("more", "export-global-options"),
        ],
        open: &["more"],
    };

    /// Keeps its two settings, says Overwrite cannot be chosen now and why, while Keep
    /// both is still to come, puts a tip on the
    /// choice and a note with a fix under its custom row, and counts one setting there.
    #[derive(Default)]
    struct Keeper {
        conflict: u32,
        upscale: bool,
        fixed: bool,
        /// The setting `--conf` gives for the run, if one.
        given: Option<&'static str>,
    }

    impl UtilityTool for Keeper {
        type Action = ();

        fn definition(&self) -> &'static UtilityDefinition {
            &KEEPER
        }

        fn body(&mut self, _: Widget, ui: &mut egui::Ui, _: &mut Context<'_>) -> Vec<()> {
            ui.label("graph");
            Vec::new()
        }

        fn setting(&self, row: &SettingRow) -> Option<ControlValue> {
            match row.key? {
                "keeper.conflict" => Some(ControlValue::Choice(self.conflict)),
                "keeper.upscale" => Some(ControlValue::Bool(self.upscale)),
                _ => None,
            }
        }

        fn set(&mut self, key: &str, value: &ControlValue) {
            match (key, value) {
                ("keeper.conflict", ControlValue::Choice(at)) => self.conflict = *at,
                ("keeper.upscale", ControlValue::Bool(on)) => self.upscale = *on,
                _ => {}
            }
        }

        fn disabled(&self, row: &SettingRow, _: &Translator) -> Option<(Vec<usize>, String)> {
            (row.id == "conflict").then(|| (vec![1], "Not on this computer".to_owned()))
        }

        fn tip(&self, row: &Row, _: &Translator) -> Option<String> {
            matches!(row, Row::Setting(SettingRow { id: "conflict", .. }))
                .then(|| "What happens to a file there".to_owned())
        }

        fn note(&self, row: &Row, _: &Translator) -> Option<Note> {
            matches!(row, Row::Custom { .. }).then(|| Note {
                text: "Something stops it".to_owned(),
                fix: (!self.fixed).then(|| "Fix it".to_owned()),
            })
        }

        fn fix(&mut self, _: &Row) {
            self.fixed = true;
        }

        fn given(&self, key: &str) -> bool {
            self.given == Some(key)
        }

        fn set_in(&self, _: Widget) -> usize {
            1
        }
    }

    /// The page drawer draws a utility's settings by their keys in full with their own
    /// words, its titles its own, its group open at first, an option disabled with the
    /// utility's reason, a tooltip and a note with its fix from the utility, a boolean as
    /// No and Yes, and counts the settings set in a closed group; a change is reported by
    /// its key, for the app to route to the utility (decision 93).
    #[test]
    fn a_utility_s_settings_are_drawn_by_their_keys_with_its_words_tips_and_notes() {
        use egui::accesskit::Role;
        use egui_kittest::kittest::NodeT as _;
        struct Scene {
            settings: Settings,
            keeper: Keeper,
            changes: Vec<Change>,
        }
        let mut harness = Harness::builder()
            .with_size(egui::vec2(320.0, 400.0))
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
                    let (changes, _) = rows(ui, &mut scene.keeper, &mut cx);
                    // As the app routes them: the utility's settings to it, the groups to
                    // the settings.
                    for change in changes {
                        match &change {
                            Change::Setting { setting, value, .. } => {
                                scene.keeper.set(setting, value);
                            }
                            Change::Group { tool, group } => {
                                let groups = scene.settings.edit.groups.entry(tool.to_string());
                                let open = groups.or_default().entry(group.to_string());
                                let open = open.or_insert(true);
                                *open = !*open;
                            }
                            _ => {}
                        }
                        scene.changes.push(change);
                    }
                },
                Scene {
                    settings: Settings::default(),
                    keeper: Keeper::default(),
                    changes: Vec::new(),
                },
            );
        harness.run();
        harness.get_by_label("Storage options");
        let group = harness.get_by_role_and_label(Role::Button, "Global options");
        assert_eq!(group.accesskit_node().data().is_expanded(), Some(true));
        let conflict = harness.get_by_role_and_label(Role::ComboBox, "On conflict");
        let tip = conflict.accesskit_node().description();
        assert_eq!(tip.as_deref(), Some("What happens to a file there"));
        harness.get_by_label("Something stops it");
        harness
            .get_by_role_and_label(Role::Button, "Fix it")
            .click();
        harness.run();
        assert!(harness.state().keeper.fixed);
        assert!(harness.query_by_label("Fix it").is_none());
        // Overwrite can't be chosen now, nor Keep both, still to come, each saying its own
        // reason; Skip it, a name, can.
        harness
            .get_by_role_and_label(Role::ComboBox, "On conflict")
            .click();
        harness.run();
        for (option, why) in [
            ("Overwrite", "Not on this computer"),
            ("Keep both", "Coming in a later version"),
        ] {
            let item = harness.get_by_role_and_label(Role::MenuItemCheckBox, option);
            assert!(item.accesskit_node().is_disabled(), "{option}");
            item.hover();
            harness.run();
            harness.run();
            assert!(harness.query_by_label(why).is_some(), "{option}: {why}");
        }
        harness
            .get_by_role_and_label(Role::MenuItemCheckBox, "Skip it")
            .click();
        harness.run();
        let upscale = harness.get_by_role_and_label(Role::ComboBox, "Allow upscaling");
        upscale.click();
        harness.run();
        harness
            .get_by_role_and_label(Role::MenuItemCheckBox, "Yes")
            .click();
        harness.run();
        let keeper = &harness.state().keeper;
        assert_eq!((keeper.conflict, keeper.upscale), (2, true));
        let keys: Vec<&str> = (harness.state().changes.iter())
            .filter_map(|change| match change {
                Change::Setting { setting, .. } => Some(*setting),
                _ => None,
            })
            .collect();
        assert_eq!(keys, ["keeper.conflict", "keeper.upscale"]);
        // Closed, the group counts its two settings set and its custom row's one.
        harness
            .get_by_role_and_label(Role::Button, "Global options")
            .click();
        harness.run();
        let group = harness.get_by_role_and_label(Role::Button, "Global options");
        assert_eq!(
            group.accesskit_node().description().as_deref(),
            Some("3 set")
        );
        // A setting `--conf` gives for the run is drawn disabled at its value, saying so;
        // the other stays live.
        harness.state_mut().keeper.given = Some("keeper.upscale");
        harness
            .get_by_role_and_label(Role::Button, "Global options")
            .click();
        harness.run();
        let upscale = harness.get_by_role_and_label(Role::ComboBox, "Allow upscaling");
        assert!(upscale.accesskit_node().is_disabled());
        assert_eq!(upscale.accesskit_node().value().as_deref(), Some("Yes"));
        let why = upscale.accesskit_node().description();
        assert_eq!(why.as_deref(), Some("Set by --conf for this run"));
        let conflict = harness.get_by_role_and_label(Role::ComboBox, "On conflict");
        assert!(!conflict.accesskit_node().is_disabled());
    }
}
