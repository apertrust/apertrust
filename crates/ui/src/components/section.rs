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
//! reset, presets. A title that would run under them is cut at its end with …, whole in the
//! tooltip and for screen readers. A module has no options button; utilities have options
//! only. A closed section is its header alone, with no summary of what is set. An open
//! section shows its tabs, if it has any, and its body. Holding H over a section, or with
//! its header in focus, shows the photo without it: the section reports that it is peeking,
//! and the app asks the engine. The section never reads the key itself: the app's actions
//! registry says whether it is held ([`Section::peek`]), so the key can be rebound.

use egui::{
    Align, FontFamily, FontId, Layout, Margin, Rect, Role, Sense, Stroke, Ui, UiBuilder,
    WidgetInfo, pos2, vec2,
};

use super::{IconButton, Tabs, focus_ring, icon_colour, icon_lit, truncated};
use crate::i18n::Args;
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
    presets: bool,
    options: bool,
    peek: bool,
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
            presets: true,
            options: true,
            peek: false,
        }
    }

    /// A module: reset and presets, and no options button.
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

    /// Whether a module's reset button can run, as it can by default: drawn disabled
    /// where nothing may change, as an action that cannot run is.
    pub fn reset(mut self, enabled: bool) -> Self {
        self.reset = enabled;
        self
    }

    /// Whether the presets button can run: drawn disabled while nothing is behind it, as
    /// an action that cannot run is.
    pub fn presets(mut self, enabled: bool) -> Self {
        self.presets = enabled;
        self
    }

    /// Whether a utility's options button can run; see [`Self::presets`]. A module has
    /// none, so this changes nothing on one.
    pub fn options(mut self, enabled: bool) -> Self {
        self.options = enabled;
        self
    }

    /// Whether the keys that peek (H by default) are held, as the app's actions registry
    /// reports them; the section peeks while it is under the pointer or its header has
    /// focus.
    pub fn peek(mut self, held: bool) -> Self {
        self.peek = held;
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

        // The buttons, right to left.
        let buttons = ui
            .scope_builder(
                UiBuilder::new()
                    .max_rect(header.shrink2(vec2(t.spacing.s, 0.0)))
                    .layout(Layout::right_to_left(Align::Center)),
                |ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let mut pressed = Vec::new();
                    if self.module {
                        if IconButton::new(Icon::Presets, &say("section-presets"))
                            .enabled(self.presets)
                            .show(ui)
                            .clicked()
                        {
                            pressed.push(SectionAction::Presets);
                        }
                        if IconButton::new(Icon::Reset, &say("section-reset"))
                            .enabled(self.reset)
                            .show(ui)
                            .clicked()
                        {
                            pressed.push(SectionAction::Reset);
                        }
                    } else if IconButton::new(Icon::Options, &say("section-options"))
                        .enabled(self.options)
                        .show(ui)
                        .clicked()
                    {
                        pressed.push(SectionAction::Options);
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

        if self.open {
            let margin = t.spacing.l;
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
