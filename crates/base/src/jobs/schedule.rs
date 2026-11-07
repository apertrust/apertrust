// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2009-2025  darktable developers (ported from src/control/jobs.c)
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

//! darktable's adding and scheduling, `dt_control_add_job` and `_control_schedule_job`
//! in `src/control/jobs.c` at the reference release, restated as pure functions over the
//! queues (decision 57): [`add`] and [`pick`] change only the [`Queues`] they are given,
//! with no threads, time or locks, so tables worked through darktable's code hold them.
//! The [`Control`](super::Control) calls them under its lock.

use std::collections::VecDeque;

use super::Queue;

/// darktable's `DT_CONTROL_FG_PRIORITY`: the priority a job of a foreground queue starts
/// with; a background queue's starts with 0.
pub const FG_PRIORITY: u8 = 4;

/// darktable's `DT_CONTROL_MAX_JOBS`: how many jobs `SystemForeground`'s stack holds.
pub const MAX_JOBS: usize = 30;

/// What tells two jobs apart for `SystemForeground`'s deduplication, darktable's
/// `_control_job_equal` without the queue, which [`add`] compares itself: the kind for
/// darktable's execute function (Apertrust's jobs have no state callback to compare),
/// then the key for its params' bytes, or the description.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identity {
    pub kind: &'static str,
    pub key: Option<Vec<u8>>,
    pub description: String,
}

impl Identity {
    /// darktable's `_control_job_equal`: the keys compared where this one has a key that
    /// is not empty and the other's is as long, as darktable compares params by their
    /// size; otherwise the descriptions, so keys of different lengths fall back to the
    /// descriptions, as darktable's do.
    pub fn equal(&self, other: &Identity) -> bool {
        if self.kind != other.kind {
            return false;
        }
        match (&self.key, &other.key) {
            (Some(mine), Some(theirs)) if !mine.is_empty() && mine.len() == theirs.len() => {
                mine == theirs
            }
            _ => self.description == other.description,
        }
    }
}

/// A job as the queues hold it: what identifies it and darktable's per-job priority,
/// which ageing raises while it heads its queue and which it keeps when it moves.
pub trait Scheduled {
    fn identity(&self) -> &Identity;
}

/// A queued job with its priority, darktable's `unsigned char priority`, saturating
/// here (difference J15).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry<J> {
    pub job: J,
    pub priority: u8,
}

/// darktable's `control->queues`: one list per [`Queue`], in [`Queue::ALL`]'s order, the
/// head at the front.
#[derive(Clone, Debug)]
pub struct Queues<J> {
    lists: [VecDeque<Entry<J>>; 5],
}

impl<J> Default for Queues<J> {
    fn default() -> Self {
        Self {
            lists: std::array::from_fn(|_| VecDeque::new()),
        }
    }
}

impl<J> Queues<J> {
    /// The jobs a queue holds, its head first.
    pub fn list(&self, queue: Queue) -> &VecDeque<Entry<J>> {
        &self.lists[queue as usize]
    }

    fn list_mut(&mut self, queue: Queue) -> &mut VecDeque<Entry<J>> {
        &mut self.lists[queue as usize]
    }

    /// Takes the first job of a queue that `matches` from it, as a cancel does.
    pub fn remove(&mut self, queue: Queue, matches: impl Fn(&J) -> bool) -> Option<Entry<J>> {
        let list = self.list_mut(queue);
        let at = list.iter().position(|entry| matches(&entry.job))?;
        list.remove(at)
    }

    /// Empties every queue, as a shutdown discards what waits, in queue order.
    pub fn drain(&mut self) -> Vec<(Queue, Entry<J>)> {
        let mut drained = Vec::new();
        for queue in Queue::ALL {
            drained.extend(self.list_mut(queue).drain(..).map(|entry| (queue, entry)));
        }
        drained
    }

    /// How many jobs wait in every queue together.
    pub fn len(&self) -> usize {
        self.lists.iter().map(VecDeque::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// What [`add`] did with a job.
#[derive(Debug, PartialEq, Eq)]
pub enum Added<J> {
    /// Queued, at the stack's top for `SystemForeground`, else at its queue's end; past
    /// [`MAX_JOBS`], `dropped` the stack's bottom job, discarded.
    Queued { dropped: Option<J> },
    /// An equal job was queued in `SystemForeground`: it moved to the top, keeping its
    /// priority, and the new one, handed back, is discarded.
    Moved { new: J },
    /// An equal job is running: the new one, handed back, is discarded.
    Running { new: J },
}

/// darktable's `dt_control_add_job` for one of the five queues, given the running jobs
/// with their queues (darktable's `control->job`). `SystemForeground` is a stack of at
/// most [`MAX_JOBS`] with deduplication: a job equal to a running one of that queue is
/// discarded; one equal to a queued one moves that one to the top and is discarded;
/// past the limit the bottom one is. The other queues are FIFOs. A job starts at
/// [`FG_PRIORITY`] in the foreground queues and at 0 in the rest.
pub fn add<J: Scheduled>(
    queues: &mut Queues<J>,
    running: &[(Queue, J)],
    queue: Queue,
    job: J,
) -> Added<J> {
    if queue != Queue::SystemForeground {
        queues.list_mut(queue).push_back(Entry {
            job,
            priority: queue.priority(),
        });
        return Added::Queued { dropped: None };
    }

    let equal = |other: &J| job.identity().equal(other.identity());
    if running
        .iter()
        .any(|(other_queue, other)| *other_queue == queue && equal(other))
    {
        return Added::Running { new: job };
    }

    let list = queues.list_mut(queue);
    if let Some(at) = list.iter().position(|entry| equal(&entry.job)) {
        let moved = list.remove(at).expect("the position is in the list");
        list.push_front(moved);
        return Added::Moved { new: job };
    }

    list.push_front(Entry {
        job,
        priority: FG_PRIORITY,
    });
    let dropped = if list.len() > MAX_JOBS {
        list.pop_back().map(|entry| entry.job)
    } else {
        None
    };
    Added::Queued { dropped }
}

/// darktable's `_control_schedule_job`: each queue's head in queue order, `UserExport`
/// skipped while an export runs; the head of strictly highest priority wins, so a tie
/// goes to the earlier queue; it leaves its queue, and every other queue's head gains
/// one priority, `UserExport`'s too while it is skipped. Returns the winner with its
/// queue and the priority it won with, or `None` when no head may run.
///
/// A priority stops at 255, where darktable's `unsigned char` wraps back to 0 after a
/// head has lost 256 times in a row, and then loses to every other (difference J15).
pub fn pick<J>(queues: &mut Queues<J>, export_running: bool) -> Option<(Queue, Entry<J>)> {
    let mut winner = None;
    let mut max_priority = -1;
    for queue in Queue::ALL {
        if export_running && queue == Queue::UserExport {
            continue;
        }
        if let Some(head) = queues.list(queue).front()
            && i32::from(head.priority) > max_priority
        {
            max_priority = i32::from(head.priority);
            winner = Some(queue);
        }
    }
    let winner = winner?;
    let entry = queues
        .list_mut(winner)
        .pop_front()
        .expect("the winner has a head");
    for queue in Queue::ALL {
        if queue == winner {
            continue;
        }
        if let Some(head) = queues.list_mut(queue).front_mut() {
            head.priority = head.priority.saturating_add(1);
        }
    }
    Some((winner, entry))
}

#[cfg(test)]
mod tests {
    use super::*;
    use Queue::{SystemBackground, SystemForeground, UserBackground, UserExport, UserForeground};

    /// A job in the tables: a label, and an identity whose key is the label's bytes.
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct Test(&'static str, Identity);

    impl Scheduled for Test {
        fn identity(&self) -> &Identity {
            &self.1
        }
    }

    fn job(label: &'static str) -> Test {
        Test(
            label,
            Identity {
                kind: "test",
                key: Some(label.as_bytes().to_vec()),
                description: label.to_owned(),
            },
        )
    }

    /// Each queue's labels with their priorities, head first, in queue order.
    fn heads(queues: &Queues<Test>) -> Vec<Vec<(&'static str, u8)>> {
        Queue::ALL
            .iter()
            .map(|&queue| {
                queues
                    .list(queue)
                    .iter()
                    .map(|entry| (entry.job.0, entry.priority))
                    .collect()
            })
            .collect()
    }

    fn picked(queues: &mut Queues<Test>, export_running: bool) -> Option<(&'static str, u8)> {
        pick(queues, export_running).map(|(_, entry)| (entry.job.0, entry.priority))
    }

    fn add_all(queues: &mut Queues<Test>, jobs: &[(Queue, &'static str)]) {
        for &(queue, label) in jobs {
            assert_eq!(
                add(queues, &[], queue, job(label)),
                Added::Queued { dropped: None }
            );
        }
    }

    /// Ageing, worked through `_control_schedule_job` with six user foreground jobs
    /// (priority 4 each, `DT_CONTROL_FG_PRIORITY`) and one user background job (0):
    ///
    /// | pick | user fg head | user bg head | winner        | after             |
    /// | ---- | ------------ | ------------ | ------------- | ----------------- |
    /// | 1    | u1: 4        | b1: 0        | u1            | b1: 1             |
    /// | 2    | u2: 4        | b1: 1        | u2            | b1: 2             |
    /// | 3    | u3: 4        | b1: 2        | u3            | b1: 3             |
    /// | 4    | u4: 4        | b1: 3        | u4            | b1: 4             |
    /// | 5    | u5: 4        | b1: 4        | u5, tie: fg   | b1: 5             |
    /// | 6    | u6: 4        | b1: 5        | b1, 5 > 4     | u6: 5             |
    /// | 7    | u6: 5        | none         | u6            |                   |
    #[test]
    fn a_waiting_head_gains_a_priority_each_time_it_loses() {
        let mut queues = Queues::default();
        add_all(
            &mut queues,
            &[
                (UserForeground, "u1"),
                (UserForeground, "u2"),
                (UserForeground, "u3"),
                (UserForeground, "u4"),
                (UserForeground, "u5"),
                (UserForeground, "u6"),
                (UserBackground, "b1"),
            ],
        );
        let expected = [
            ("u1", 4, 1),
            ("u2", 4, 2),
            ("u3", 4, 3),
            ("u4", 4, 4),
            ("u5", 4, 5),
        ];
        for (label, priority, b1) in expected {
            assert_eq!(picked(&mut queues, false), Some((label, priority)));
            assert_eq!(heads(&queues)[UserBackground as usize][0], ("b1", b1));
        }
        assert_eq!(picked(&mut queues, false), Some(("b1", 5)));
        assert_eq!(heads(&queues)[UserForeground as usize], [("u6", 5)]);
        assert_eq!(picked(&mut queues, false), Some(("u6", 5)));
        assert_eq!(picked(&mut queues, false), None);
    }

    /// Every queue, with one export at a time, worked through `_control_schedule_job`.
    /// Heads before each pick, in queue order (user fg, system fg, user bg, export,
    /// system bg):
    ///
    /// | pick | heads                          | export runs | winner             |
    /// | ---- | ------------------------------ | ----------- | ------------------ |
    /// | 1    | u1 4, s1 4, b1 0, e1 0, g1 0   | no          | u1, tie: earlier   |
    /// | 2    | s1 5, b1 1, e1 1, g1 1         | no          | s1                 |
    /// | 3    | b1 2, e1 2, g1 2               | no          | b1, tie: earlier   |
    /// | 4    | e1 3, g1 3                     | no          | e1, tie: earlier   |
    /// | 5    | e2 0, g1 4                     | yes         | g1, export skipped |
    /// | 6    | e2 1                           | yes         | none               |
    /// | 7    | e2 1                           | no          | e2                 |
    ///
    /// e2 starts at 0 behind e1 and ages only once it heads its queue: not at pick 4,
    /// whose winner's queue it is, as darktable's loop raises every head but the
    /// winner queue's, then 1 after pick 5, where it is skipped but still ages; pick 6
    /// finds no head that may run and changes nothing.
    #[test]
    fn heads_compete_by_priority_then_queue_order_and_one_export_runs() {
        let mut queues = Queues::default();
        add_all(
            &mut queues,
            &[
                (SystemBackground, "g1"),
                (UserExport, "e1"),
                (UserExport, "e2"),
                (UserBackground, "b1"),
                (SystemForeground, "s1"),
                (UserForeground, "u1"),
            ],
        );
        assert_eq!(picked(&mut queues, false), Some(("u1", 4)));
        assert_eq!(picked(&mut queues, false), Some(("s1", 5)));
        assert_eq!(picked(&mut queues, false), Some(("b1", 2)));
        assert_eq!(picked(&mut queues, false), Some(("e1", 3)));
        assert_eq!(
            heads(&queues),
            [vec![], vec![], vec![], vec![("e2", 0)], vec![("g1", 4)]]
        );
        assert_eq!(picked(&mut queues, true), Some(("g1", 4)));
        assert_eq!(heads(&queues)[UserExport as usize], [("e2", 1)]);
        assert_eq!(picked(&mut queues, true), None);
        assert_eq!(heads(&queues)[UserExport as usize], [("e2", 1)]);
        assert_eq!(picked(&mut queues, false), Some(("e2", 1)));
        assert!(queues.is_empty());
    }

    /// A priority saturates: a head at 255 that loses, here a tie that goes to the
    /// earlier queue, stays at 255, where darktable's `unsigned char` wraps to 0.
    #[test]
    fn a_priority_saturates_at_255() {
        let mut queues = Queues::default();
        add_all(
            &mut queues,
            &[(UserForeground, "u1"), (UserBackground, "b1")],
        );
        queues.list_mut(UserForeground)[0].priority = 255;
        queues.list_mut(UserBackground)[0].priority = 255;
        assert_eq!(picked(&mut queues, false), Some(("u1", 255)));
        assert_eq!(heads(&queues)[UserBackground as usize], [("b1", 255)]);
    }

    /// `SystemForeground` as `dt_control_add_job` keeps it: a stack, so the newest runs
    /// first. Adding s1, s2, s3 then s1 again (equal by key):
    ///
    /// | add      | stack, top first | outcome                            |
    /// | -------- | ---------------- | ---------------------------------- |
    /// | s1       | s1               | queued                             |
    /// | s2       | s2 s1            | queued                             |
    /// | s3       | s3 s2 s1         | queued                             |
    /// | s1 again | s1 s3 s2         | the queued s1 moved up, new one discarded |
    #[test]
    fn system_foreground_is_a_stack_and_an_equal_job_moves_up() {
        let mut queues = Queues::default();
        add_all(
            &mut queues,
            &[
                (SystemForeground, "s1"),
                (SystemForeground, "s2"),
                (SystemForeground, "s3"),
            ],
        );
        assert_eq!(
            add(&mut queues, &[], SystemForeground, job("s1")),
            Added::Moved { new: job("s1") }
        );
        assert_eq!(
            heads(&queues)[SystemForeground as usize],
            [("s1", 4), ("s3", 4), ("s2", 4)]
        );
        assert_eq!(picked(&mut queues, false), Some(("s1", 4)));
        assert_eq!(picked(&mut queues, false), Some(("s3", 4)));
        assert_eq!(picked(&mut queues, false), Some(("s2", 4)));
    }

    /// A job that moves up keeps the priority it gained as the head: s1 heads the stack
    /// and ages to 5 as a user fg job wins; s2 goes on top at 4; s1 added again moves up
    /// at 5, not 4.
    #[test]
    fn a_job_that_moves_up_keeps_its_priority() {
        let mut queues = Queues::default();
        add_all(
            &mut queues,
            &[(SystemForeground, "s1"), (UserForeground, "u1")],
        );
        assert_eq!(picked(&mut queues, false), Some(("u1", 4)));
        add_all(&mut queues, &[(SystemForeground, "s2")]);
        assert_eq!(
            add(&mut queues, &[], SystemForeground, job("s1")),
            Added::Moved { new: job("s1") }
        );
        assert_eq!(
            heads(&queues)[SystemForeground as usize],
            [("s1", 5), ("s2", 4)]
        );
    }

    /// A job equal to a running one of `SystemForeground` is discarded; one equal to a
    /// running job of another queue is not, as darktable's equality compares queues.
    #[test]
    fn a_job_equal_to_a_running_one_is_discarded() {
        let mut queues = Queues::default();
        let running = [(SystemForeground, job("s1")), (UserForeground, job("s2"))];
        assert_eq!(
            add(&mut queues, &running, SystemForeground, job("s1")),
            Added::Running { new: job("s1") }
        );
        assert_eq!(
            add(&mut queues, &running, SystemForeground, job("s2")),
            Added::Queued { dropped: None }
        );
        assert_eq!(heads(&queues)[SystemForeground as usize], [("s2", 4)]);
    }

    /// Only `SystemForeground` deduplicates: the FIFOs queue equal jobs twice.
    #[test]
    fn the_other_queues_are_fifos_without_deduplication() {
        for queue in [UserForeground, UserBackground, UserExport, SystemBackground] {
            let mut queues = Queues::default();
            add_all(&mut queues, &[(queue, "a"), (queue, "b"), (queue, "a")]);
            let priority = queue.priority();
            assert_eq!(
                heads(&queues)[queue as usize],
                [("a", priority), ("b", priority), ("a", priority)],
                "{queue:?}"
            );
        }
    }

    /// Past [`MAX_JOBS`] the stack's bottom, the oldest, is discarded: 31 adds keep the
    /// last 30, j30 on top and j1 dropped.
    #[test]
    fn the_stack_holds_thirty_and_drops_its_bottom() {
        const LABELS: [&str; 31] = [
            "j0", "j1", "j2", "j3", "j4", "j5", "j6", "j7", "j8", "j9", "j10", "j11", "j12", "j13",
            "j14", "j15", "j16", "j17", "j18", "j19", "j20", "j21", "j22", "j23", "j24", "j25",
            "j26", "j27", "j28", "j29", "j30",
        ];
        let mut queues = Queues::default();
        for label in &LABELS[..MAX_JOBS] {
            add_all(&mut queues, &[(SystemForeground, label)]);
        }
        assert_eq!(
            add(&mut queues, &[], SystemForeground, job(LABELS[30])),
            Added::Queued {
                dropped: Some(job("j0"))
            }
        );
        let stack = &heads(&queues)[SystemForeground as usize];
        assert_eq!(stack.len(), MAX_JOBS);
        assert_eq!(stack[0], ("j30", 4));
        assert_eq!(stack[MAX_JOBS - 1], ("j1", 4));
    }

    /// `_control_job_equal`, row by row: the kind first; the keys where the first has
    /// one that is not empty and the second's is as long; otherwise the descriptions.
    ///
    /// | kinds | keys             | descriptions | equal | why                          |
    /// | ----- | ---------------- | ------------ | ----- | ---------------------------- |
    /// | same  | [1 2], [1 2]     | a, b         | yes   | keys compared                |
    /// | same  | [1 2], [1 3]     | a, a         | no    | keys compared, not the text  |
    /// | same  | [1 2], [1 2 3]   | a, a         | yes   | lengths differ: descriptions |
    /// | same  | [1 2], [1 2 3]   | a, b         | no    | lengths differ: descriptions |
    /// | same  | none, [1 2]      | a, a         | yes   | no key: descriptions         |
    /// | same  | [], []           | a, a         | yes   | empty, as size 0: text       |
    /// | same  | none, none       | a, b         | no    | descriptions                 |
    /// | other | [1 2], [1 2]     | a, a         | no    | another kind                 |
    #[test]
    fn jobs_are_equal_as_darktables_control_job_equal() {
        fn identity(kind: &'static str, key: Option<&[u8]>, description: &str) -> Identity {
            Identity {
                kind,
                key: key.map(<[u8]>::to_vec),
                description: description.to_owned(),
            }
        }
        let rows: [(Identity, Identity, bool); 8] = [
            (
                identity("t", Some(&[1, 2]), "a"),
                identity("t", Some(&[1, 2]), "b"),
                true,
            ),
            (
                identity("t", Some(&[1, 2]), "a"),
                identity("t", Some(&[1, 3]), "a"),
                false,
            ),
            (
                identity("t", Some(&[1, 2]), "a"),
                identity("t", Some(&[1, 2, 3]), "a"),
                true,
            ),
            (
                identity("t", Some(&[1, 2]), "a"),
                identity("t", Some(&[1, 2, 3]), "b"),
                false,
            ),
            (
                identity("t", None, "a"),
                identity("t", Some(&[1, 2]), "a"),
                true,
            ),
            (
                identity("t", Some(&[]), "a"),
                identity("t", Some(&[]), "a"),
                true,
            ),
            (identity("t", None, "a"), identity("t", None, "b"), false),
            (
                identity("t", Some(&[1, 2]), "a"),
                identity("u", Some(&[1, 2]), "a"),
                false,
            ),
        ];
        for (row, (first, second, equal)) in rows.iter().enumerate() {
            assert_eq!(first.equal(second), *equal, "row {row}");
        }
    }
}
