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

//! The Metadata tool's summary of several photos (decision 110): [`super::Command::Summarise`]
//! answered by [`super::Event::Summary`]. The session takes what it knows of the photos,
//! their listing and the facts read before; a job with no progress, off the engine thread,
//! reads the facts it lacks, each photo's once (`Shot::read`), hands them to the engine
//! thread to keep for the next summary, and answers as it goes, at most every [`AGAIN`],
//! the rows of photos not read yet `Waiting`, then once all are read. A later `Summarise`
//! cancels it; the facts it read by then are kept. A folder's open reads none of them: its
//! capture times come from the EXIF alone.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use apertrust_library::ops::Shot;
use apertrust_library::summary::{self, Facts, Summed};

use super::jobs::Job;
use super::{Event, Inbox, Outbox};

/// The least time between two answers to the same command while its facts are read.
pub(super) const AGAIN: Duration = Duration::from_millis(250);

/// A photo as the session knows it, owned so a job builds the summary off the engine
/// thread.
#[derive(Debug, Clone)]
pub(super) struct Known {
    pub path: PathBuf,
    pub name: String,
    pub folder: PathBuf,
    pub size: u64,
    pub edited: bool,
    /// `None` while its facts are not read yet; `Some(None)` where there are none.
    pub facts: Option<Option<Arc<Shot>>>,
}

impl Known {
    /// A photo the collection lacks: none of its facts, and none to read.
    pub fn outside(path: &Path) -> Known {
        Known {
            path: path.to_owned(),
            name: (path.file_name()).map_or_else(String::new, |n| n.to_string_lossy().into()),
            folder: path.parent().map(Path::to_owned).unwrap_or_default(),
            size: 0,
            edited: false,
            facts: Some(None),
        }
    }

    fn summed(&self) -> Summed<'_> {
        let shot = self.facts.as_ref().and_then(Option::as_ref);
        Summed {
            name: &self.name,
            folder: &self.folder,
            // A photo opened alone has its size from its facts.
            size: match (self.size, shot) {
                (0, Some(shot)) => shot.file_size,
                (size, _) => size,
            },
            edited: self.edited,
            facts: match (&self.facts, shot) {
                (None, _) => Facts::Waiting,
                (_, Some(shot)) => Facts::Read(shot),
                (Some(_), None) => Facts::Missing,
            },
        }
    }
}

/// What a summary's job answers with and to: the command's number, the collection's open,
/// which the facts read are kept for, and where the answers and the facts go.
pub(super) struct Run {
    pub through: u64,
    pub open: u64,
    pub outbox: Outbox,
    pub inbox: Sender<Inbox>,
    /// Holds the job before it reads the second photo's facts, for a test.
    #[cfg(test)]
    pub hold: Option<crate::export_job::testing::Hold>,
}

impl Run {
    fn answer(&self, known: &[Known]) {
        let photos: Vec<Summed<'_>> = known.iter().map(Known::summed).collect();
        self.outbox.send(Event::Summary {
            through: self.through,
            summary: Arc::new(summary::build(&photos)),
        });
    }

    /// Hands `read` to the engine thread to keep, emptying it.
    fn keep(&self, read: &mut Vec<(PathBuf, Option<Arc<Shot>>)>) {
        if !read.is_empty() {
            let facts = std::mem::take(read);
            let _ = self.inbox.send(Inbox::Facts {
                open: self.open,
                facts,
            });
        }
    }
}

/// The job answering `run` over `known`: an answer at once, the facts not read yet then
/// read in turn, an answer again at most every [`AGAIN`] while they are, and once they all
/// are. A cancel stops it between two photos, the facts read by then kept, with no answer.
pub(super) fn job(run: Run, mut known: Vec<Known>) -> Job {
    let description = format!("summary of {} photos", known.len());
    Job::new("summary", description, move |context| {
        let unread: Vec<usize> = (known.iter().enumerate())
            .filter(|(_, photo)| photo.facts.is_none())
            .map(|(at, _)| at)
            .collect();
        if context.cancelled() {
            return Ok(());
        }
        run.answer(&known);
        let mut read = Vec::new();
        let mut answered = Instant::now();
        for (count, at) in unread.iter().enumerate() {
            #[cfg(test)]
            if count == 1
                && let Some(hold) = &run.hold
            {
                hold.wait();
            }
            if context.cancelled() {
                run.keep(&mut read);
                return Ok(());
            }
            let shot = Shot::read(&known[*at].path).map(Arc::new);
            read.push((known[*at].path.clone(), shot.clone()));
            known[*at].facts = Some(shot);
            // Cancelled during the read: the facts kept, nothing sent.
            if context.cancelled() {
                run.keep(&mut read);
                return Ok(());
            }
            let last = count + 1 == unread.len();
            if !last && answered.elapsed() >= AGAIN {
                run.keep(&mut read);
                run.answer(&known);
                answered = Instant::now();
            }
        }
        run.keep(&mut read);
        if !unread.is_empty() && !context.cancelled() {
            run.answer(&known);
        }
        Ok(())
    })
}
