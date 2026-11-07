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

//! The popup service's native side on the Mac (decision 40; `docs/gui.md`, Popups): every
//! plain list a component asks for, a dropdown or a pull-down, is AppKit's menu, through
//! `apertrust-display`. Elsewhere nothing is installed and the design system draws them.
//!
//! The service, [`Native`], takes a request inside the frame and only keeps it: the frame
//! has the window, which [`show`] is given once the frame ends, and AppKit's menu runs
//! later still, from the run loop outside winit's handler (`apertrust_display::pop_up_menu`).
//! Its answer comes back through `popup::answered` once the menu has closed, so a component
//! reads it with `popup::answer` on the next frame, as it reads a drawn popup's. The window
//! keeps the keyboard focus while the menu is open, and the service gives the focus back to
//! the control that opened it, chosen or not.

use std::sync::{Arc, Mutex, PoisonError};

use apertrust_display::{Appearance, KeyEquivalent, MenuEntry, MenuItem, PopUpMenu};
use apertrust_ui::popup::{self, Answer, Entry, Kind, Request, Service};
use eframe::egui::{self, Id};
use egui_winit::winit::raw_window_handle::HasWindowHandle;

/// The service the app installs on the Mac: it takes every plain list and holds it for
/// [`show`].
#[derive(Clone, Default)]
struct Native {
    asked: Arc<Mutex<Vec<(Id, Request)>>>,
}

impl Native {
    fn asked(&self) -> std::sync::MutexGuard<'_, Vec<(Id, Request)>> {
        self.asked.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Service for Native {
    /// Both kinds today are plain lists; a kind that holds controls, as the slider popup
    /// will, stays drawn.
    fn takes(&self, request: &Request) -> bool {
        matches!(request.kind, Kind::PullDown | Kind::Dropdown)
    }

    fn open(&self, id: Id, request: Request) {
        let mut asked = self.asked();
        asked.retain(|(open, _)| *open != id);
        asked.push((id, request));
    }

    fn close(&self, id: Id) {
        self.asked().retain(|(open, _)| *open != id);
    }
}

/// Where `ctx` keeps its service, for [`show`].
fn key() -> Id {
    Id::unique("apertrust native popups")
}

/// Installs the native service in `ctx` on macOS, for the window `ctx` draws; elsewhere
/// does nothing, and every popup stays drawn.
pub fn install(ctx: &egui::Context) {
    if !cfg!(target_os = "macos") {
        return;
    }
    let native = Native::default();
    ctx.data_mut(|d| d.insert_temp(key(), native.clone()));
    popup::install(ctx, Arc::new(native));
}

/// After the frame: hands the menus asked for during it to AppKit, shown in `window`, the
/// one `ctx` draws. Each answers on a later frame; one that cannot be shown is dismissed.
pub fn show(ctx: &egui::Context, window: &impl HasWindowHandle) {
    let Some(native) = ctx.data(|d| d.get_temp::<Native>(key())) else {
        return;
    };
    let asked = std::mem::take(&mut *native.asked());
    if asked.is_empty() {
        return;
    }
    let look = Look::of(ctx);
    for (id, request) in asked {
        let answering = ctx.clone();
        let shown = apertrust_display::pop_up_menu(window, menu(&request, &look), move |chosen| {
            let answer = chosen.map_or(Answer::Dismissed, Answer::Chosen);
            popup::answered(&answering, id, answer);
        });
        match shown {
            Ok(true) => {}
            Ok(false) => popup::answered(ctx, id, Answer::Dismissed),
            Err(e) => {
                tracing::warn!(target: "gpu", "popup menu not shown: {e}");
                popup::answered(ctx, id, Answer::Dismissed);
            }
        }
    }
}

/// What a menu takes from the interface.
struct Look {
    /// egui's points to the window's: the interface's scale.
    scale: f32,
    appearance: Appearance,
    /// Space as the keymap spells it, translated.
    space: String,
}

impl Look {
    /// Light for the light theme, dark for the dark and high-contrast ones (`docs/gui.md`,
    /// Popups: Appearance).
    fn of(ctx: &egui::Context) -> Look {
        Look {
            scale: ctx.zoom_factor(),
            appearance: if apertrust_ui::theme(ctx).dark {
                Appearance::Dark
            } else {
                Appearance::Light
            },
            space: apertrust_ui::translator(ctx).text("key-space"),
        }
    }
}

/// `request` as AppKit's menu: its anchor in the window's points, a dropdown's checked
/// option over the control, a pull-down under it.
fn menu(request: &Request, look: &Look) -> PopUpMenu {
    let anchor = request.anchor;
    let scale = f64::from(look.scale);
    let points = |v: f32| f64::from(v) * scale;
    PopUpMenu {
        entries: entries(&request.entries, true, &look.space),
        anchor: [
            points(anchor.left()),
            points(anchor.top()),
            points(anchor.width()),
            points(anchor.height()),
        ],
        over: match request.kind {
            Kind::Dropdown => request.checked(),
            Kind::PullDown => None,
        },
        appearance: look.appearance,
        scale,
    }
}

/// The lines of a level, at the `top` or not: a submenu's `&` marks an access key only at
/// the top, where AppKit has none, so it is taken out there.
fn entries(lines: &[Entry], top: bool, space: &str) -> Vec<MenuEntry> {
    lines
        .iter()
        .map(|entry| match entry {
            Entry::Item(item) => MenuEntry::Item(MenuItem {
                id: item.id.clone(),
                title: item.text.clone(),
                enabled: item.enabled,
                on: item.checked == Some(true),
                tip: item.tip.clone(),
                keys: item
                    .shortcut
                    .as_deref()
                    .and_then(|keys| key_equivalent(keys, space)),
            }),
            Entry::Separator => MenuEntry::Separator,
            Entry::Submenu(submenu) => MenuEntry::Submenu {
                title: if top {
                    submenu.shown()
                } else {
                    submenu.text.clone()
                },
                entries: entries(&submenu.entries, false, space),
            },
        })
        .collect()
}

/// Keys as the Mac's keymap spells them (`Shift+Cmd+Z`, `actions::Shortcut::spell`) as
/// AppKit's key equivalent, which it shows in its glyphs; `space` is Space's translated
/// name. None for keys it cannot show, which the item then shows none for.
fn key_equivalent(spelt: &str, space: &str) -> Option<KeyEquivalent> {
    // The key follows the last +, or is + itself after another (`Cmd++`).
    let (names, key) = match spelt.strip_suffix('+') {
        Some("") => ("", "+"),
        Some(names) if names.ends_with('+') => (&names[..names.len() - 1], "+"),
        _ => spelt.rsplit_once('+').unwrap_or(("", spelt)),
    };
    let mut keys = KeyEquivalent {
        key: String::new(),
        command: false,
        shift: false,
        option: false,
        control: false,
    };
    for name in names.split('+').filter(|name| !name.is_empty()) {
        match name {
            "Cmd" => keys.command = true,
            "Shift" => keys.shift = true,
            "Option" => keys.option = true,
            "Ctrl" => keys.control = true,
            _ => return None,
        }
    }
    keys.key = match key {
        "Return" => KeyEquivalent::RETURN.to_owned(),
        "Tab" => KeyEquivalent::TAB.to_owned(),
        "Escape" => KeyEquivalent::ESCAPE.to_owned(),
        "Delete" => KeyEquivalent::BACKSPACE.to_owned(),
        "Forward Delete" => KeyEquivalent::DELETE.to_owned(),
        "↑" => KeyEquivalent::UP.to_owned(),
        "↓" => KeyEquivalent::DOWN.to_owned(),
        "←" => KeyEquivalent::LEFT.to_owned(),
        "→" => KeyEquivalent::RIGHT.to_owned(),
        "Home" => KeyEquivalent::HOME.to_owned(),
        "End" => KeyEquivalent::END.to_owned(),
        "PageUp" => KeyEquivalent::PAGE_UP.to_owned(),
        "PageDown" => KeyEquivalent::PAGE_DOWN.to_owned(),
        // egui spells Minus with the minus sign.
        "\u{2212}" => "-".to_owned(),
        _ if key == space => " ".to_owned(),
        _ => {
            let function = key
                .strip_prefix('F')
                .filter(|n| !n.is_empty())
                .and_then(|n| n.parse().ok())
                .and_then(KeyEquivalent::function);
            let mut chars = key.chars();
            match (function, chars.next(), chars.next()) {
                (Some(function), ..) => function,
                (None, Some(c), None) => c.to_lowercase().collect(),
                _ => return None,
            }
        }
    };
    Some(keys)
}

#[cfg(test)]
mod tests {
    use super::*;
    use apertrust_ui::popup::{Item, Submenu};
    use eframe::egui::{Rect, pos2, vec2};

    fn look(scale: f32) -> Look {
        Look {
            scale,
            appearance: Appearance::Dark,
            space: "Space".to_owned(),
        }
    }

    fn keys(key: &str, command: bool, shift: bool, option: bool, control: bool) -> KeyEquivalent {
        KeyEquivalent {
            key: key.to_owned(),
            command,
            shift,
            option,
            control,
        }
    }

    #[test]
    fn a_dropdown_puts_its_checked_option_over_the_control() {
        let anchor = Rect::from_min_size(pos2(100.0, 40.0), vec2(80.0, 18.0));
        let request = Request::dropdown(
            anchor,
            "Illuminant",
            vec![
                Item::new("0", "Daylight").checked(false),
                Item::new("1", "Black body").checked(true),
                Item::new("2", "Fluorescent")
                    .checked(false)
                    .enabled(false)
                    .tip("Coming in a later version"),
            ],
        );
        let menu = menu(&request, &look(1.5));
        assert_eq!(menu.over, Some(1));
        // In the window's points: egui's times the interface's scale.
        assert_eq!(menu.anchor, [150.0, 60.0, 120.0, 27.0]);
        assert_eq!(menu.scale, 1.5);
        assert_eq!(
            menu.entries,
            vec![
                MenuEntry::Item(MenuItem {
                    id: "0".to_owned(),
                    title: "Daylight".to_owned(),
                    enabled: true,
                    on: false,
                    tip: None,
                    keys: None,
                }),
                MenuEntry::Item(MenuItem {
                    id: "1".to_owned(),
                    title: "Black body".to_owned(),
                    enabled: true,
                    on: true,
                    tip: None,
                    keys: None,
                }),
                MenuEntry::Item(MenuItem {
                    id: "2".to_owned(),
                    title: "Fluorescent".to_owned(),
                    enabled: false,
                    on: false,
                    tip: Some("Coming in a later version".to_owned()),
                    keys: None,
                }),
            ]
        );
        // Custom, a preset row's text that is no option: nothing over the control, the
        // list under it.
        let custom = Request::dropdown(anchor, "Settings", vec![Item::new("0", "As shot")]);
        assert_eq!(super::menu(&custom, &look(1.0)).over, None);
    }

    #[test]
    fn a_pull_down_keeps_its_lines_separators_and_submenus_and_opens_under_its_button() {
        let request = Request::pull_down(
            Rect::from_min_size(pos2(10.0, 0.0), vec2(24.0, 24.0)),
            "Menu",
            vec![
                Submenu::new(
                    "file",
                    "&File",
                    vec![
                        Item::new("open", "Open Image…").shortcut("Cmd+O").into(),
                        Entry::Separator,
                        Submenu::new("recent", "Open &Recent", vec![]).into(),
                    ],
                )
                .into(),
                Item::new("names", "Show tab names").checked(true).into(),
            ],
        );
        let menu = menu(&request, &look(1.0));
        assert_eq!(menu.over, None);
        assert_eq!(
            menu.entries,
            vec![
                MenuEntry::Submenu {
                    // The access key's & taken out at the top, kept below it.
                    title: "File".to_owned(),
                    entries: vec![
                        MenuEntry::Item(MenuItem {
                            id: "open".to_owned(),
                            title: "Open Image…".to_owned(),
                            enabled: true,
                            on: false,
                            tip: None,
                            keys: Some(keys("o", true, false, false, false)),
                        }),
                        MenuEntry::Separator,
                        MenuEntry::Submenu {
                            title: "Open &Recent".to_owned(),
                            entries: vec![],
                        },
                    ],
                },
                MenuEntry::Item(MenuItem {
                    id: "names".to_owned(),
                    title: "Show tab names".to_owned(),
                    enabled: true,
                    on: true,
                    tip: None,
                    keys: None,
                }),
            ]
        );
    }

    #[test]
    fn the_mac_s_spelling_of_keys_is_appkit_s_key_equivalent() {
        let space = "Пробіл";
        assert_eq!(
            key_equivalent("Shift+Cmd+Z", space),
            Some(keys("z", true, true, false, false))
        );
        assert_eq!(
            key_equivalent("Ctrl+Option+E", space),
            Some(keys("e", false, false, true, true))
        );
        assert_eq!(
            key_equivalent("Cmd+,", space),
            Some(keys(",", true, false, false, false))
        );
        assert_eq!(
            key_equivalent("Cmd++", space),
            Some(keys("+", true, false, false, false))
        );
        assert_eq!(
            key_equivalent("+", space),
            Some(keys("+", false, false, false, false))
        );
        assert_eq!(
            key_equivalent("Cmd+\u{2212}", space),
            Some(keys("-", true, false, false, false))
        );
        assert_eq!(
            key_equivalent("G", space),
            Some(keys("g", false, false, false, false))
        );
        assert_eq!(
            key_equivalent("Cmd+2", space),
            Some(keys("2", true, false, false, false))
        );
        assert_eq!(
            key_equivalent("Пробіл", space),
            Some(keys(" ", false, false, false, false))
        );
        assert_eq!(
            key_equivalent("Delete", space),
            Some(keys("\u{8}", false, false, false, false))
        );
        assert_eq!(
            key_equivalent("Cmd+Return", space),
            Some(keys("\r", true, false, false, false))
        );
        assert_eq!(
            key_equivalent("Shift+→", space),
            Some(keys("\u{f703}", false, true, false, false))
        );
        assert_eq!(
            key_equivalent("F6", space),
            Some(keys("\u{f709}", false, false, false, false))
        );
        // What AppKit could not be given shows no keys.
        assert_eq!(key_equivalent("Insert", space), None);
        assert_eq!(key_equivalent("Hyper+K", space), None);
    }

    #[test]
    fn the_service_takes_plain_lists_and_holds_them_until_the_frame_ends() {
        let native = Native::default();
        let request = Request::pull_down(Rect::NOTHING, "More", vec![]);
        assert!(native.takes(&request));
        assert!(native.takes(&Request::dropdown(Rect::NOTHING, "Mode", vec![])));
        let (a, b) = (Id::unique("a"), Id::unique("b"));
        native.open(a, request.clone());
        native.open(b, request.clone());
        // Opened again in place of itself; closed before the frame ended, never shown.
        native.open(a, request.clone());
        native.close(b);
        assert_eq!(*native.asked(), vec![(a, request)]);
    }
}
