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

//! The Metadata tool's rows with several photos selected (decision 110): the session's
//! summary of them (`Command::Summarise`, `Event::Summary`), asked for as the selection
//! changes while the tool is shown and open, the latest kept, and worded in the language:
//! a shared value as one photo's row is worded, a range, a total, or Different with its
//! tooltip. The rows show nothing until the summary of the selection comes, "…" after
//! [`BLANK`].

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use apertrust_engine::session::Command;
use apertrust_engine::session::facts::{FactSummary, FactValue, Key, Summary};
use apertrust_ui::Translator;
use apertrust_ui::i18n::Args;

use super::format::{self, Of, Source, naive, say};

/// How long the rows stay blank before they show "…".
pub const BLANK: Duration = Duration::from_millis(150);

/// The summary the tool asked for, and the latest answer to it.
#[derive(Debug, Default)]
pub struct Asking {
    /// The selection's `through` it last asked for; none while it asks for none.
    asked: Option<(u64, (bool, u64))>,
    /// The number of the command that asked: an answer to an earlier one is not drawn.
    sent: u64,
    /// When it asked.
    since: Option<Instant>,
    /// The answer held, by its `through`.
    held: Option<(u64, Arc<Summary>)>,
}

impl Asking {
    /// What to send for `photos`, the selection as of its `through`, while `wanted`, the
    /// tool shown and open: a `Summarise` once for each selection of two or more photos, and
    /// again as the open photo's edit turns `edited` or not, which no new selection tells.
    /// Otherwise it asks for nothing and forgets what it held, so showing it again asks
    /// afresh.
    pub fn ask(
        &mut self,
        wanted: bool,
        through: u64,
        edited: (bool, u64),
        photos: &[PathBuf],
    ) -> Option<Command> {
        if !wanted || photos.len() < 2 {
            *self = Asking::default();
            return None;
        }
        if self.asked == Some((through, edited)) {
            return None;
        }
        // A new selection blanks the rows until its summary comes; an edit, the same
        // selection's, keeps them.
        if self.asked.map(|(asked, _)| asked) != Some(through) {
            self.held = None;
            self.since = Some(Instant::now());
        }
        self.asked = Some((through, edited));
        Some(Command::Summarise {
            photos: photos.to_vec(),
        })
    }

    /// The `Summarise` was sent as the command numbered `number`.
    pub fn sent(&mut self, number: u64) {
        self.sent = number;
    }

    /// Takes the session's summary for the command numbered `through` where it answers the
    /// latest request or a later one and is not older than the one held, as the Export
    /// tool keeps its preview.
    pub fn receive(&mut self, through: u64, summary: Arc<Summary>) {
        let older = (self.held.as_ref()).is_some_and(|(held, _)| through < *held);
        if self.asked.is_some() && through >= self.sent && !older {
            self.held = Some((through, summary));
        }
    }

    /// The summary held, where it is of the selection as of its `through`, the one asked
    /// for; none for an earlier selection's, while the new one is asked.
    pub fn held(&self, through: u64) -> Option<&Summary> {
        let asked = self.asked.map(|(asked, _)| asked);
        let held = self.held.as_ref().filter(|_| asked == Some(through));
        held.map(|(_, summary)| &**summary)
    }

    /// How long until the rows waiting show "…", while they wait; none once they do or
    /// a summary is held.
    /// Whether the rows waiting show "…": [`BLANK`] has passed since it asked.
    pub fn dotted(&self) -> bool {
        self.since.is_some_and(|since| since.elapsed() >= BLANK)
    }

    pub fn until_dotted(&self) -> Option<Duration> {
        let since = self.since.filter(|_| self.held.is_none())?;
        BLANK
            .checked_sub(since.elapsed())
            .filter(|left| !left.is_zero())
    }
}

/// What the rows are worded with.
pub struct Words<'a> {
    pub words: &'a Translator,
    pub home: Option<&'a Path>,
    /// The active photo's facts, whose value Different's tooltip marks.
    pub active: Option<Of<'a>>,
}

/// A row as several photos show it: its value, `None` for "—", empty while it waits; and
/// its tooltip.
#[derive(Debug, Default, PartialEq)]
pub struct Row {
    pub value: Option<String>,
    pub tip: String,
}

/// The row `id` of `summary`, the summary held, none until one comes; `none` the row's
/// words for a value no photo has.
pub fn row(id: &str, summary: Option<&Summary>, none: &str, cx: &Words<'_>) -> Row {
    let Some(summary) = summary else {
        // Nothing yet, then "…".
        return Row {
            value: Some(String::new()),
            tip: String::new(),
        };
    };
    let Some(fact) = summary.get(id) else {
        return Row {
            value: None,
            tip: none.to_owned(),
        };
    };
    let words = cx.words;
    match fact {
        FactSummary::Waiting { read, of } => Row {
            value: Some(DOTS.to_owned()),
            tip: say(
                words,
                "metadata-reading",
                &[("read", number(*read, words)), ("of", number(*of, words))],
            ),
        },
        FactSummary::None => Row {
            value: None,
            tip: none.to_owned(),
        },
        FactSummary::Same(value) if id == "edited" && value.key == Key::Bool(false) => Row {
            value: Some(words.text("metadata-edited-never")),
            tip: String::new(),
        },
        FactSummary::Same(value) if id != "edited" => {
            let shown = worded(id, value, cx);
            Row {
                tip: shown.clone(),
                // A value worded as nothing reads "—".
                value: Some(shown).filter(|shown| !shown.is_empty()),
            }
        }
        FactSummary::Same(_) | FactSummary::Different { .. } if id == "edited" => {
            let edited = edited_count(fact, summary.photos);
            Row {
                value: Some(words.text("metadata-different")),
                tip: counted(
                    words,
                    "metadata-edited-count",
                    &[],
                    &[("count", edited), ("of", summary.photos)],
                ),
            }
        }
        FactSummary::Different {
            values,
            more,
            missing,
        } => Row {
            value: Some(words.text("metadata-different")),
            tip: different(id, values, *more, *missing, cx),
        },
        FactSummary::Range { low, high, missing } => Row {
            value: Some(range(id, low, high, cx)).filter(|range| !range.is_empty()),
            tip: covers(summary.photos - missing, *missing, words),
        },
        FactSummary::Total { bytes } => Row {
            value: Some(say(
                words,
                "metadata-size-total",
                &[("size", format::size(*bytes, words))],
            )),
            tip: covers(summary.photos, 0, words),
        },
        FactSummary::Same(_) => Row::default(),
    }
}

/// What the rows waiting show.
const DOTS: &str = "…";

/// What a row shows while no summary is `held`: nothing, or "…" once [`BLANK`] has passed.
pub fn waiting(row: Row, held: bool, asking: &Asking) -> Row {
    if held || row.value.as_deref() != Some("") || !asking.dotted() {
        return row;
    }
    Row {
        value: Some(DOTS.to_owned()),
        ..row
    }
}

fn number(count: usize, words: &Translator) -> String {
    words.format_number(count as f64, 0, false)
}

/// How many photos are edited, from Edited's values.
fn edited_count(fact: &FactSummary, photos: usize) -> usize {
    match fact {
        FactSummary::Same(_) => photos,
        FactSummary::Different { values, .. } => (values.iter())
            .filter(|(value, _)| value.key == Key::Bool(true))
            .map(|(_, count)| count)
            .sum(),
        _ => 0,
    }
}

/// `value` worded as the row `id` words one photo's: through its shot's facts, or the
/// listing's own.
pub fn worded(id: &str, value: &FactValue, cx: &Words<'_>) -> String {
    if let Some(shot) = &value.shot {
        let source = Source {
            of: Of::shot(shot),
            history: None,
            home: cx.home,
            words: cx.words,
        };
        return format::fact(id, &source).value.unwrap_or_default();
    }
    match &value.key {
        Key::Text(text) if id == "folder" => crate::export::shown(Path::new(text), cx.home),
        Key::Text(text) => text.clone(),
        _ => String::new(),
    }
}

/// Different's tooltip: the values, most common first, each with its count and the active
/// photo's marked, "and 3 more values", then how many photos have none; a value some
/// share where the others have none, as "9 photos: Ricoh GR IV; 3 have none".
fn different(
    id: &str,
    values: &[(FactValue, usize)],
    more: usize,
    missing: usize,
    cx: &Words<'_>,
) -> String {
    let words = cx.words;
    let active = cx.active.map(|of| {
        let source = Source {
            of,
            history: None,
            home: cx.home,
            words,
        };
        format::fact(id, &source).value
    });
    if let ([(value, count)], true) = (values, missing > 0) {
        return counted(
            words,
            "metadata-shared-some",
            &[("value", worded(id, value, cx))],
            &[("count", *count), ("none", missing)],
        );
    }
    let mut lines: Vec<String> = (values.iter())
        .map(|(value, count)| {
            let mut shown = worded(id, value, cx);
            if active
                .as_ref()
                .is_some_and(|active| active.as_ref() == Some(&shown))
            {
                shown = say(words, "metadata-this-photo", &[("value", shown)]);
            }
            say(
                words,
                "metadata-value-count",
                &[("value", shown), ("count", number(*count, words))],
            )
        })
        .collect();
    if more > 0 {
        lines.push(counted(
            words,
            "metadata-more-values",
            &[],
            &[("count", more)],
        ));
    }
    if missing > 0 {
        lines.push(counted(
            words,
            "metadata-photos-none",
            &[],
            &[("count", missing)],
        ));
    }
    lines.join("\n")
}

/// A range's or a total's tooltip: how many photos it covers, and how many have none.
fn covers(covered: usize, missing: usize, words: &Translator) -> String {
    let mut lines = vec![counted(
        words,
        "metadata-covers",
        &[],
        &[("count", covered)],
    )];
    if missing > 0 {
        lines.push(counted(
            words,
            "metadata-photos-none",
            &[],
            &[("count", missing)],
        ));
    }
    lines.join("\n")
}

/// The message `id` with `texts`, and `counts` as numbers, which its plurals choose by.
fn counted(
    words: &Translator,
    id: &str,
    texts: &[(&str, String)],
    counts: &[(&str, usize)],
) -> String {
    let mut args = Args::new();
    for (name, text) in texts {
        args.set(*name, text.as_str());
    }
    for (name, count) in counts {
        args.set(*name, *count as f64);
    }
    words.text_with(id, Some(&args))
}

/// A range from `low` to `high`: Date taken's to the minute, the date once on one day and
/// the year once in one year; Exposure time's and ISO's with their unit once; Aperture's
/// as each end is worded.
fn range(id: &str, low: &FactValue, high: &FactValue, cx: &Words<'_>) -> String {
    let words = cx.words;
    let shot = |value: &FactValue| value.shot.clone();
    let (Some(low_shot), Some(high_shot)) = (shot(low), shot(high)) else {
        return String::new();
    };
    match id {
        "date_taken" => {
            let at = |exif: &apertrust_engine::session::Exif| exif.taken.as_ref().and_then(naive);
            let (Some(low), Some(high)) = (at(&low_shot.exif), at(&high_shot.exif)) else {
                return String::new();
            };
            dates(low, high, words)
        }
        "exposure_time" => {
            let seconds = |exif: &apertrust_engine::session::Exif| exif.exposure_time;
            let (Some(low), Some(high)) = (seconds(&low_shot.exif), seconds(&high_shot.exif))
            else {
                return String::new();
            };
            let shutter = |s| crate::exposure::shutter(s, words);
            say(
                words,
                "metadata-range-exposure",
                &[("low", shutter(low)), ("high", shutter(high))],
            )
        }
        "iso" => {
            let iso =
                |iso: Option<f32>| words.format_number(f64::from(iso.unwrap_or(0.0)), 0, false);
            say(
                words,
                "metadata-range-iso",
                &[("low", iso(low_shot.iso)), ("high", iso(high_shot.iso))],
            )
        }
        _ => say(
            words,
            "metadata-range",
            &[("low", worded(id, low, cx)), ("high", worded(id, high, cx))],
        ),
    }
}

/// Two dates and times, earliest first: "4 Jul 2026, 08:10 – 09:42" on one day; "4 Jul –
/// 6 Jul 2026" in one year; "8 Jul 2025 – 4 Jul 2026" over years.
fn dates(low: chrono::NaiveDateTime, high: chrono::NaiveDateTime, words: &Translator) -> String {
    use chrono::Datelike as _;
    let (from, to) = (words.format_time(low), words.format_time(high));
    // One minute: the date and the time once.
    if low.date() == high.date() && from == to {
        return say(
            words,
            "metadata-day-time",
            &[
                ("date", words.format_date(low.date(), true)),
                ("time", from),
            ],
        );
    }
    if low.date() == high.date() {
        return say(
            words,
            "metadata-range-day",
            &[
                ("date", words.format_date(low.date(), true)),
                ("low", from),
                ("high", to),
            ],
        );
    }
    let year = low.year() != high.year();
    say(
        words,
        "metadata-range",
        &[
            ("low", words.format_date(low.date(), year)),
            ("high", words.format_date(high.date(), true)),
        ],
    )
}

#[cfg(test)]
mod tests {
    use apertrust_ui::Locale;

    use super::*;

    fn at(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> chrono::NaiveDateTime {
        let date = chrono::NaiveDate::from_ymd_opt(year, month, day).unwrap();
        date.and_hms_opt(hour, minute, 0).unwrap()
    }

    /// The date once on one day and the year once in one year, in every language: the
    /// time is the medium date and time's, the date taken off.
    #[test]
    fn ranges_of_dates_write_the_date_and_the_year_once() {
        let words = Translator::new(Locale::EnGb);
        let ranges = [
            (at(2026, 7, 4, 8, 10), at(2026, 7, 4, 9, 42)),
            (at(2026, 7, 4, 8, 10), at(2026, 7, 6, 8, 10)),
            (at(2025, 7, 8, 8, 10), at(2026, 7, 4, 8, 10)),
        ];
        let written: Vec<String> = (ranges.iter())
            .map(|(low, high)| dates(*low, *high, &words))
            .collect();
        assert_eq!(
            written,
            [
                "4 Jul 2026, 08:10 – 09:42",
                "4 Jul – 6 Jul 2026",
                "8 Jul 2025 – 4 Jul 2026"
            ]
        );
        let minute = dates(at(2026, 7, 4, 8, 10), at(2026, 7, 4, 8, 10), &words);
        assert_eq!(minute, "4 Jul 2026, 08:10");
        for locale in [Locale::EnUs, Locale::Uk, Locale::Pl] {
            let words = Translator::new(locale);
            let (low, high) = ranges[0];
            let day = dates(low, high, &words);
            assert_eq!(day.matches("2026").count(), 1, "{locale:?}: {day}");
            let days = dates(ranges[1].0, ranges[1].1, &words);
            assert_eq!(days.matches("2026").count(), 1, "{locale:?}: {days}");
        }
    }

    /// While the facts are read, a row's tooltip counts them, the count moving with each
    /// answer.
    #[test]
    fn reading_counts_the_photos_read() {
        use apertrust_engine::session::facts::{Facts, Summed, build};
        let words = Translator::new(Locale::EnGb);
        let cx = Words {
            words: &words,
            home: None,
            active: None,
        };
        let folder = Path::new("/photos");
        let names: Vec<String> = (0..12).map(|n| format!("R{n:02}.DNG")).collect();
        let tip = |read: usize| {
            let photos: Vec<Summed<'_>> = (names.iter().enumerate())
                .map(|(n, name)| Summed {
                    name,
                    folder,
                    size: 1,
                    edited: false,
                    facts: if n < read {
                        Facts::Missing
                    } else {
                        Facts::Waiting
                    },
                })
                .collect();
            let summary = build(&photos);
            let row = row("lens", Some(&summary), "", &cx);
            assert_eq!(row.value.as_deref(), Some(DOTS));
            row.tip
        };
        assert_eq!(tip(1), "Reading 1 of 12 photos");
        assert_eq!(tip(6), "Reading 6 of 12 photos");
    }

    #[test]
    fn a_summary_for_an_earlier_request_or_selection_is_not_held() {
        let photos = vec![PathBuf::from("/a.ARW"), PathBuf::from("/b.ARW")];
        let mut asking = Asking::default();
        assert!(asking.ask(true, 3, (false, 0), &photos).is_some());
        asking.sent(10);
        // Asked once for each selection.
        assert!(asking.ask(true, 3, (false, 0), &photos).is_none());
        asking.receive(9, Arc::new(Summary::default()));
        assert!(asking.held(3).is_none(), "an earlier command's");
        asking.receive(10, Arc::new(Summary::default()));
        assert!(asking.held(3).is_some());
        // The open photo's edit turning edited asks again, keeping the rows meanwhile.
        assert!(asking.ask(true, 3, (true, 0), &photos).is_some());
        assert!(asking.held(3).is_some());
        assert!(asking.ask(true, 3, (true, 0), &photos).is_none());
        // A batch's end asks again too.
        assert!(asking.ask(true, 3, (true, 1), &photos).is_some());
        assert!(asking.held(3).is_some());
        // Not for the selection after it, until that is asked for and answered.
        assert!(asking.held(4).is_none());
        assert!(asking.ask(true, 4, (false, 0), &photos).is_some());
        assert!(asking.held(4).is_none());
        // One photo, or the tool closed, asks for nothing and forgets.
        assert!(asking.ask(true, 5, (false, 0), &photos[..1]).is_none());
        assert!(asking.ask(false, 6, (false, 0), &photos).is_none());
        asking.receive(12, Arc::new(Summary::default()));
        assert!(asking.held(6).is_none());
    }
}
