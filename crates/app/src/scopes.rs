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

//! The Scopes section (`docs/gui.md`, The scopes; decision 19): a utility section pinned
//! at the top of the side panel's editing tabs, showing the design system's `Histogram`
//! with what the engine counted of the frame shown.
//!
//! - **What it shows:** the app's `ScopeView` (the graph, the scale, the channels, the
//!   clipping overlays' toggles), which the shell keeps in settings, and the latest
//!   [`Reading`], kept as the widget takes it ([`Measured`]); a reading of a photo that
//!   has closed is dropped with it. The clipping toggles are drawn disabled until the app
//!   draws the overlays on the photo.
//! - **What it asks:** the session counts only what [`Command::Scopes`] names, so the app
//!   asks for the totals while the section is in the side panel, open or closed, and for
//!   the graph shown while it is open ([`wanted`]); a change of graph, or the section
//!   opening or closing, asks again ([`Scopes::ask`]). The session counts the frame shown
//!   again for a graph it was not counted for, so a graph chosen shows at once.

use apertrust_engine::session::{Command, Graphs, Reading};
use apertrust_ui::components::{
    Clipped, Histogram, HistogramAction, HistogramBins, Scope, ScopeView, Section, SectionAction,
    VectorscopeBins, WaveformBins,
};
use apertrust_ui::tr;
use eframe::egui::{self, Color32};

/// The section's id, by which settings keep whether it is open.
pub const ID: &str = "scopes";

/// Whether the section is open when settings have not kept it otherwise.
pub const OPEN: bool = true;

/// What the user did to the section this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    /// Open it, or close it.
    Toggle,
    /// The graph, the scale, a channel or a clipping toggle.
    View(HistogramAction),
}

/// What the session is to count of each frame: nothing while the section is not in the
/// side panel (`pinned`); the totals while it is, open or closed; and the graph `view`
/// shows while it is `open`, the waveform's counts for the waveform and the parade.
pub fn wanted(pinned: bool, open: bool, view: ScopeView) -> Option<Graphs> {
    if !pinned {
        return None;
    }
    let scope = view.scope;
    Some(Graphs {
        histogram: open && scope == Scope::Histogram,
        waveform: open && matches!(scope, Scope::Waveform | Scope::Parade),
        vectorscope: open && scope == Scope::Vectorscope,
    })
}

/// A reading's counts as the widget takes them, each count a number of pixels.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Measured {
    /// Red, green, blue and luminance, when the reading has the histogram.
    histogram: Option<[Vec<f32>; 4]>,
    /// The waveform's columns, levels and channels, when the reading has it.
    waveform: Option<(usize, usize, [Vec<f32>; 4])>,
    /// The vectorscope's cells across, their counts and their colours, when the reading
    /// has it.
    vectorscope: Option<(usize, Vec<f32>, Vec<Color32>)>,
    /// What clips, as shares of the pixels counted; none when nothing was.
    clipped: Option<Clipped>,
}

impl Measured {
    pub fn new(reading: &Reading) -> Measured {
        // Viewport frames have far fewer pixels than an f32 counts exactly, 2^24.
        let counts = |counts: &[u32]| counts.iter().map(|&c| c as f32).collect::<Vec<f32>>();
        let histogram = reading
            .histogram
            .as_ref()
            .map(|h| [&h.red, &h.green, &h.blue, &h.luminance].map(|channel| counts(channel)));
        let waveform = reading.waveform.as_ref().map(|w| {
            let channels = [&w.red, &w.green, &w.blue, &w.luminance].map(|c| counts(c));
            (w.columns, w.levels, channels)
        });
        let vectorscope = reading.vectorscope.as_ref().map(|v| {
            // The engine's colours are sRGB-encoded, as egui's.
            let colours = v
                .colours
                .iter()
                .map(|&[r, g, b]| Color32::from_rgb(r, g, b));
            (v.size, counts(&v.counts), colours.collect())
        });
        let totals = reading.totals;
        let share = |n: u32| (f64::from(n) / f64::from(totals.pixels)) as f32;
        let clipped = (totals.pixels > 0).then(|| Clipped {
            shadows: share(totals.shadows),
            highlights: share(totals.highlights),
        });
        Measured {
            histogram,
            waveform,
            vectorscope,
            clipped,
        }
    }
}

/// What the section mirrors of the session: the latest reading, and what the session was
/// last asked to count.
#[derive(Debug, Default)]
pub struct Scopes {
    measured: Option<Measured>,
    /// As the session starts, nothing until asked.
    asked: Option<Graphs>,
}

impl Scopes {
    /// Keeps `reading`, the latest: readings arrive in the order their frames were counted.
    pub fn receive(&mut self, reading: &Reading) {
        self.measured = Some(Measured::new(reading));
    }

    /// Drops the reading kept: its photo has closed, or the scopes have stopped, so the
    /// section says nothing is measured rather than show what no longer is.
    pub fn forget(&mut self) {
        self.measured = None;
    }

    pub fn measured(&self) -> Option<&Measured> {
        self.measured.as_ref()
    }

    /// The command that asks the session to count `wanted`, unless it was asked already.
    pub fn ask(&mut self, wanted: Option<Graphs>) -> Option<Command> {
        (wanted != self.asked).then(|| {
            self.asked = wanted;
            Command::Scopes(wanted)
        })
    }
}

/// Draws the section, open or closed as `open` says, its graph as `view` has it with what
/// `measured` holds, drawn disabled unless `enabled` (no photo is shown); it peeks while
/// `peek` holds, as every section does. Returns what the user did.
pub fn section(
    ui: &mut egui::Ui,
    view: ScopeView,
    open: bool,
    measured: Option<&Measured>,
    peek: bool,
    enabled: bool,
) -> Vec<Change> {
    let title = tr(ui.ctx(), "section-scopes");
    let mut action = None;
    // A utility: options only, drawn disabled while nothing is behind them.
    let out = Section::new(&title, open)
        .options(false)
        .peek(peek)
        .show(ui, |ui| {
            let mut scopes = Histogram::new(view).overlays(false);
            if let Some(measured) = measured {
                if let Some([red, green, blue, luminance]) = &measured.histogram {
                    scopes = scopes.histogram(HistogramBins {
                        red,
                        green,
                        blue,
                        luminance,
                    });
                }
                if let Some((columns, levels, [red, green, blue, luminance])) = &measured.waveform {
                    scopes = scopes.waveform(WaveformBins {
                        columns: *columns,
                        levels: *levels,
                        red,
                        green,
                        blue,
                        luminance,
                    });
                }
                if let Some((size, counts, colours)) = &measured.vectorscope {
                    scopes = scopes.vectorscope(VectorscopeBins {
                        size: *size,
                        counts,
                        colours,
                    });
                }
                if let Some(clipped) = measured.clipped {
                    scopes = scopes.clipped(clipped);
                }
            }
            action = ui.add_enabled_ui(enabled, |ui| scopes.show(ui)).inner;
        });
    // Options is drawn disabled: nothing is behind it yet.
    let toggled = out
        .actions
        .into_iter()
        .filter(|a| *a == SectionAction::Toggle);
    let mut changes: Vec<Change> = toggled.map(|_| Change::Toggle).collect();
    changes.extend(action.map(Change::View));
    changes
}

/// What tests of the scopes share.
#[cfg(test)]
pub mod testing {
    use std::sync::Arc;

    use apertrust_engine::scopes::{
        BINS, CELLS, COLUMNS, Histogram, LEVELS, Totals, Vectorscope, Waveform,
    };
    use apertrust_engine::session::Reading;

    /// A reading of a frame of 60,000 pixels, 6 of them clipped black and 3,000 white,
    /// with the histogram, the waveform or the vectorscope as asked, each channel's counts
    /// flat, and the vectorscope's every cell azure.
    pub fn reading(histogram: bool, waveform: bool, vectorscope: bool) -> Reading {
        let flat = |n: usize, at: u32| vec![at; n];
        Reading {
            through: 7,
            width: 300,
            height: 200,
            totals: Totals {
                pixels: 60_000,
                shadows: 6,
                highlights: 3_000,
            },
            histogram: histogram.then(|| Histogram {
                red: flat(BINS, 1),
                green: flat(BINS, 2),
                blue: flat(BINS, 3),
                luminance: flat(BINS, 4),
            }),
            waveform: waveform.then(|| Waveform {
                columns: COLUMNS,
                levels: LEVELS,
                red: flat(COLUMNS * LEVELS, 5),
                green: flat(COLUMNS * LEVELS, 6),
                blue: flat(COLUMNS * LEVELS, 7),
                luminance: flat(COLUMNS * LEVELS, 8),
            }),
            vectorscope: vectorscope.then(|| {
                Box::new(Vectorscope {
                    size: CELLS,
                    counts: flat(CELLS * CELLS, 9),
                    colours: Arc::from(vec![[0, 136, 255]; CELLS * CELLS]),
                })
            }),
            gpu_ms: f64::NAN,
            latency_ms: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use apertrust_engine::scopes::{BINS, CELLS, COLUMNS, LEVELS, Totals};

    use super::testing::reading;
    use super::*;

    #[test]
    fn the_totals_are_asked_while_the_section_is_there_and_the_graph_while_it_is_open() {
        let view = |scope| ScopeView {
            scope,
            ..ScopeView::default()
        };
        let histogram = view(Scope::Histogram);
        assert_eq!(wanted(false, true, histogram), None);
        assert_eq!(wanted(true, false, histogram), Some(Graphs::default()));
        let counted = |histogram, waveform, vectorscope| {
            Some(Graphs {
                histogram,
                waveform,
                vectorscope,
            })
        };
        assert_eq!(wanted(true, true, histogram), counted(true, false, false));
        assert_eq!(
            wanted(true, true, view(Scope::Waveform)),
            counted(false, true, false)
        );
        assert_eq!(
            wanted(true, true, view(Scope::Parade)),
            counted(false, true, false)
        );
        // The vectorscope, chosen or kept in settings, is counted while the section is
        // open, and closed only the totals are.
        let vectorscope = view(Scope::Vectorscope);
        assert_eq!(wanted(true, true, vectorscope), counted(false, false, true));
        assert_eq!(wanted(true, false, vectorscope), Some(Graphs::default()));
        assert_eq!(wanted(false, true, vectorscope), None);
    }

    #[test]
    fn the_session_is_asked_only_when_what_is_wanted_changes() {
        let mut scopes = Scopes::default();
        // As the session starts, nothing is counted: nothing to ask for that.
        assert_eq!(scopes.ask(None), None);
        let totals = Some(Graphs::default());
        assert_eq!(scopes.ask(totals), Some(Command::Scopes(totals)));
        assert_eq!(scopes.ask(totals), None);
        assert_eq!(scopes.ask(None), Some(Command::Scopes(None)));
    }

    #[test]
    fn a_reading_is_kept_as_the_widget_takes_it() {
        let measured = Measured::new(&reading(true, false, false));
        let histogram = measured.histogram.as_ref().expect("asked for");
        assert_eq!(histogram.each_ref().map(Vec::len), [BINS; 4]);
        assert_eq!(histogram.each_ref().map(|c| c[0]), [1.0, 2.0, 3.0, 4.0]);
        assert_eq!(measured.waveform, None);
        assert_eq!(
            measured.clipped,
            Some(Clipped {
                shadows: 0.0001,
                highlights: 0.05,
            })
        );
        let (columns, levels, channels) = Measured::new(&reading(false, true, false))
            .waveform
            .expect("asked for");
        assert_eq!((columns, levels), (COLUMNS, LEVELS));
        assert_eq!(channels.each_ref().map(|c| c[0]), [5.0, 6.0, 7.0, 8.0]);
        let (size, counts, colours) = Measured::new(&reading(false, false, true))
            .vectorscope
            .expect("asked for");
        assert_eq!(
            (size, counts.len(), colours.len()),
            (CELLS, CELLS * CELLS, CELLS * CELLS)
        );
        assert_eq!(counts[0], 9.0);
        assert_eq!(colours[0], Color32::from_rgb(0, 136, 255));
        // Nothing counted: nothing clips, nor is anything measured.
        let empty = Reading {
            totals: Totals::default(),
            ..reading(false, false, false)
        };
        assert_eq!(Measured::new(&empty), Measured::default());

        let mut scopes = Scopes::default();
        scopes.receive(&reading(true, false, false));
        assert!(scopes.measured().is_some());
        scopes.forget();
        assert_eq!(scopes.measured(), None);
    }
}
