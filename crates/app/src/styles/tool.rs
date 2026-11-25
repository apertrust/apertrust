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

//! The Styles tool (decision 106; `docs/gui.md`, The Styles tool), a utility of the Photos
//! tab, drawn as Collections is: its header, then its rows on the panel, with no frame.
//!
//! - **The styles applied** to the active photo, newest first, from `Values::styles`: a
//!   grey ✓ before each name, then one empty row. A click or a right-click on one opens one
//!   command, Clear (`Command::ClearStyle`).
//! - **Find style,** a sunken field that narrows the tree to the styles whose names hold
//!   its text, their folders open; ↓ moves into the tree, Return applies the first style
//!   it shows, × or Escape clears it.
//! - **The tree** ([`read`]) of two root folders, always drawn and open at first: Custom
//!   Styles, the person's own, `<user data>/styles/*.apstyle`, "No styles yet." while it
//!   holds none, then Built-in Styles, the styles Apertrust ships
//!   (`apertrust_library::presets::builtin_styles`), named by their Fluent messages. A `|`
//!   in a name makes folders inside its root, as darktable's do, each level its folders
//!   first, then its styles, by name in the file browser's order. Every style row keeps a
//!   chevron's width before its name for an accent ✓ while the style is applied. A click
//!   applies the style (`Command::ApplyStyle`) as `styles.mode` says; a right-click,
//!   Shift+F10 or the menu key opens Apply or Clear, then, for a person's style, Rename
//!   Style… and Delete Style…, F2 renaming and Delete asking, and for a built-in, which is
//!   read-only, Copy to Custom Styles…. A row that cannot apply, a newer Apertrust's file
//!   or with no photo taking edits, is drawn disabled, its tooltip saying why, and keeps
//!   its menu, F2 and Delete.
//! - **Nothing is chosen:** the tool has no current style or selection.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use apertrust_engine::session::{Command, StyleMode, StyleState, tool_values};
use apertrust_engine::sidecar::Parked;
use apertrust_library::listing::browser_order;
use apertrust_library::presets::{Stored, builtin_styles};
use apertrust_tools::utility::UtilityDefinition;
use apertrust_tools::{ProcessingTool, ToolValues, Widget, messages};
use apertrust_ui::components::settings::Search;
use apertrust_ui::components::{context_click, menu_asked, menu_of};
use apertrust_ui::i18n::Args;
use apertrust_ui::popup::{Entry, Item, Submenu};
use apertrust_ui::{Icon, Translator, theme};
use eframe::egui::{self, Rect, Sense, UiBuilder, vec2};

use crate::tools::Pressed;
use crate::tree_row::{LEVEL_INDENT, TreeRow};
use crate::utility::{Context, UtilityTool};

/// The tool's id, and its declaration's.
pub const ID: &str = "styles";

/// The two root folders' paths, in the tree's order: Custom Styles, then Built-in Styles.
/// Every other folder's path is its root's, then its names, each after a `|`.
pub const CUSTOM: &str = "custom";
pub const BUILT_IN: &str = "built-in";

/// What the styles applied record as a built-in style's file, before its id: no file name
/// ([`apertrust_library::presets::file_name`]) holds a `:`, so it is never a person's.
pub use apertrust_engine::history::BUILT_IN_FILE;

/// One style file as the tree lists it.
#[derive(Clone, Debug, PartialEq)]
pub struct Listed {
    /// Its name as written, trimmed, a `|` between its folders and its own name.
    pub name: String,
    pub path: PathBuf,
    /// Its tools this build has, each with its values, migrated and clamped
    /// (`session::tool_values`); `None` where a newer Apertrust wrote the file, or where
    /// none of its entries applies, which is then drawn disabled.
    pub tools: Option<Vec<(String, ToolValues)>>,
    /// Why each of its entries this build cannot apply is left out: a newer tool version,
    /// a tool it does not have or has retired.
    pub left_out: Vec<Parked>,
    /// Whether a newer Apertrust wrote the file itself (`Stored::newer`): it is never
    /// rewritten, so Rename Style… is not offered; a file whose tools are all retired can
    /// still be renamed.
    pub newer: bool,
    /// A built-in style's id (`data/styles/<id>.apstyle`): read-only, with no file of the
    /// person's; `None` for the person's own.
    pub builtin: Option<&'static str>,
}

impl Listed {
    /// Its file's name, as the styles applied record it: a built-in's id after
    /// [`BUILT_IN_FILE`], `builtin:lomo`.
    pub fn file(&self) -> String {
        if let Some(id) = self.builtin {
            return format!("{BUILT_IN_FILE}{id}");
        }
        self.path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    /// Its own name, after its folders'.
    pub fn leaf(&self) -> &str {
        leaf(&self.name)
    }

    /// Its folders, outermost first.
    fn folders(&self) -> Vec<&str> {
        let mut parts: Vec<&str> = self.name.split('|').map(str::trim).collect();
        parts.pop();
        parts
    }
}

/// The own name of a style named `name`, after its folders'.
fn leaf(name: &str) -> &str {
    name.rsplit('|').next().unwrap_or(name).trim()
}

/// The styles built in, each named as `words` show it ([`builtin_name`]), then the styles
/// in `folder` (`styles::stored`), each of their entries made a tool's values for `tools`
/// or left out, and why.
pub fn read(folder: &Path, tools: &[&dyn ProcessingTool], words: &Translator) -> Vec<Listed> {
    let builtins = (builtin_styles().into_iter()).map(|(id, stored)| Listed {
        name: builtin_name(id, words),
        builtin: Some(id),
        ..listed(&stored, tools)
    });
    let own = (super::stored(folder).into_iter()).map(|stored| listed(&stored, tools));
    let mut styles: Vec<Listed> = builtins.chain(own).collect();
    styles.sort_by(|a, b| browser_order(&a.name, &b.name));
    styles
}

/// The name the style built in as `id` shows, the Fluent message `style-builtin-<id>`, in
/// `words`: its folders' and its own, a `|` between them.
pub fn builtin_name(id: &str, words: &Translator) -> String {
    words.text(&format!("style-builtin-{id}"))
}

/// `stored` as the tree lists it, a person's, each of its entries made a tool's values for
/// `tools` or left out, and why.
fn listed(stored: &Stored, tools: &[&dyn ProcessingTool]) -> Listed {
    let mut left_out = Vec::new();
    let values = stored.newer().is_none().then(|| {
        (stored.entries.iter())
            .filter_map(
                |entry| match tool_values(entry, tools, apertrust_tools::RETIRED) {
                    Ok(values) => Some(values),
                    Err(why) => {
                        left_out.push(why);
                        None
                    }
                },
            )
            .collect()
    });
    // None of its entries applies: parked, as a newer Apertrust's file is.
    let values = values.filter(|values: &Vec<_>| !values.is_empty() || left_out.is_empty());
    Listed {
        name: stored.name.trim().to_owned(),
        path: stored.path.clone(),
        tools: values,
        left_out,
        newer: stored.newer().is_some(),
        builtin: None,
    }
}

/// Why `style` cannot be applied, where it is parked ([`Listed::tools`]), in `words`: its
/// tools all retired, or a newer Apertrust's.
fn parked(style: &Listed, words: &Translator) -> String {
    let retired =
        !style.left_out.is_empty() && (style.left_out.iter()).all(|why| *why == Parked::Retired);
    words.text(if retired {
        "styles-parked-retired"
    } else {
        "styles-parked"
    })
}

/// The style applied, of `applied`, that `style` is: a built-in by its file, `builtin:<id>`,
/// whatever language it was applied in; a person's by its name, never a built-in's entry.
fn applied_entry<'a>(style: &Listed, applied: &'a [StyleState]) -> Option<&'a StyleState> {
    let file = style.file();
    applied.iter().find(|entry| match style.builtin {
        Some(_) => entry.file == file,
        None => entry.name == style.name && !entry.file.starts_with(BUILT_IN_FILE),
    })
}

/// The command a click on `style` sends: the style applied as `mode` says, its tools this
/// build has, under its name, or for a built-in already among the styles `applied` the
/// name it was applied under, so it is applied again rather than twice; none for a file
/// this build cannot apply.
pub fn apply(style: &Listed, mode: StyleMode, applied: &[StyleState]) -> Option<Command> {
    let name = applied_entry(style, applied).map_or(&style.name, |entry| &entry.name);
    Some(Command::ApplyStyle {
        name: name.clone(),
        file: style.file(),
        mode,
        tools: style.tools.clone()?,
        photos: Vec::new(),
    })
}

/// What the Viewer says where `style`'s entries were not all applied, in `words`: how many
/// were left out, and why, a retired tool's where every one is, else a newer Apertrust's;
/// none where none was.
pub fn left_out(style: &Listed, words: &Translator) -> Option<String> {
    (!style.left_out.is_empty()).then(|| {
        let retired = (style.left_out.iter()).all(|why| *why == Parked::Retired);
        let reason = if retired {
            "styles-reason-retired"
        } else {
            "styles-reason-newer"
        };
        let mut args = Args::new();
        args.set("count", style.left_out.len() as f64);
        args.set("name", style.name.as_str());
        args.set("reason", words.text(reason));
        words.text_with("styles-left-out", Some(&args))
    })
}

/// The command Clear sends for the style applied `name` from `file`.
pub fn clear(name: &str, file: &str) -> Command {
    Command::ClearStyle {
        name: name.to_owned(),
        file: file.to_owned(),
    }
}

/// What the tool keeps between frames: the styles read, none until it first shows, after
/// one is created, copied, renamed or deleted, or after the language changes; Find style's
/// text; and the folders open, by path, the two roots at first.
#[derive(Debug)]
pub struct Kept {
    pub listed: Option<Vec<Listed>>,
    pub find: String,
    pub open: BTreeSet<String>,
}

impl Default for Kept {
    fn default() -> Self {
        Kept {
            listed: None,
            find: String::new(),
            open: [CUSTOM, BUILT_IN].map(str::to_owned).into(),
        }
    }
}

/// What the tool asks of the app.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    /// A style applied, by its file ([`Listed::file`]), which a built-in's and a person's
    /// of the same name do not share.
    Apply(String),
    /// A style applied taken off, by the name it was applied under and its file, which a
    /// built-in's and a person's of the same name do not share.
    Clear {
        name: String,
        file: String,
    },
    Rename(String),
    Delete(String),
    /// Copy to Custom Styles…, for the built-in style of this name.
    Copy(String),
    /// A folder opened or closed, by its path: its root's, then its names, each after a `|`.
    Toggle(String),
    /// Find style's text, whole.
    Find(String),
    /// How a click applies a style, from ⋯ › Mode.
    Mode(StyleMode),
}

/// Why the tree's styles cannot be applied now, nor the applied ones cleared, in `words`:
/// with no photo open, one open for viewing only, or one still opening, which takes no
/// edit yet (`ready` false) and is said as no photo is.
pub fn unavailable(
    photo: bool,
    view_only: bool,
    ready: bool,
    words: &Translator,
) -> Option<String> {
    match (photo, view_only, ready) {
        (false, _, _) | (true, false, false) => Some(words.text("styles-no-photo")),
        (true, true, _) => Some(words.text("styles-view-only")),
        (true, false, true) => None,
    }
}

/// The Styles tool as a utility, as the frame gives it: the styles read, the styles applied
/// to the active photo, the tool's own state, whether a style can be applied now, and
/// whether the platform asks before Delete.
pub struct StylesTool<'a> {
    pub listed: &'a [Listed],
    /// The styles applied, newest first; none with no photo open.
    pub applied: Option<&'a [StyleState]>,
    pub kept: &'a Kept,
    /// Why the tree's styles cannot be applied now, nor the applied ones cleared, where
    /// they cannot.
    pub unavailable: Option<String>,
    /// The tools' titles, by id, for a style's tooltip.
    pub titles: &'a dyn Fn(&str) -> String,
    /// Whether the platform shows the alert Delete Style… asks first with: where it does
    /// not, Delete Style… is disabled, as a preset's Delete… is.
    pub alerts: bool,
}

/// The Styles tool's declaration, among the utilities (decision 93).
pub fn declared() -> &'static UtilityDefinition {
    apertrust_tools::utility::utility(ID).expect("Styles is built in")
}

/// The id of the row ↓ in Find style gives the focus, the tree's first.
fn tree_entry() -> egui::Id {
    egui::Id::unique("styles tree first")
}

impl UtilityTool for StylesTool<'_> {
    type Action = Action;

    fn definition(&self) -> &'static UtilityDefinition {
        declared()
    }

    fn body(&mut self, widget: Widget, ui: &mut egui::Ui, cx: &mut Context<'_>) -> Vec<Action> {
        let mut actions = Vec::new();
        if widget != Widget::Styles {
            return actions;
        }
        ui.spacing_mut().item_spacing.y = 0.0;
        if let Some(applied) = self.applied {
            self.applied_list(ui, applied, cx.words, &mut actions);
        }
        self.find(ui, cx.words, &mut actions);
        ui.add_space(theme(ui.ctx()).spacing.xs);
        let label = cx.words.text("styles-tree");
        ui.scope_builder(
            UiBuilder::new()
                .id_salt("styles tree")
                .accessibility_role(egui::Role::Tree)
                .accessibility_label(&label),
            |ui| self.tree(ui, cx.words, &mut actions),
        );
        actions
    }
}

/// The styles of `listed` Find style's `find` leaves: those whose names hold it, ignoring
/// case; every one where it is empty.
fn shown<'a>(listed: &'a [Listed], find: &str) -> Vec<&'a Listed> {
    let find = find.trim().to_lowercase();
    (listed.iter())
        .filter(|style| find.is_empty() || style.name.to_lowercase().contains(&find))
        .collect()
}

/// One node of the tree: a folder, by its path ([`BUILT_IN`], [`CUSTOM`]), holding its
/// children, or a style.
#[derive(Debug, PartialEq)]
enum Node<'a> {
    Folder {
        name: String,
        path: String,
        children: Vec<Node<'a>>,
    },
    Style(&'a Listed),
}

impl Node<'_> {
    fn name(&self) -> &str {
        match self {
            Node::Folder { name, .. } => name,
            Node::Style(style) => style.leaf(),
        }
    }
}

/// The tree of `styles` inside the root folder `root`, folders from the `|` in their names,
/// each level its folders first, then its styles, each by name in the file browser's order,
/// as the board draws Film over the styles.
fn tree_of<'a>(styles: &[&'a Listed], root: &str) -> Vec<Node<'a>> {
    let mut nodes: Vec<Node<'a>> = Vec::new();
    for style in styles {
        let mut level = &mut nodes;
        let mut path = root.to_owned();
        for folder in style.folders() {
            path.push('|');
            path.push_str(folder);
            let at = level
                .iter()
                .position(|node| matches!(node, Node::Folder { name, .. } if name == folder));
            let at = at.unwrap_or_else(|| {
                level.push(Node::Folder {
                    name: folder.to_owned(),
                    path: path.clone(),
                    children: Vec::new(),
                });
                level.len() - 1
            });
            let Node::Folder { children, .. } = &mut level[at] else {
                unreachable!("a folder")
            };
            level = children;
        }
        level.push(Node::Style(style));
    }
    sort(&mut nodes);
    nodes
}

fn sort(nodes: &mut [Node<'_>]) {
    let style = |node: &Node<'_>| matches!(node, Node::Style(_));
    nodes.sort_by(|a, b| (style(a).cmp(&style(b))).then(browser_order(a.name(), b.name())));
    for node in nodes {
        if let Node::Folder { children, .. } = node {
            sort(children);
        }
    }
}

/// The ids of the rows' menus' items.
const APPLY: &str = "apply";
const CLEAR: &str = "clear";
const RENAME: &str = "rename";
const DELETE: &str = "delete";
const COPY: &str = "copy";

impl StylesTool<'_> {
    /// The name `style` is applied under, where it is among the styles applied
    /// ([`applied_entry`]): a built-in's in whatever language it was applied in.
    fn applied_as(&self, style: &Listed) -> Option<&str> {
        let applied = self.applied.unwrap_or_default();
        applied_entry(style, applied).map(|entry| entry.name.as_str())
    }

    /// The tree of the styles Find style leaves, and whether its folders are all open: its
    /// two roots, titled in `words`, each drawn but while Find style finds none of its own.
    fn nodes(&self, words: &Translator) -> (Vec<Node<'_>>, bool) {
        let finding = !self.kept.find.trim().is_empty();
        let (builtins, own): (Vec<&Listed>, Vec<&Listed>) = shown(self.listed, &self.kept.find)
            .into_iter()
            .partition(|style| style.builtin.is_some());
        let roots = [
            (CUSTOM, "styles-custom", own),
            (BUILT_IN, "styles-built-in", builtins),
        ];
        let nodes = (roots.into_iter())
            .filter(|(_, _, styles)| !finding || !styles.is_empty())
            .map(|(path, title, styles)| Node::Folder {
                name: words.text(title),
                path: path.to_owned(),
                children: tree_of(&styles, path),
            })
            .collect();
        (nodes, finding)
    }

    /// Whether the folder `path` shows its rows.
    fn open(&self, path: &str, finding: bool) -> bool {
        finding || self.kept.open.contains(path)
    }

    /// The first style the tree shows, in its order, a closed folder's hidden: what Return
    /// in Find style applies.
    fn first_shown(&self, words: &Translator) -> Option<&Listed> {
        let (nodes, finding) = self.nodes(words);
        fn walk<'a>(nodes: &[Node<'a>], open: &dyn Fn(&str) -> bool) -> Option<&'a Listed> {
            nodes.iter().find_map(|node| match node {
                Node::Style(style) => Some(*style),
                Node::Folder { path, children, .. } if open(path) => walk(children, open),
                Node::Folder { .. } => None,
            })
        }
        walk(&nodes, &|path| self.open(path, finding))
    }

    /// Find style, with × at its end while it holds text: ↓ gives the tree's first row the
    /// focus, Return applies the first style the tree shows, the field keeping the focus.
    fn find(&self, ui: &mut egui::Ui, words: &Translator, actions: &mut Vec<Action>) {
        let t = theme(ui.ctx());
        let name = words.text("styles-find");
        let id = Search::id_of("styles find");
        // Whether the field had the keys before this frame's Return: a single line lets go
        // of the focus on Return, as Find Tool…'s does.
        let had_focus = ui.memory(|m| m.has_focus(id));
        let field = ui.scope(|ui| Search::new("styles find", &name, &self.kept.find).show(ui));
        if let Some(text) = field.inner {
            actions.push(Action::Find(text));
        }
        let focused = ui.memory(|m| m.has_focus(id));
        let down = || ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown));
        if focused && down() {
            ui.memory_mut(|m| m.request_focus(tree_entry()));
        }
        let returned = had_focus && !focused && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if returned {
            ui.memory_mut(|m| m.request_focus(id));
            if self.unavailable.is_none()
                && let Some(first) = self
                    .first_shown(words)
                    .filter(|style| style.tools.is_some())
            {
                actions.push(Action::Apply(first.file()));
            }
        }
        if !self.kept.find.is_empty() {
            let rect = field.response.rect;
            let side = t.sizes.icon + t.spacing.xs;
            let at = Rect::from_center_size(
                egui::pos2(rect.right() - side / 2.0 - t.spacing.xxs, rect.center().y),
                vec2(side, side),
            );
            let clear = words.text("styles-find-clear");
            let mut inside = ui.new_child(UiBuilder::new().max_rect(at));
            let pressed = apertrust_ui::components::IconButton::new(Icon::Close, &clear)
                .size(side)
                .show(&mut inside)
                .clicked();
            if pressed {
                actions.push(Action::Find(String::new()));
                ui.memory_mut(|m| m.request_focus(id));
            }
        }
    }

    /// The tree's rows, the first drawn, folder or style, taking the id ↓ in Find style
    /// gives the focus.
    fn tree(&self, ui: &mut egui::Ui, words: &Translator, actions: &mut Vec<Action>) {
        let (nodes, finding) = self.nodes(words);
        let mut first = Some(tree_entry());
        self.rows(ui, words, &nodes, (1, finding), &mut first, actions);
    }

    fn rows(
        &self,
        ui: &mut egui::Ui,
        words: &Translator,
        nodes: &[Node<'_>],
        (level, finding): (usize, bool),
        first: &mut Option<egui::Id>,
        actions: &mut Vec<Action>,
    ) {
        for node in nodes {
            match node {
                Node::Folder {
                    name,
                    path,
                    children,
                } => {
                    let open = self.open(path, finding);
                    let id = first.take();
                    let id = id.unwrap_or_else(|| ui.make_persistent_id(("styles folder", path)));
                    let (response, _) = TreeRow {
                        id: Some(id),
                        level,
                        name,
                        expanded: Some(open),
                        mark: None,
                        icon: None,
                        count: None,
                        chosen: false,
                        enabled: true,
                        role: egui::Role::TreeItem,
                        label: None,
                    }
                    .show(ui);
                    let key = |key: egui::Key| {
                        response.has_focus()
                            && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key))
                    };
                    let (left, right) = (key(egui::Key::ArrowLeft), key(egui::Key::ArrowRight));
                    if response.clicked() || (left && open) || (right && !open) {
                        actions.push(Action::Toggle(path.clone()));
                    }
                    if open && children.is_empty() {
                        empty_row(ui, &words.text("styles-empty"), level + 1);
                    } else if open {
                        self.rows(ui, words, children, (level + 1, finding), first, actions);
                    }
                }
                Node::Style(style) => {
                    let id = first.take();
                    let id =
                        id.unwrap_or_else(|| ui.make_persistent_id(("styles style", style.file())));
                    self.style_row(ui, words, style, (id, level), actions);
                }
            }
        }
    }

    /// A style's row: a click applies it where it can; its menu, F2 and Delete always work.
    fn style_row(
        &self,
        ui: &mut egui::Ui,
        words: &Translator,
        style: &Listed,
        (id, level): (egui::Id, usize),
        actions: &mut Vec<Action>,
    ) {
        let t = theme(ui.ctx());
        let applied_as = self.applied_as(style);
        let applied = applied_as.is_some();
        let parked = style.tools.is_none();
        let can_apply = !parked && self.unavailable.is_none();
        let (response, _) = TreeRow {
            id: Some(id),
            level,
            name: style.leaf(),
            expanded: None,
            mark: applied.then_some((Icon::Check, t.colors.accent_text)),
            icon: None,
            count: None,
            chosen: false,
            enabled: can_apply,
            role: egui::Role::TreeItem,
            label: None,
        }
        .show(ui);
        let tools = self.tools_tip(style);
        let tip = if parked {
            self::parked(style, words)
        } else if let Some(why) = &self.unavailable {
            why.clone()
        } else {
            tools.clone()
        };
        // Applied in its description, as screen readers hear it, then its tools, or why a
        // newer Apertrust's file holds none.
        let mut said = Vec::new();
        if applied {
            said.push(words.text("styles-applied-state"));
        }
        said.push(if parked {
            self::parked(style, words)
        } else {
            tools
        });
        let described = said.join(", ");
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_description(described);
        });
        let response = response.on_hover_text(tip);
        let mac_menu = context_click(ui.ctx(), &response);
        if response.clicked() && !mac_menu && can_apply {
            actions.push(Action::Apply(style.file()));
        }
        // A built-in is read-only: no key renames or deletes it.
        let own = style.builtin.is_none();
        let focused = own && response.has_focus();
        let pressed =
            |key: egui::Key| focused && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key));
        // A newer Apertrust's file is never rewritten, so never renamed.
        if !style.newer && pressed(egui::Key::F2) {
            actions.push(Action::Rename(style.name.clone()));
        }
        if self.alerts && (pressed(egui::Key::Delete) || pressed(egui::Key::Backspace)) {
            actions.push(Action::Delete(style.name.clone()));
        }
        let keys = menu_asked(ui.ctx(), &response);
        let asked = response.secondary_clicked() || mac_menu || keys;
        let menu = row_menu_name(style.leaf(), words);
        let entries = || {
            let first = if applied {
                Item::new(CLEAR, words.text("styles-clear")).enabled(self.unavailable.is_none())
            } else {
                Item::new(APPLY, words.text("styles-apply")).enabled(can_apply)
            };
            let items = if own {
                vec![
                    Item::new(RENAME, words.text("styles-rename")).enabled(!style.newer),
                    Item::new(DELETE, words.text("styles-delete")).enabled(self.alerts),
                ]
            } else {
                vec![Item::new(COPY, words.text("styles-copy"))]
            };
            [Entry::Item(first), Entry::Separator]
                .into_iter()
                .chain(items.into_iter().map(Entry::Item))
                .collect()
        };
        let chosen = menu_of(ui, &response, &menu, (asked, keys), entries);
        let name = style.name.clone();
        actions.extend(match chosen.as_deref() {
            Some(APPLY) => Some(Action::Apply(style.file())),
            Some(CLEAR) => applied_as.map(|name| Action::Clear {
                name: name.to_owned(),
                file: style.file(),
            }),
            Some(RENAME) => Some(Action::Rename(name)),
            Some(DELETE) => Some(Action::Delete(name)),
            Some(COPY) => Some(Action::Copy(name)),
            _ => None,
        });
    }

    /// A style's tooltip: the titles of the tools it sets, as the board writes them,
    /// "Colour balance RGB, Vignetting".
    fn tools_tip(&self, style: &Listed) -> String {
        let titles: Vec<String> = (style.tools.iter().flatten())
            .map(|(tool, _)| (self.titles)(tool))
            .collect();
        titles.join(", ")
    }

    /// The styles applied to the photo, newest first, each a row with a grey ✓ before its
    /// own name, then one empty row: a list to screen readers. A click or a right-click on
    /// one opens its one command, Clear, disabled on a photo open for viewing only.
    fn applied_list(
        &self,
        ui: &mut egui::Ui,
        applied: &[StyleState],
        words: &Translator,
        actions: &mut Vec<Action>,
    ) {
        let t = theme(ui.ctx());
        let label = words.text("styles-applied");
        let clearable = self.unavailable.is_none();
        ui.scope_builder(
            UiBuilder::new()
                .id_salt("styles applied")
                .accessibility_role(egui::Role::List)
                .accessibility_label(&label),
            |ui| {
                for style in applied {
                    let name = leaf(&style.name);
                    let mut args = Args::new();
                    args.set("name", name);
                    let (response, _) = TreeRow {
                        id: Some(ui.make_persistent_id(("styles applied", &style.name))),
                        level: 1,
                        name,
                        expanded: None,
                        mark: Some((Icon::Check, t.colors.text_secondary)),
                        icon: None,
                        count: None,
                        chosen: false,
                        enabled: true,
                        role: egui::Role::ListItem,
                        label: Some(words.text_with("styles-applied-item", Some(&args))),
                    }
                    .show(ui);
                    let keys = menu_asked(ui.ctx(), &response);
                    let asked = response.clicked() || response.secondary_clicked() || keys;
                    let menu = row_menu_name(name, words);
                    let entries = || {
                        let clear = Item::new(CLEAR, words.text("styles-clear")).enabled(clearable);
                        vec![Entry::Item(clear)]
                    };
                    if menu_of(ui, &response, &menu, (asked, keys), entries).as_deref()
                        == Some(CLEAR)
                    {
                        actions.push(Action::Clear {
                            name: style.name.clone(),
                            file: style.file.clone(),
                        });
                    }
                }
            },
        );
        // Room for the next.
        ui.add_space(t.sizes.panel_line);
    }
}

/// What screen readers call a row's menu.
fn row_menu_name(name: &str, words: &Translator) -> String {
    let mut args = Args::new();
    args.set("name", name);
    words.text_with("styles-row-menu", Some(&args))
}

/// The line an empty folder shows, Custom Styles with no style: "No styles yet.", in
/// `text_secondary`, where a style's name at `level` starts, as [`TreeRow`] sets it in.
fn empty_row(ui: &mut egui::Ui, words: &str, level: usize) {
    let t = theme(ui.ctx());
    let size = vec2(ui.available_width(), t.sizes.panel_line);
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    if ui.is_rect_visible(rect) {
        let font = egui::FontId::proportional(t.text.body);
        let galley = ui
            .painter()
            .layout_no_wrap(words.to_owned(), font, t.colors.text_secondary);
        let indent = (level - 1) as f32 * LEVEL_INDENT + Icon::ChevronEnd.size() + t.spacing.xs;
        let at = egui::pos2(
            rect.left() + indent,
            rect.center().y - galley.size().y / 2.0,
        );
        ui.painter().galley(at, galley, t.colors.text_secondary);
    }
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, words));
}

/// The tool's own items in its ⋯, after Create a Style…: Mode ▸ Append and Replace, the one
/// in use checked, then Import Style… and Export Style…, drawn disabled as later.
pub fn options(words: &Translator, mode: StyleMode) -> Vec<Entry> {
    let later = words.text("styles-later");
    let mode_item = |id: &str, message: &str, of: StyleMode| {
        Entry::Item(Item::new(id, words.text(message)).checked(mode == of))
    };
    vec![
        Entry::Submenu(Submenu::new(
            MODE,
            words.text("styles-mode"),
            vec![
                mode_item(MODE_APPEND, "styles-mode-append", StyleMode::Append),
                mode_item(MODE_REPLACE, "styles-mode-replace", StyleMode::Replace),
            ],
        )),
        Entry::Item(
            Item::new(IMPORT, words.text("styles-import"))
                .enabled(false)
                .tip(later.clone()),
        ),
        Entry::Item(
            Item::new(EXPORT, words.text("styles-export"))
                .enabled(false)
                .tip(later),
        ),
    ]
}

/// The ids of the tool's own ⋯ items.
const MODE: &str = "mode";
const MODE_APPEND: &str = "mode-append";
const MODE_REPLACE: &str = "mode-replace";
const IMPORT: &str = "import-style";
const EXPORT: &str = "export-style";

/// What the tool's header asked for, `pressed`, of its own: Mode's items.
pub fn pressed(pressed: &Pressed) -> Option<Action> {
    match pressed {
        Pressed::Own(id) if id == MODE_APPEND => Some(Action::Mode(StyleMode::Append)),
        Pressed::Own(id) if id == MODE_REPLACE => Some(Action::Mode(StyleMode::Replace)),
        _ => None,
    }
}

/// The title of the tool `id`, as a style's tooltip names it.
pub fn title(words: &Translator, id: &str) -> String {
    words.text(&messages::title(id))
}

#[cfg(test)]
mod tests;
