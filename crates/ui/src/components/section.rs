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

//! A section of the side panel: a module or a utility such as Scopes or History.
//!
//! The header holds the chevron and the title, then the buttons, in the wireframe's order:
//! a module's reset, presets and options, a utility's options alone, or with a reset
//! before them where it has one, as History's resets the edit, each as narrow as its
//! icon and evenly spaced, the last one's ink ending where the rows do. A module's options
//! are drawn disabled until what their menu holds is decided. A title that would run under
//! the buttons is cut at its end with …, whole in the tooltip and for screen readers. A
//! closed section is its header alone, with no summary of what is set. An open section
//! shows its tabs, if it has any, and its body. Holding H over a section, or with its
//! header in focus, shows the photo without it: the section reports that it is peeking, and
//! the app asks the engine. The section never reads the key itself: the app's actions
//! registry says whether it is held ([`Section::peek`]), so the key can be rebound.

use egui::{
    Align, FontFamily, FontId, Layout, Margin, Rect, Role, Sense, Stroke, Ui, UiBuilder,
    WidgetInfo, pos2, vec2,
};

use super::{IconButton, Tabs, focus_ring, icon_colour, icon_lit, pull_down, truncated};
use crate::i18n::Args;
use crate::icons::STROKE;
use crate::popup::Entry;
use crate::{Icon, fonts, theme, translator};

/// What the user did to a section's frame this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SectionAction {
    /// Open it, or close it.
    Toggle,
    Reset,
    Presets,
    Options,
    /// Another of the section's tabs.
    Tab(usize),
}

/// What the section reports: the user's actions, and whether the photo should show
/// without it while H is held.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SectionOutput {
    pub actions: Vec<SectionAction>,
    pub peeking: bool,
    /// The id of the item chosen this frame in the menu its options button opens, given
    /// by [`Section::options_menu`].
    pub chosen: Option<String>,
}

pub struct Section<'a> {
    title: &'a str,
    open: bool,
    tabs: &'a [&'a str],
    tab: usize,
    /// Tabs that cannot be chosen yet, by index, and their tooltip.
    disabled_tabs: (&'a [usize], &'a str),
    module: bool,
    reset: bool,
    /// A utility's reset button, by its name, where it has one.
    utility_reset: Option<&'a str>,
    presets: bool,
    options: bool,
    /// What the options button opens, a pull-down of the popup service's, where the
    /// section gives one.
    menu: Option<Vec<Entry>>,
    peek: bool,
    not_applied: bool,
}

impl Section<'_> {
    /// The body's margin at its start and end, 12 points (`spacing.l`): a module's rows
    /// start inside it, and a [`section_list`]'s scroll bar floats in its end.
    pub fn body_margin(t: &crate::Theme) -> f32 {
        t.spacing.l
    }
}

impl<'a> Section<'a> {
    /// A utility section: options only. `title` is translated.
    pub fn new(title: &'a str, open: bool) -> Self {
        Section {
            title,
            open,
            tabs: &[],
            tab: 0,
            disabled_tabs: (&[], ""),
            module: false,
            reset: true,
            utility_reset: None,
            presets: true,
            options: true,
            menu: None,
            peek: false,
            not_applied: false,
        }
    }

    /// A module: reset, presets and options, its options drawn disabled for now ("No
    /// options yet", `section-options-none`).
    pub fn module(mut self) -> Self {
        self.module = true;
        self
    }

    pub fn tabs(mut self, tabs: &'a [&'a str], selected: usize) -> Self {
        self.tabs = tabs;
        self.tab = selected;
        self
    }

    /// Tabs with nothing behind them yet, by index: drawn disabled, with `tip`,
    /// translated, as their tooltip ([`Tabs::disabled`]).
    pub fn disabled_tabs(mut self, tabs: &'a [usize], tip: &'a str) -> Self {
        self.disabled_tabs = (tabs, tip);
        self
    }

    /// Whether a module's reset button, or a utility's ([`Self::utility_reset`]), can run,
    /// as it can by default: drawn disabled where nothing may change, as an action that
    /// cannot run is.
    pub fn reset(mut self, enabled: bool) -> Self {
        self.reset = enabled;
        self
    }

    /// A utility with a reset button before its options, named `name`, translated, as
    /// History's header has one, which resets the edit (`docs/history.md`). A module's
    /// reset is its own, so this changes nothing on one.
    pub fn utility_reset(mut self, name: &'a str) -> Self {
        self.utility_reset = Some(name);
        self
    }

    /// Whether the presets button can run: drawn disabled while nothing is behind it, as
    /// an action that cannot run is.
    pub fn presets(mut self, enabled: bool) -> Self {
        self.presets = enabled;
        self
    }

    /// Whether a utility's options button can run; see [`Self::presets`]. A module's is
    /// drawn disabled whatever this says, so this changes nothing on one.
    pub fn options(mut self, enabled: bool) -> Self {
        self.options = enabled;
        self
    }

    /// A utility's options as a menu its options button opens, `entries` translated: the
    /// item chosen comes back as [`SectionOutput::chosen`], in place of
    /// [`SectionAction::Options`].
    pub fn options_menu(mut self, entries: Vec<Entry>) -> Self {
        self.menu = Some(entries);
        self
    }

    /// Whether the keys that peek (H by default) are held, as the app's actions registry
    /// reports them; the section peeks while it is under the pointer or its header has
    /// focus.
    pub fn peek(mut self, held: bool) -> Self {
        self.peek = held;
        self
    }

    /// Whether the module is off and renders nothing, as a switched module is until a
    /// control is written (decision 32): nothing more is drawn, and screen readers hear
    /// "Not applied" (`section-not-applied`) first in the toggle's description.
    pub fn not_applied(mut self, off: bool) -> Self {
        self.not_applied = off;
        self
    }

    /// Draws the section; `body` draws what is inside it while it is open.
    pub fn show(self, ui: &mut Ui, body: impl FnOnce(&mut Ui)) -> SectionOutput {
        let t = theme(ui.ctx());
        let tr = translator(ui.ctx());
        let mut args = Args::new();
        args.set("section", self.title);
        let say = |id: &str| tr.text_with(id, Some(&args));
        let id = ui.make_persistent_id(("section", self.title));
        let mut output = SectionOutput::default();
        let top = ui.cursor().top();
        let width = ui.available_width();

        let (header, _) =
            ui.allocate_exact_size(vec2(width, t.sizes.section_header), Sense::hover());
        let menu = self.menu;
        let mut chosen = None;

        // The buttons, right to left, each `sizes.section_button` wide round its icon and
        // a control high, `spacing.section_button_gap` apart. The last one's ink, not its
        // box, ends where the rows do (`body_margin`, 12 points): it is ⋯ on every header,
        // Lucide's ellipsis, whose last dot ends 4 of its 24 units inside the icon's box,
        // less half the line drawn round it (`icons::STROKE`, 1.25 points), so 1.7 points
        // inside at 14 and the header's end inset 10.3.
        let icon = Icon::Options.size();
        let ink = (t.sizes.section_button - icon) / 2.0 + 4.0 / 24.0 * icon - STROKE / 2.0;
        let end = Section::body_margin(&t) - ink;
        let narrow = t.sizes.section_button;
        let buttons = ui
            .scope_builder(
                UiBuilder::new()
                    .max_rect(Rect::from_min_max(
                        pos2(header.left() + t.spacing.s, header.top()),
                        pos2(header.right() - end, header.bottom()),
                    ))
                    .layout(Layout::right_to_left(Align::Center)),
                |ui| {
                    ui.spacing_mut().item_spacing.x = t.spacing.section_button_gap;
                    let mut pressed = Vec::new();
                    let name = say("section-options");
                    if self.module {
                        // Drawn disabled until what its menu holds is decided.
                        let none = tr.text("section-options-none");
                        IconButton::new(Icon::Options, &name)
                            .width(narrow)
                            .tip(&none)
                            .enabled(false)
                            .show(ui);
                        if IconButton::new(Icon::Presets, &say("section-presets"))
                            .width(narrow)
                            .enabled(self.presets)
                            .show(ui)
                            .clicked()
                        {
                            pressed.push(SectionAction::Presets);
                        }
                        if IconButton::new(Icon::Reset, &say("section-reset"))
                            .width(narrow)
                            .enabled(self.reset)
                            .show(ui)
                            .clicked()
                        {
                            pressed.push(SectionAction::Reset);
                        }
                    } else {
                        let options = IconButton::new(Icon::Options, &name)
                            .width(narrow)
                            .enabled(self.options)
                            .show(ui);
                        match menu {
                            Some(entries) => chosen = pull_down(ui, &options, &name, || entries),
                            None if options.clicked() => pressed.push(SectionAction::Options),
                            None => {}
                        }
                        if let Some(reset) = self.utility_reset
                            && IconButton::new(Icon::Reset, reset)
                                .width(narrow)
                                .enabled(self.reset)
                                .show(ui)
                                .clicked()
                        {
                            pressed.push(SectionAction::Reset);
                        }
                    }
                    (pressed, ui.min_rect().left())
                },
            )
            .inner;
        let (mut pressed, buttons_left) = buttons;

        // The toggle: chevron and title, the header's whole height for the pointer.
        let toggle_rect = Rect::from_min_max(
            pos2(header.left() + t.spacing.xxs, header.top()),
            pos2(buttons_left - t.spacing.xxs, header.bottom()),
        );
        let toggle = ui.interact(toggle_rect, id, Sense::click());
        if toggle.clicked() {
            output.actions.push(SectionAction::Toggle);
        }
        toggle.widget_info(|| WidgetInfo::labeled(Role::Button, true, self.title));
        ui.ctx()
            .accesskit_node_builder(id, |node| node.set_expanded(self.open));

        let painter = ui.painter();
        let mut x = toggle_rect.left() + t.spacing.s;
        let chevron = if self.open {
            Icon::ChevronDown
        } else {
            Icon::Chevron
        };
        chevron.paint(
            painter,
            Rect::from_center_size(pos2(x + 6.0, header.center().y), vec2(12.0, 12.0)),
            // `text` while open or lit, `text_secondary` closed.
            icon_colour(&t, true, self.open || icon_lit(painter.ctx(), &toggle)),
        );
        x += 12.0 + t.spacing.s;
        // A title too long for the toggle is cut at its end, before the buttons; the
        // tooltip and screen readers have it whole.
        let font = FontId::new(t.text.section_title, FontFamily::Name(fonts::MEDIUM.into()));
        let title = painter.layout_job(truncated(
            self.title,
            font,
            (toggle_rect.right() - x).max(0.0),
            t.colors.text,
        ));
        let cut = title.elided;
        painter.galley(
            pos2(x, header.center().y - title.size().y / 2.0),
            title,
            t.colors.text,
        );
        if toggle.has_focus() {
            focus_ring(painter, toggle_rect, &t);
        }

        let focused = toggle.has_focus();
        let tip = say("section-toggle");
        let toggle = toggle.on_hover_text(if cut {
            format!("{}\n{tip}", self.title)
        } else {
            tip
        });
        drop(toggle);
        if self.not_applied {
            let not_applied = tr.text("section-not-applied");
            ui.ctx().accesskit_node_builder(id, |node| {
                let said = match node.description() {
                    Some(tip) => format!("{not_applied}\n{tip}"),
                    None => not_applied,
                };
                node.set_description(said);
            });
        }

        if self.open {
            let margin = Section::body_margin(&t);
            if !self.tabs.is_empty() {
                // The tabs start on the header's bottom edge, whatever the item spacing,
                // and hang out by their padding so the first name sits on the content's
                // edge.
                let hang = t.spacing.m;
                let chosen = ui
                    .scope_builder(
                        UiBuilder::new().max_rect(Rect::from_min_max(
                            pos2(header.left() + margin - hang, header.bottom()),
                            pos2(header.right() - margin, header.bottom() + 40.0),
                        )),
                        |ui| {
                            let (disabled, tip) = self.disabled_tabs;
                            Tabs::new(&say("section-tabs"), self.tabs, self.tab)
                                .disabled(disabled, tip)
                                .hang(hang)
                                .show(ui)
                        },
                    )
                    .inner;
                ui.add_space(t.spacing.m);
                if let Some(i) = chosen {
                    pressed.push(SectionAction::Tab(i));
                }
            }
            egui::Frame::new()
                .inner_margin(Margin {
                    left: margin as i8,
                    right: margin as i8,
                    // Without tabs the body starts 4 points under the header, outside
                    // its band.
                    top: if self.tabs.is_empty() {
                        t.spacing.xs as i8
                    } else {
                        0
                    },
                    bottom: t.spacing.m as i8,
                })
                .show(ui, |ui| {
                    ui.set_width(width - 2.0 * margin);
                    body(ui);
                });
        }

        let bottom = ui.cursor().top();
        ui.painter().hline(
            header.x_range(),
            bottom - t.shape.hairline / 2.0,
            Stroke::new(t.shape.hairline, t.colors.separator),
        );

        // Holding H shows the photo without the section under the pointer, its header and
        // body only; the registry leaves typing alone.
        let hovered = ui.rect_contains_pointer(Rect::from_min_max(
            pos2(header.left(), top),
            pos2(header.right(), bottom),
        ));
        output.chosen = chosen;
        output.peeking = self.peek && (hovered || focused);
        if output.peeking {
            ui.painter().rect_filled(
                header.shrink2(vec2(0.0, 2.0)),
                t.shape.radius,
                t.colors.accent_subtle.gamma_multiply(0.6),
            );
        }
        output.actions.append(&mut pressed);
        output
    }
}

/// A list scrolling inside a section's body: every scrolling area inside a module is one,
/// so they all look and behave alike. It is `sizes.section_list` high whatever `add` draws,
/// so the section keeps its height, and scrolls when the content is taller. Its scroll bar
/// is egui's floating one in the quiet bar's colours ([`super::quiet_scroll`]), drawn in
/// the body's end margin ([`Section::body_margin`]), clear of the rows: hidden until the
/// pointer is over the list and the content is taller than it, then a thin thumb on its
/// `sunken` track, as wide as the side panel's under the pointer and while dragged. Call
/// it inside a section's body; `id_salt` tells it from another list there.
pub fn section_list<R>(
    ui: &mut Ui,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    add: impl FnOnce(&mut Ui) -> R,
) -> R {
    let t = theme(ui.ctx());
    let margin = Section::body_margin(&t);
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, t.sizes.section_list), Sense::hover());
    let mut list = ui.new_child(
        UiBuilder::new()
            .max_rect(rect.with_max_x(rect.right() + margin))
            .layout(*ui.layout()),
    );
    list.spacing_mut().scroll = egui::style::ScrollStyle {
        bar_width: ui.spacing().scroll.bar_width,
        // The quiet thumb's colours, not the text's.
        foreground_color: false,
        ..egui::style::ScrollStyle::floating()
    };
    let area = egui::ScrollArea::vertical()
        .id_salt(id_salt)
        .auto_shrink([false, false])
        .content_margin(Margin {
            right: margin as i8,
            ..Margin::ZERO
        });
    super::quiet_scroll(&mut list, area, add)
}
