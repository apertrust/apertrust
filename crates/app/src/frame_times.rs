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

//! The window's frame times under `-d perf`, the measurement decision 112 starts from
//! (`docs/gui.md`, Performance): one line a second while a gesture or a render is under
//! way, of the intervals between the window's frames, the CPU time eframe reports for
//! each and the rest of the frame, the interval less that time: the acquire's and the
//! present's waits, but vsync's pacing and any idle time too, so it is no measure of the
//! wait alone.
//!
//! ```text
//!     4.2051 [perf] window: 12 frames, 83.2 ms, p95 95.1, worst 101.4, 11 over 20, CPU p95 3.1, rest 79.8
//! ```
//!
//! An interval counts where the frame before it ended under way, judged once its actions
//! have run, so the interval after the frame that sends an edit does, and the idle time
//! before a gesture never does; as the work ends, the second left unfinished is written
//! too, however few its frames.

use std::time::{Duration, Instant};

/// How often the line is written while the work lasts.
pub const PERIOD: Duration = Duration::from_secs(1);

/// The interval the line counts the frames over: a 60 Hz frame and some slack.
pub const LONG_MS: f64 = 20.0;

/// One frame: the interval since the frame before, and the CPU time eframe reports for
/// the frame before, which ran within it; `None` for the first frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    pub interval_ms: f64,
    pub cpu_ms: Option<f64>,
}

/// What the line says of a second's frames ([`summary`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Summary {
    pub frames: usize,
    pub median_ms: f64,
    pub p95_ms: f64,
    pub worst_ms: f64,
    /// How many intervals passed [`LONG_MS`].
    pub long: usize,
    /// The CPU time's 95th percentile, and the median of the rest of the frame, the interval
    /// less its CPU time: `None` where no frame of the second reported its CPU time.
    pub cpu_p95_ms: Option<f64>,
    pub rest_ms: Option<f64>,
}

/// The `p` quantile of `sorted`, by nearest rank: the smallest value at least a `p`
/// share of them are at or under. `sorted` is not empty.
fn quantile(sorted: &[f64], p: f64) -> f64 {
    let rank = (p * sorted.len() as f64).ceil() as usize;
    sorted[rank.clamp(1, sorted.len()) - 1]
}

fn sorted(values: impl Iterator<Item = f64>) -> Vec<f64> {
    let mut values: Vec<f64> = values.collect();
    values.sort_by(f64::total_cmp);
    values
}

/// The intervals' median, 95th percentile and worst, how many passed [`LONG_MS`], the CPU
/// time's 95th percentile and the median of the rest, each frame's rest its interval less
/// its CPU time; `None` for no samples.
pub fn summary(samples: &[Sample]) -> Option<Summary> {
    let intervals = sorted(samples.iter().map(|s| s.interval_ms));
    let worst = *intervals.last()?;
    let timed = || {
        samples
            .iter()
            .filter_map(|s| Some((s.interval_ms, s.cpu_ms?)))
    };
    let cpu = sorted(timed().map(|(_, cpu)| cpu));
    let rests = sorted(timed().map(|(interval, cpu)| (interval - cpu).max(0.0)));
    Some(Summary {
        frames: samples.len(),
        median_ms: quantile(&intervals, 0.5),
        p95_ms: quantile(&intervals, 0.95),
        worst_ms: worst,
        long: intervals.iter().filter(|&&ms| ms > LONG_MS).count(),
        cpu_p95_ms: (!cpu.is_empty()).then(|| quantile(&cpu, 0.95)),
        rest_ms: (!rests.is_empty()).then(|| quantile(&rests, 0.5)),
    })
}

/// The line for `summary`.
pub fn line(summary: &Summary) -> String {
    let Summary {
        frames,
        median_ms,
        p95_ms,
        worst_ms,
        long,
        ..
    } = summary;
    let mut line = format!(
        "window: {frames} {}, {median_ms:.1} ms, p95 {p95_ms:.1}, worst {worst_ms:.1}, {long} over {LONG_MS:.0}",
        if *frames == 1 { "frame" } else { "frames" }
    );
    if let (Some(cpu), Some(rest)) = (summary.cpu_p95_ms, summary.rest_ms) {
        line.push_str(&format!(", CPU p95 {cpu:.1}, rest {rest:.1}"));
    }
    line
}

/// The window's frames, gathered a second at a time while the work lasts.
#[derive(Debug, Default)]
pub struct FrameTimes {
    /// When this frame began.
    began: Option<Instant>,
    /// When the last frame began, while it ended under way.
    last: Option<Instant>,
    /// When the second being gathered began, and its frames.
    since: Option<Instant>,
    samples: Vec<Sample>,
}

impl FrameTimes {
    /// A frame beginning at `now`, with the CPU time eframe reports for the frame before:
    /// its interval counts where that frame ended under way. The line of the second it
    /// completes, if it completes one.
    pub fn start(&mut self, now: Instant, cpu_ms: Option<f64>) -> Option<String> {
        self.began = Some(now);
        let last = self.last?;
        let since = *self.since.get_or_insert(last);
        let interval_ms = now.duration_since(last).as_secs_f64() * 1e3;
        self.samples.push(Sample {
            interval_ms,
            cpu_ms,
        });
        if now.duration_since(since) < PERIOD {
            return None;
        }
        let done = summary(&self.samples).map(|summary| line(&summary));
        self.samples.clear();
        self.since = Some(now);
        done
    }

    /// The frame's end, `under_way` while a gesture or a render is, once its actions have
    /// run: where it is not, the work has ended, and the line of its unfinished second, if
    /// that has any frames.
    pub fn end(&mut self, under_way: bool) -> Option<String> {
        if under_way {
            self.last = self.began;
            return None;
        }
        let done = summary(&self.samples).map(|summary| line(&summary));
        *self = Self::default();
        done
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn samples(intervals: &[f64], cpu: Option<f64>) -> Vec<Sample> {
        (intervals.iter())
            .map(|&interval_ms| Sample {
                interval_ms,
                cpu_ms: cpu,
            })
            .collect()
    }

    #[test]
    fn the_summary_gives_the_median_the_95th_percentile_the_worst_and_the_long_ones() {
        // 1 to 100 ms, shuffled: by nearest rank the median is the 50th, the 95th
        // percentile the 95th.
        let mut intervals: Vec<f64> = (1..=100).map(f64::from).collect();
        intervals.reverse();
        intervals.swap(3, 70);
        let summary = summary(&samples(&intervals, Some(2.0))).expect("a summary");
        assert_eq!(summary.frames, 100);
        assert_eq!(summary.median_ms, 50.0);
        assert_eq!(summary.p95_ms, 95.0);
        assert_eq!(summary.worst_ms, 100.0);
        // 21 to 100 ms pass 20 ms; 20 ms itself does not.
        assert_eq!(summary.long, 80);
        assert_eq!(summary.cpu_p95_ms, Some(2.0));
        assert_eq!(summary.rest_ms, Some(48.0));
    }

    #[test]
    fn one_frame_is_its_own_median_and_worst() {
        let summary = summary(&samples(&[83.0], None)).expect("a summary");
        assert_eq!(
            (summary.median_ms, summary.p95_ms, summary.worst_ms),
            (83.0, 83.0, 83.0)
        );
        assert_eq!(summary.long, 1);
        // No CPU time reported, so no rest either.
        assert_eq!((summary.cpu_p95_ms, summary.rest_ms), (None, None));
    }

    #[test]
    fn the_cpu_time_and_the_rest_come_from_the_frames_that_report_it() {
        let samples = [
            Sample {
                interval_ms: 16.0,
                cpu_ms: None,
            },
            Sample {
                interval_ms: 80.0,
                cpu_ms: Some(4.0),
            },
            Sample {
                interval_ms: 70.0,
                cpu_ms: Some(3.0),
            },
            Sample {
                interval_ms: 90.0,
                cpu_ms: Some(5.0),
            },
        ];
        let summary = summary(&samples).expect("a summary");
        assert_eq!(summary.cpu_p95_ms, Some(5.0));
        // Rests of 67, 76 and 85 ms.
        assert_eq!(summary.rest_ms, Some(76.0));
        // A CPU time over its interval leaves no rest.
        let over = [Sample {
            interval_ms: 2.0,
            cpu_ms: Some(3.0),
        }];
        assert_eq!(super::summary(&over).expect("a summary").rest_ms, Some(0.0));
    }

    #[test]
    fn no_frames_no_summary() {
        assert_eq!(summary(&[]), None);
    }

    #[test]
    fn the_line_names_each_number() {
        let summary = Summary {
            frames: 12,
            median_ms: 83.2,
            p95_ms: 95.1,
            worst_ms: 101.4,
            long: 11,
            cpu_p95_ms: Some(3.1),
            rest_ms: Some(79.8),
        };
        assert_eq!(
            line(&summary),
            "window: 12 frames, 83.2 ms, p95 95.1, worst 101.4, 11 over 20, CPU p95 3.1, rest 79.8"
        );
        let alone = Summary {
            frames: 1,
            cpu_p95_ms: None,
            rest_ms: None,
            ..summary
        };
        assert_eq!(
            line(&alone),
            "window: 1 frame, 83.2 ms, p95 95.1, worst 101.4, 11 over 20"
        );
    }

    #[test]
    fn a_line_a_second_while_the_work_lasts() {
        let start = Instant::now();
        let at = |ms: u64| start + Duration::from_millis(ms);
        let mut times = FrameTimes::default();
        // The idle time before the work never counts.
        assert_eq!(times.start(at(0), Some(1.0)), None);
        assert_eq!(times.end(true), None);
        let mut lines = Vec::new();
        for ms in (50..=2000).step_by(50) {
            lines.extend(times.start(at(ms), Some(1.0)));
            lines.extend(times.end(true));
        }
        assert_eq!(lines.len(), 2);
        assert!(
            lines[0].starts_with("window: 20 frames, 50.0 ms,"),
            "{lines:?}"
        );
        assert!(
            lines[1].starts_with("window: 20 frames, 50.0 ms,"),
            "{lines:?}"
        );
        // The frame after the work's last counts, then the work is over: nothing counts
        // until a frame ends under way again.
        assert_eq!(times.start(at(2050), Some(1.0)), None);
        assert!(times.end(false).is_some());
        assert_eq!(times.start(at(5000), Some(1.0)), None);
        assert!(times.samples.is_empty());
        assert_eq!(times.end(true), None);
        assert_eq!(times.start(at(5016), Some(1.0)), None);
        assert_eq!(times.samples.len(), 1);
    }

    #[test]
    fn the_interval_after_a_frame_that_ends_under_way_counts() {
        // The frame that sends an edit begins with nothing pending and ends with it pending;
        // the next begins once its report has come, nothing pending again.
        let start = Instant::now();
        let mut times = FrameTimes::default();
        assert_eq!(times.start(start, None), None);
        assert_eq!(times.end(true), None);
        assert_eq!(
            times.start(start + Duration::from_millis(400), Some(2.0)),
            None
        );
        assert_eq!(
            times.end(false).as_deref(),
            Some(
                "window: 1 frame, 400.0 ms, p95 400.0, worst 400.0, 1 over 20, CPU p95 2.0, rest 398.0"
            )
        );
    }

    #[test]
    fn the_unfinished_second_is_written_as_the_work_ends() {
        // A half-second drag: four frames, one of them a 400 ms stall.
        let start = Instant::now();
        let at = |ms: u64| start + Duration::from_millis(ms);
        let mut times = FrameTimes::default();
        times.start(at(0), None);
        times.end(true);
        for ms in [16, 32, 432] {
            assert_eq!(times.start(at(ms), None), None);
            assert_eq!(times.end(true), None);
        }
        assert_eq!(times.start(at(448), None), None);
        let line = times.end(false).expect("the unfinished second's line");
        assert!(
            line.starts_with("window: 4 frames, 16.0 ms, p95 400.0, worst 400.0, 1 over 20"),
            "{line}"
        );
        // Then nothing is left to write.
        assert_eq!(times.end(false), None);
    }

    #[test]
    fn a_frame_after_a_long_freeze_completes_its_second() {
        let start = Instant::now();
        let mut times = FrameTimes::default();
        times.start(start, None);
        times.end(true);
        let line = times.start(start + Duration::from_millis(1500), Some(2.0));
        assert_eq!(
            line.as_deref(),
            Some(
                "window: 1 frame, 1500.0 ms, p95 1500.0, worst 1500.0, 1 over 20, CPU p95 2.0, rest 1498.0"
            )
        );
    }
}
