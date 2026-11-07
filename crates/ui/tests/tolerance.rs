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

//! The snapshot tolerance every snapshot test uses, `kittest.toml`'s, against what it must
//! absorb and what it must still catch (`docs/gui.md`, Testing). Two sliders are drawn as a
//! reference and again with one change, on the same GPU, and the second image is compared
//! with the first as a snapshot test compares a render with its snapshot: noise of 2/255 in
//! every channel of every pixel, as different GPUs rasterise, passes; a colour token 3/255
//! lighter, a slider moved a point and a label's last letter lost each fail.
//!
//! The images go to Cargo's temporary folder for tests, never `tests/snapshots`.

use std::path::Path;

use apertrust_ui::components::SliderRow;
use apertrust_ui::{Builtin, Theme};
use egui::Color32;
use egui_kittest::{Harness, SnapshotError, SnapshotOptions, SnapshotResult};

/// Two sliders as the reference draws them, or with one change.
#[derive(Clone, Copy)]
struct Panel {
    /// Added to each channel of the slider fill's token.
    fill: u8,
    /// Space above the second slider, in points.
    offset: f32,
    /// The second slider's label.
    label: &'static str,
}

const REFERENCE: Panel = Panel {
    fill: 0,
    offset: 0.0,
    label: "Contrast",
};

fn photograph(panel: Panel) -> Harness<'static> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(360.0, 120.0))
        .with_context_setup(move |ctx| {
            let mut theme = Theme::builtin(Builtin::Dark);
            let fill = theme.colors.slider_fill;
            theme.colors.slider_fill = Color32::from_rgb(
                fill.r().saturating_add(panel.fill),
                fill.g().saturating_add(panel.fill),
                fill.b().saturating_add(panel.fill),
            );
            apertrust_ui::install(ctx, theme);
        })
        .build_ui(move |ui| {
            let colour = apertrust_ui::theme(ui.ctx()).colors.panel;
            ui.painter()
                .rect_filled(ui.ctx().content_rect(), 0.0, colour);
            SliderRow::new("Exposure", 1.5, -4.0..=4.0)
                .decimals(2)
                .show(ui);
            ui.add_space(panel.offset);
            SliderRow::new(panel.label, 40.0, -100.0..=100.0).show(ui);
        });
    harness.run();
    harness
}

/// Adds -2 to 2 to every colour channel of an RGBA image, from a fixed sequence.
fn shake(rgba: &mut [u8]) {
    let mut state: u32 = 0x9E37_79B9;
    for pixel in rgba.as_chunks_mut::<4>().0 {
        for channel in &mut pixel[..3] {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            let step = (state % 5) as i16 - 2;
            *channel = (i16::from(*channel) + step).clamp(0, 255) as u8;
        }
    }
}

/// Compares `panel` with the reference under `kittest.toml`'s options, the reference
/// written as the snapshot `name` in a folder of its own.
fn compare(name: &str, panel: Panel, noise: bool) -> SnapshotResult {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join("snapshot-tolerance");
    std::fs::create_dir_all(&folder).expect("the temporary folder is writable");
    photograph(REFERENCE)
        .render()
        .expect("the reference renders")
        .save(folder.join(format!("{name}.png")))
        .expect("the reference is written");
    let mut image = photograph(panel).render().expect("the change renders");
    if noise {
        shake(&mut image);
    }
    egui_kittest::try_image_snapshot_options(
        &image,
        name,
        &SnapshotOptions::new().output_path(folder),
    )
}

/// Whether snapshots are being accepted (`UPDATE_SNAPSHOTS`, read as egui_kittest reads it),
/// when a comparison that fails writes the new image instead, so no change can be caught.
fn accepting() -> bool {
    std::env::var("UPDATE_SNAPSHOTS")
        .is_ok_and(|value| matches!(value.as_str(), "true" | "1" | "yes" | "on" | "force"))
}

/// Asserts that `panel` fails against the reference, as a changed snapshot must.
fn assert_caught(name: &str, panel: Panel) {
    if accepting() {
        return;
    }
    match compare(name, panel, false) {
        Err(SnapshotError::Diff { .. }) => {}
        other => panic!("the snapshot tolerance let {name} through: {other:?}"),
    }
}

#[test]
fn noise_of_two_levels_in_every_channel_passes() {
    if let Err(error) = compare("noise", REFERENCE, true) {
        panic!("{error}");
    }
}

#[test]
fn a_token_three_levels_lighter_fails() {
    assert_caught(
        "token",
        Panel {
            fill: 3,
            ..REFERENCE
        },
    );
}

#[test]
fn a_slider_moved_by_a_point_fails() {
    assert_caught(
        "offset",
        Panel {
            offset: 1.0,
            ..REFERENCE
        },
    );
}

#[test]
fn a_label_missing_its_last_letter_fails() {
    assert_caught(
        "letter",
        Panel {
            label: "Contras",
            ..REFERENCE
        },
    );
}
