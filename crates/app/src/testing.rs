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

//! What the app's tests share across its tools: a log of a test's own, and a utility's
//! section drawn and photographed.

use std::sync::{Mutex, MutexGuard, PoisonError};

use apertrust_base::log::{Domain, Kept};
use apertrust_ui::{Builtin, Locale, Theme};
use eframe::egui;
use egui_kittest::Harness;
use tracing::Dispatch;

/// A harness `width` points wide and `height` high, in `which`'s theme and British
/// English, drawing `draw` with `state` each frame, as a utility's tests draw its section.
pub fn drawn<S>(
    which: Builtin,
    (width, height): (f32, f32),
    state: S,
    draw: impl FnMut(&mut egui::Ui, &mut S) + 'static,
) -> Harness<'static, S> {
    Harness::builder()
        .with_size(egui::vec2(width, height))
        .with_context_setup(move |ctx| {
            apertrust_ui::install(ctx, Theme::builtin(which));
            apertrust_ui::set_locale(ctx, Locale::EnGb);
        })
        .build_ui_state(draw, state)
}

/// Photographs what `make` draws in the dark and light themes, `name-dark` and
/// `name-light`, once run and left as `prepare` leaves it.
pub fn photograph<S>(
    name: &str,
    make: impl Fn(Builtin) -> Harness<'static, S>,
    prepare: impl Fn(&mut Harness<'static, S>),
) {
    let mut results = egui_kittest::SnapshotResults::new();
    for (which, theme) in [(Builtin::Dark, "dark"), (Builtin::Light, "light")] {
        let mut harness = make(which);
        harness.run();
        prepare(&mut harness);
        harness.snapshot(format!("{name}-{theme}"));
        results.extend_harness(&mut harness);
    }
}

/// A log of its own, keeping `chosen`'s domains, and the dispatch that logs to it, with
/// the guard that keeps other tests' logs out while it is held.
pub fn kept(chosen: Option<&[Domain]>) -> (Kept, Dispatch, MutexGuard<'static, ()>) {
    let serial = serial();
    let (kept, dispatch) = Kept::scoped::<&str>(&[]).expect("a log");
    kept.keep(chosen, String::new);
    (kept, dispatch, serial)
}

/// Held while a test's log is in use: tracing's callsites keep one interest for every
/// dispatcher, rebuilt as a filter changes, and a rebuild in another test meanwhile can
/// lose an event of this one's.
fn serial() -> MutexGuard<'static, ()> {
    static SERIAL: Mutex<()> = Mutex::new(());
    SERIAL.lock().unwrap_or_else(PoisonError::into_inner)
}
