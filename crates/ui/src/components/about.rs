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

//! A section header's tooltip (`docs/gui.md`, The header's tooltip): what the tool is for,
//! in a sentence or two, then, under a hairline, labelled rows with small icons: Kind and
//! Works on (Measures for a utility) in plain words, then darktable's Input, Process and
//! Output word for word, quieter. The section draws it ([`crate::components::Section::about`])
//! over its chevron, title and empty space, never its buttons.
//!
//! It is a tooltip, not a popover: nothing in it is clicked, so it is painted on the
//! tooltip layer and never takes the focus. Its timing is its own, the tooltips' 0.6 s at
//! rest, a sweep down the headers showing each at once, and its place too: beside the side
//! panel over the photo, level with the header, or under it inside the panel where the
//! window leaves too little room.

use egui::text::LayoutJob;
use egui::{
    Color32, FontFamily, FontId, Galley, Id, LayerId, Order, Rect, Stroke, StrokeKind, TextFormat,
    Ui, pos2, vec2,
};
use std::sync::Arc;

use crate::i18n::Args;
use crate::{Icon, Theme, Translator, fonts, popup};

/// What a section's header tooltip says, every word translated.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct About {
    /// What the tool does and when to reach for it, a sentence or two.
    pub description: String,
    /// The Kind row's value: "Fixes a problem and shapes the look", "Edits nothing of its
    /// own"; none for a tool that is not darktable's, whose tooltip is its description.
    pub kind: Option<String>,
    /// What a tool works on, or a utility measures; none for a utility that measures
    /// nothing, as History.
    pub place: Option<Place>,
    /// darktable's input, process and output lines, word for word, for a tool that is
    /// darktable's; none for a utility.
    pub facts: Option<[String; 3]>,
}

/// The second row: what a tool works on, or what a utility measures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Place {
    WorksOn(String),
    Measures(String),
}

impl About {
    /// Its rows, each its icon, its label's message, its value and whether it is one of
    /// darktable's quieter lines.
    fn rows(&self) -> Vec<(Icon, &'static str, &str, bool)> {
        let mut rows = Vec::new();
        if let Some(kind) = &self.kind {
            rows.push((Icon::Kind, "section-about-kind", kind.as_str(), false));
        }
        match &self.place {
            Some(Place::WorksOn(value)) => {
                rows.push((Icon::WorksOn, "section-about-works-on", value, false));
            }
            Some(Place::Measures(value)) => {
                rows.push((Icon::Measures, "section-about-measures", value, false));
            }
            None => {}
        }
        if let Some([input, process, output]) = &self.facts {
            rows.push((Icon::Input, "section-about-input", input, true));
            rows.push((Icon::Process, "section-about-process", process, true));
            rows.push((Icon::Output, "section-about-output", output, true));
        }
        rows
    }

    /// What screen readers hear as the header's description: the tooltip's words without
    /// the name, which the header announces, each row as "label: value".
    pub(super) fn said(&self, tr: &Translator) -> String {
        let mut lines = vec![self.description.clone()];
        for (_, label, value, _) in self.rows() {
            let mut args = Args::new();
            args.set("label", tr.text(label));
            args.set("value", value.to_owned());
            lines.push(tr.text_with("section-about-row", Some(&args)));
        }
        lines.join("\n")
    }
}

/// This tooltip's own delay at rest, in seconds, the wireframe's tooltips' 0.6 s; egui's
/// own tooltips, which the rest of the app draws, keep egui's 0.5 s, left as it is.
const DELAY: f64 = 0.6;
/// This tooltip's own sweep: how soon after one header's tooltip goes the next header's
/// shows at once.
const SWEEP: f64 = 0.4;

/// Every row label a tooltip can have, which the label column is as wide as the widest
/// of, in the language, so every header's tooltip lines its values up alike.
const LABELS: [&str; 6] = [
    "section-about-kind",
    "section-about-works-on",
    "section-about-measures",
    "section-about-input",
    "section-about-process",
    "section-about-output",
];

/// Which header's tooltip waits or shows, for the whole window: one at a time.
#[derive(Clone, Copy, Debug, Default)]
struct Timing {
    /// The pass the claims below are of.
    pass: u64,
    /// The header that asks for its tooltip in that pass, and how: 2 under the pointer, 1
    /// with the keyboard's focus; the pointer's wins. Whether something hid it as it asked,
    /// as a press on the header the pointer has just reached, which carries over to its
    /// becoming the target.
    claim: Option<(u8, Id, bool)>,
    /// The header whose tooltip waits or shows: the claim of the pass before.
    target: Option<Id>,
    /// Whether it is the pointer's, which must rest, rather than the focus's.
    pointer: bool,
    /// How it claimed it: 2 the pointer, 1 the focus, 0 for no target.
    how: u8,
    /// When it became the target.
    since: f64,
    shown: bool,
    /// Hidden by a click, a key, a scroll, a drag, the peek or a menu, until the target
    /// changes.
    dismissed: bool,
    /// When a shown tooltip last went because its header was left.
    left: Option<f64>,
}

/// How the header asks for its tooltip this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Asked {
    No,
    /// Under the pointer.
    Pointer,
    /// With the keyboard's focus; `arrived` in the frame the focus reached it.
    Focus {
        arrived: bool,
    },
}

/// What hid a header's tooltip this frame ([`dismissed`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Dismissed {
    /// A key pressed.
    pub key: bool,
    /// Anything else: a press, a scroll, a drag, a menu open, the peek keys held.
    pub other: bool,
}

/// Whether the header `id` shows its tooltip this frame: after [`DELAY`] at rest, at once
/// within [`SWEEP`] of another's going, never once `dismiss` has held since it asked,
/// but for the key that brought the keyboard's focus to it, Tab or Shift+Tab, which egui
/// may report in the same pass as the focus. Asks for the frame that shows it.
pub(super) fn shows(ui: &Ui, id: Id, asked: Asked, dismiss: Dismissed) -> bool {
    let ctx = ui.ctx();
    let key = Id::unique("apertrust-ui section header tooltip");
    let now = ctx.input(|i| i.time);
    let pass = ctx.cumulative_pass_nr();
    let mut s: Timing = ctx.data(|d| d.get_temp(key)).unwrap_or_default();
    if s.pass != pass {
        // A claim counts only from the pass just before: after a pass that drew no header,
        // the target is given up as if its header had been left.
        let next = s.claim.take().filter(|_| s.pass + 1 == pass);
        let next_id = next.map(|(_, id, _)| id);
        if next_id != s.target {
            if s.target.is_some() {
                s.left = if s.shown && !s.dismissed {
                    Some(now)
                } else if s.dismissed {
                    None
                } else {
                    s.left
                };
            }
            s.target = next_id;
            s.pointer = next.is_some_and(|(how, _, _)| how == 2);
            s.how = next.map_or(0, |(how, _, _)| how);
            s.since = now;
            s.shown = false;
            s.dismissed = next.is_some_and(|(_, _, dismissed)| dismissed);
        }
        s.pass = pass;
    }
    let how = match asked {
        Asked::No => 0,
        Asked::Focus { .. } => 1,
        Asked::Pointer => 2,
    };
    if how > 0 && s.claim.is_none_or(|(best, _, _)| how > best) {
        // A header keeps its tooltip from the key that moved the focus to it, in the frame
        // the focus arrives; any other key hides it, in any other frame, as it does under
        // the pointer.
        let brought = asked == Asked::Focus { arrived: true };
        let hid = dismiss.other || (dismiss.key && !brought);
        s.claim = Some((how, id, hid && s.target != Some(id)));
    }
    if how > s.how && s.target != Some(id) {
        // It takes the target over on the next frame, which may otherwise wait for the
        // pointer to move; a claim weaker than the target's waits for it to end.
        ctx.request_repaint();
    }
    let mut show = false;
    if s.target == Some(id) && how == 0 {
        // Its header no longer asks: the next frame gives the target up, or to another.
        ctx.request_repaint();
    } else if s.target == Some(id) {
        if dismiss.key || dismiss.other {
            s.dismissed = true;
            s.left = None;
        }
        let swept = s.pointer && s.left.is_some_and(|left| s.since - left <= SWEEP);
        let rest = if s.pointer {
            (now - s.since).min(ctx.input(|i| f64::from(i.pointer.time_since_last_movement())))
        } else {
            now - s.since
        };
        let ready = s.shown || swept || rest >= DELAY;
        if !s.dismissed {
            if ready {
                s.shown = true;
                show = true;
            } else {
                ctx.request_repaint_after_secs((DELAY - rest) as f32);
            }
        }
    }
    ctx.data_mut(|d| d.insert_temp(key, s));
    show
}

/// What this frame hides a header's tooltip with: a key, or a press, a scroll, a drag, a
/// menu open, or `peek`, the peek keys held.
pub(super) fn dismissed(ui: &Ui, peek: bool) -> Dismissed {
    let ctx = ui.ctx();
    let (key, input) = ctx.input(|i| {
        let key = i
            .events
            .iter()
            .any(|event| matches!(event, egui::Event::Key { pressed: true, .. }));
        let other = i.pointer.any_pressed()
            || i.smooth_scroll_delta() != egui::Vec2::ZERO
            || i.events
                .iter()
                .any(|event| matches!(event, egui::Event::MouseWheel { .. }));
        (key, other)
    });
    let other =
        input || peek || ctx.dragged_id().is_some() || popup::any_open(ctx) || ctx.any_popup_open();
    Dismissed { key, other }
}

/// One laid-out row.
struct Line {
    icon: Icon,
    label: Arc<Galley>,
    value: Arc<Galley>,
    quiet: bool,
}

/// Paints `about` for the header `header`, with `name` first, bold, where the header cuts
/// it: beside the panel the header spans, on the side with more room, level with the
/// header; where that side has less than `about.room`, under the header inside the panel,
/// or over it near the window's foot. Kept inside the window. Its sizes are the theme's
/// (`Theme::about`, the spacing scale).
pub(super) fn paint(ui: &Ui, id: Id, about: &About, name: Option<&str>, header: Rect) {
    let ctx = ui.ctx();
    let t = crate::theme(ctx);
    let a = &t.about;
    let tr = crate::translator(ctx);
    let screen = ctx.content_rect();
    let edge = t.spacing.m;
    let before = header.left() - screen.left();
    let after = screen.right() - header.right();
    let room = before.max(after);
    let beside = room >= a.room;
    let width = if beside {
        a.width.min(room - 2.0 * edge)
    } else {
        a.narrow_width.min(header.width() - 2.0 * edge)
    };
    let inner = (width - 2.0 * a.padding_x).max(1.0);

    let wrapped = |text: &str, font: FontId, colour: Color32, line: f32, width: f32| {
        let mut job = LayoutJob::default();
        job.append(
            text,
            0.0,
            TextFormat {
                font_id: font,
                color: colour,
                line_height: Some(line),
                ..Default::default()
            },
        );
        job.wrap.max_width = width;
        ctx.fonts_mut(|f| f.layout_job(job))
    };
    // A row's type: the small text, or the caption for darktable's quieter lines.
    let font = |quiet: bool| {
        if quiet {
            (FontId::proportional(t.text.caption), a.quiet_line)
        } else {
            (FontId::proportional(t.text.small), a.line)
        }
    };
    let quiet_label = |label: &str| {
        matches!(
            label,
            "section-about-input" | "section-about-process" | "section-about-output"
        )
    };
    let medium = FontFamily::Name(fonts::MEDIUM.into());
    let heading = name.map(|name| {
        wrapped(
            name,
            FontId::new(t.text.body, medium),
            t.colors.text_strong,
            a.name_line,
            inner,
        )
    });
    let (body, line) = font(false);
    let description = wrapped(&about.description, body, t.colors.text_strong, line, inner);
    let label = |id: &str| {
        let (font, line) = font(quiet_label(id));
        wrapped(
            &tr.text(id),
            font,
            t.colors.text_secondary,
            line,
            f32::INFINITY,
        )
    };
    // As wide as the widest label in the language, so every tooltip lines up alike.
    let gap = t.spacing.m;
    let label_width = LABELS
        .iter()
        .map(|id| label(id).size().x)
        .fold(a.label_column, f32::max)
        .ceil();
    let value_width = (inner - a.icon_column - gap - label_width - gap).max(1.0);
    let lines: Vec<Line> = about
        .rows()
        .into_iter()
        .map(|(icon, id, value, quiet)| {
            let (font, line) = font(quiet);
            let colour = if quiet {
                t.colors.text_secondary
            } else {
                t.colors.text_strong
            };
            Line {
                icon,
                label: label(id),
                value: wrapped(value, font, colour, line, value_width),
                quiet,
            }
        })
        .collect();

    // Heights: the name and `spacing.xxs` under it, the description, then, where it has
    // rows, `spacing.m`, the hairline and the rows.
    let mut height = t.spacing.m + description.size().y + a.padding_bottom;
    if let Some(heading) = &heading {
        height += heading.size().y + t.spacing.xxs;
    }
    if !lines.is_empty() {
        height += t.spacing.m + t.shape.popup_hairline + a.rule_below;
    }
    let mut first_quiet = true;
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            height += a.row_gap;
        }
        if line.quiet && first_quiet {
            height += a.quiet_gap;
            first_quiet = false;
        }
        height += line.label.size().y.max(line.value.size().y);
    }

    let rect = if beside {
        let x = if before >= after {
            header.left() - edge - width
        } else {
            header.right() + edge
        };
        let y = header
            .top()
            .min(screen.bottom() - edge - height)
            .max(screen.top() + edge);
        Rect::from_min_size(pos2(x, y), vec2(width, height))
    } else {
        let below = header.bottom() + t.spacing.xs;
        let y = if below + height > screen.bottom() - edge {
            (header.top() - t.spacing.xs - height).max(screen.top() + edge)
        } else {
            below
        };
        Rect::from_min_size(pos2(header.left() + edge, y), vec2(width, height))
    };
    let rect = Rect::from_min_size(rect.min.round(), rect.size());

    let painter = ctx.layer_painter(LayerId::new(Order::Tooltip, id.with("about")));
    paint_frame(&painter, rect, &t);
    let mut y = rect.top() + t.spacing.m;
    let x = rect.left() + a.padding_x;
    if let Some(heading) = heading {
        let h = heading.size().y;
        painter.galley(pos2(x, y), heading, t.colors.text_strong);
        y += h + t.spacing.xxs;
    }
    let h = description.size().y;
    painter.galley(pos2(x, y), description, t.colors.text_strong);
    if lines.is_empty() {
        return;
    }
    y += h + t.spacing.m;
    painter.hline(
        x..=rect.right() - a.padding_x,
        y + t.shape.popup_hairline / 2.0,
        Stroke::new(t.shape.popup_hairline, t.colors.divider),
    );
    y += t.shape.popup_hairline + a.rule_below;
    let mut first_quiet = true;
    for (i, line) in lines.into_iter().enumerate() {
        if i > 0 {
            y += a.row_gap;
        }
        if line.quiet && first_quiet {
            y += a.quiet_gap;
            first_quiet = false;
        }
        let row_height = line.label.size().y.max(line.value.size().y);
        // The icon centred on the row's first line.
        let first = if line.quiet { a.quiet_line } else { a.line };
        let side = line.icon.size();
        line.icon.paint(
            &painter,
            Rect::from_center_size(
                pos2(x + a.icon_column / 2.0, y + first / 2.0),
                vec2(side, side),
            ),
            t.colors.text_secondary,
        );
        let labels_at = x + a.icon_column + gap;
        painter.galley(pos2(labels_at, y), line.label, t.colors.text_secondary);
        let colour = if line.quiet {
            t.colors.text_secondary
        } else {
            t.colors.text_strong
        };
        painter.galley(pos2(labels_at + label_width + gap, y), line.value, colour);
        y += row_height;
    }
}

/// The tooltip's ground: `raised`, the menus' edge and shadow, the popups' radius.
fn paint_frame(painter: &egui::Painter, rect: Rect, t: &Theme) {
    let shadow = egui::Shadow {
        offset: t.elevation.shadow_offset,
        blur: t.elevation.shadow_blur,
        spread: t.elevation.shadow_spread,
        color: t.colors.menu_shadow,
    };
    painter.add(shadow.as_shape(rect, t.shape.popup_radius));
    painter.rect(
        rect,
        t.shape.popup_radius,
        t.colors.raised,
        Stroke::new(t.shape.popup_hairline, t.colors.menu_edge),
        StrokeKind::Inside,
    );
}

#[cfg(test)]
mod tests {
    use egui::{Context, Id, RawInput};

    use super::{Asked, Dismissed, shows};

    /// Runs one frame at `time`, the header `id` asking as `asked`, with a press hiding it
    /// where `pressed`, and gives whether its tooltip shows.
    fn frame(ctx: &Context, time: f64, id: Id, asked: Asked, pressed: bool) -> bool {
        let dismiss = Dismissed {
            key: false,
            other: pressed,
        };
        let mut shown = false;
        let input = RawInput {
            time: Some(time),
            ..RawInput::default()
        };
        let mut output = ctx.run_ui(input, |ui| shown = shows(ui, id, asked, dismiss));
        // The font atlas's first upload, which no renderer takes here.
        output.textures_delta.clear();
        shown
    }

    /// Runs one frame at `time` that draws no header.
    fn idle(ctx: &Context, time: f64) {
        let input = RawInput {
            time: Some(time),
            ..RawInput::default()
        };
        let mut output = ctx.run_ui(input, |_| {});
        output.textures_delta.clear();
    }

    /// A header hidden by a press, drawn again after a pass that drew no header (another
    /// tab, say) with the pointer still over it, is a new target: it shows after the delay.
    #[test]
    fn a_pass_with_no_header_gives_the_target_up() {
        let ctx = Context::default();
        let id = Id::unique("header");
        // It asks, becomes the target, and shows after the delay.
        assert!(!frame(&ctx, 0.0, id, Asked::Pointer, false));
        assert!(!frame(&ctx, 0.1, id, Asked::Pointer, false));
        assert!(
            frame(&ctx, 1.2, id, Asked::Pointer, false),
            "not shown at first"
        );
        // A press hides it.
        assert!(!frame(&ctx, 1.3, id, Asked::Pointer, true));
        assert!(!frame(&ctx, 1.4, id, Asked::Pointer, false));
        // A pass with no header, then the header again under the pointer.
        idle(&ctx, 1.5);
        assert!(!frame(&ctx, 1.6, id, Asked::Pointer, false));
        assert!(!frame(&ctx, 1.7, id, Asked::Pointer, false));
        assert!(
            frame(&ctx, 2.8, id, Asked::Pointer, false),
            "kept hidden by a stale claim"
        );
    }

    /// A press on the frame the pointer reaches a header, before it is the target, hides
    /// its tooltip as one after would, until the pointer leaves.
    #[test]
    fn a_press_as_the_pointer_arrives_hides_it_until_it_leaves() {
        let ctx = Context::default();
        let id = Id::unique("header");
        assert!(!frame(&ctx, 0.0, id, Asked::Pointer, true));
        for time in [0.25, 0.5, 1.0, 2.0] {
            assert!(
                !frame(&ctx, time, id, Asked::Pointer, false),
                "shown at {time}"
            );
        }
        // Left, then back: it shows after the delay.
        assert!(!frame(&ctx, 2.1, id, Asked::No, false));
        assert!(!frame(&ctx, 2.2, id, Asked::No, false));
        assert!(!frame(&ctx, 2.3, id, Asked::Pointer, false));
        assert!(!frame(&ctx, 2.4, id, Asked::Pointer, false));
        assert!(
            frame(&ctx, 3.5, id, Asked::Pointer, false),
            "not back after leaving"
        );
    }
}
