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

//! `cargo run -p apertrust-ui --example gallery`: the design system's gallery in a window,
//! with a theme switcher. Ctrl + and Ctrl − change the scale.

use apertrust_ui::gallery::Gallery;
use apertrust_ui::{Builtin, Theme};

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Apertrust · gallery")
            .with_inner_size([820.0, 960.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Apertrust gallery",
        options,
        Box::new(|cc| {
            apertrust_ui::install(&cc.egui_ctx, Theme::builtin(Builtin::default()));
            Ok(Box::new(App {
                gallery: Gallery::new(Builtin::default()),
            }))
        }),
    )
}

struct App {
    gallery: Gallery,
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| self.gallery.show(ui));
        });
    }
}
