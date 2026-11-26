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

//! The rating's global actions (decision 110; `docs/gui.md`, The caption under the photo):
//! the stars, the reject and the colour labels, owned by no tool, which the keys, the
//! caption, the Browser's cells, the menus and the Toolbar all send as one `Command::Rate`.
//!
//! The session takes the value, never a toggle, so the app works out which way a toggle
//! goes from the photo it is clicked on, or for a key or a menu from the active photo
//! (decision 37): Reject and a label turn on unless that photo has them; the photo's
//! current star clears the rating. The active photo alone, until the batch (decision 109):
//! a click on another cell's first line opens that photo first, then rates it
//! ([`Pending`]).

use std::path::{Path, PathBuf};

use apertrust_engine::session::{Command, Rating, RatingChange};
use apertrust_ui::Translator;
use apertrust_ui::components::{CaptionTips, CellAction, CellRating, ColourLabel, ColourLabels};

use eframe::egui::os::OperatingSystem;

use crate::actions::{ActionId, Keymap};

/// One of the rating's actions, as the registry runs it ([`ActionId::RATING`] and the
/// rest).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rate {
    /// So many stars, 0 clearing the rating.
    Stars(u8),
    Reject,
    /// A label, 0 red to 4 purple.
    Label(u8),
    ClearLabels,
}

impl Rate {
    /// The change it makes on a photo rated `rating`: a toggle on where the photo lacks it,
    /// else off.
    pub fn change(self, rating: Rating) -> RatingChange {
        match self {
            Rate::Stars(stars) => RatingChange::Stars(stars),
            Rate::Reject => RatingChange::Rejected(!rating.rejected),
            Rate::Label(label) => RatingChange::Label {
                label,
                on: rating.labels & (1 << label) == 0,
            },
            Rate::ClearLabels => RatingChange::ClearLabels,
        }
    }

    /// A cell's or the caption's action as a rating action: a click on the photo's current
    /// star clears it.
    pub fn of_cell(action: CellAction, rating: Rating) -> Rate {
        match action {
            CellAction::Rate(stars) if stars == rating.stars => Rate::Stars(0),
            CellAction::Rate(stars) => Rate::Stars(stars),
            CellAction::ToggleReject => Rate::Reject,
            CellAction::ToggleLabel(label) => Rate::Label(index(label)),
            CellAction::ClearLabels => Rate::ClearLabels,
        }
    }

    /// Whether its menu item shows checked for a photo rated `rating`: the rating's stars,
    /// the reject while set, a label while on.
    pub fn checked(self, rating: Rating) -> Option<bool> {
        match self {
            Rate::Stars(stars) => Some(stars == rating.stars),
            Rate::Reject => Some(rating.rejected),
            Rate::Label(label) => Some(rating.labels & (1 << label) != 0),
            Rate::ClearLabels => None,
        }
    }
}

/// `change` on the active photo, as the session takes it.
pub fn command(change: RatingChange) -> Command {
    Command::Rate {
        photos: Vec::new(),
        change,
    }
}

/// A label's number, 0 red to 4 purple, darktable's order.
fn index(label: ColourLabel) -> u8 {
    (ColourLabel::ALL.iter())
        .position(|each| *each == label)
        .unwrap_or_default() as u8
}

/// The rating as a cell and the caption draw it.
pub fn cell(rating: Rating) -> CellRating {
    let labels = (ColourLabel::ALL.iter().enumerate())
        .filter(|(i, _)| rating.labels & (1 << i) != 0)
        .map(|(_, label)| *label);
    CellRating {
        stars: rating.stars,
        rejected: rating.rejected,
        labels: ColourLabels::of(labels),
    }
}

/// A rating a click on a cell's first line asked of a photo that is not the active one:
/// sent once that photo has opened, as the session rates the active photo alone.
#[derive(Clone, Debug, PartialEq)]
pub struct Pending {
    pub photo: PathBuf,
    pub change: RatingChange,
}

impl Pending {
    /// The command to send now that `opened` is the photo open, where it is this one's.
    pub fn take(pending: &mut Option<Pending>, opened: Option<&Path>) -> Option<Command> {
        let waiting = pending.as_ref()?;
        if opened != Some(waiting.photo.as_path()) {
            return None;
        }
        pending.take().map(|pending| command(pending.change))
    }
}

/// What Undo and Redo name for a rating's `change` (`Move::Field`): the action's name, a
/// reject taken back its own.
pub fn undo_words(change: RatingChange, tr: &Translator) -> String {
    let message = match change {
        RatingChange::Stars(stars) => ActionId::RATING[usize::from(stars.min(5))].0,
        RatingChange::Rejected(true) => ActionId::REJECT.0,
        RatingChange::Rejected(false) => "history-unreject",
        RatingChange::Label { label, .. } => ActionId::LABEL[usize::from(label.min(4))].0,
        RatingChange::ClearLabels => ActionId::LABEL_CLEAR.0,
    };
    let message = match message {
        "history-unreject" => message.to_owned(),
        id => format!("action-{id}"),
    };
    tr.text(&message)
}

/// The caption's tooltips, each with its key as `keymap` has it, and, while several photos
/// are selected, saying that it rates this photo alone until the batch.
pub fn tips(keymap: &Keymap, os: OperatingSystem, tr: &Translator, several: bool) -> CaptionTips {
    let with_key = |words: String, id: ActionId| match keymap.tip(id, os, tr) {
        Some(key) => {
            let mut args = apertrust_ui::i18n::Args::new();
            args.set("action", words);
            args.set("key", key);
            tr.text_with("caption-tip", Some(&args))
        }
        None => words,
    };
    let this = |words: String| {
        if several {
            let mut args = apertrust_ui::i18n::Args::new();
            args.set("action", words);
            tr.text_with("caption-this-photo", Some(&args))
        } else {
            words
        }
    };
    let plain = CaptionTips::plain_with(tr);
    CaptionTips {
        reject: with_key(this(plain.reject), ActionId::REJECT),
        unreject: with_key(this(plain.unreject), ActionId::REJECT),
        stars: std::array::from_fn(|i| {
            with_key(this(plain.stars[i].clone()), ActionId::RATING[i + 1])
        }),
        clear: with_key(this(plain.clear), ActionId::RATING[0]),
        labels: std::array::from_fn(|i| {
            with_key(this(plain.labels[i].clone()), ActionId::LABEL[i])
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rated(stars: u8, rejected: bool, labels: u8) -> Rating {
        Rating {
            stars,
            rejected,
            labels,
        }
    }

    /// A toggle follows the photo it acts on: on where it lacks it, else off; a star the
    /// value given, the current one from a click clearing it.
    #[test]
    fn toggles_follow_the_photo_and_its_current_star_clears_it() {
        let red = rated(3, true, 0b1);
        assert_eq!(Rate::Reject.change(red), RatingChange::Rejected(false));
        assert_eq!(
            Rate::Reject.change(rated(3, false, 0)),
            RatingChange::Rejected(true)
        );
        assert_eq!(
            Rate::Label(0).change(red),
            RatingChange::Label {
                label: 0,
                on: false
            }
        );
        assert_eq!(
            Rate::Label(4).change(red),
            RatingChange::Label { label: 4, on: true }
        );
        assert_eq!(Rate::Stars(3).change(red), RatingChange::Stars(3));
        assert_eq!(Rate::of_cell(CellAction::Rate(3), red), Rate::Stars(0));
        assert_eq!(Rate::of_cell(CellAction::Rate(4), red), Rate::Stars(4));
        let blue = CellAction::ToggleLabel(ColourLabel::Blue);
        assert_eq!(Rate::of_cell(blue, red), Rate::Label(3));
        assert_eq!(Rate::Label(0).checked(red), Some(true));
        assert_eq!(Rate::Stars(3).checked(red), Some(true));
        assert_eq!(Rate::ClearLabels.checked(red), None);
    }

    #[test]
    fn a_rating_draws_as_a_cell_s() {
        let cell = cell(rated(2, true, 0b1_0001));
        assert_eq!((cell.stars, cell.rejected), (2, true));
        let labels: Vec<_> = cell.labels.iter().collect();
        assert_eq!(labels, [ColourLabel::Red, ColourLabel::Purple]);
    }

    /// Edit › Undo names a rating's change by its action, a reject taken back its own.
    #[test]
    fn undo_names_a_rating_s_change_by_its_action() {
        let tr = Translator::new(apertrust_ui::Locale::EnGb);
        let words = |change| undo_words(change, &tr);
        assert_eq!(words(RatingChange::Stars(3)), "3 stars");
        assert_eq!(words(RatingChange::Stars(0)), "No stars");
        assert_eq!(words(RatingChange::Rejected(true)), "Reject");
        assert_eq!(words(RatingChange::Rejected(false)), "Clear reject");
        let red = RatingChange::Label { label: 0, on: true };
        assert_eq!(words(red), "Red label");
        assert_eq!(words(RatingChange::ClearLabels), "Clear labels");
    }

    /// A rating asked of another photo is sent once that photo is open, and only then.
    #[test]
    fn a_pending_rating_waits_for_its_photo() {
        let photo = PathBuf::from("/photos/B.ARW");
        let mut pending = Some(Pending {
            photo: photo.clone(),
            change: RatingChange::Stars(2),
        });
        let other = Path::new("/photos/A.ARW");
        assert_eq!(Pending::take(&mut pending, Some(other)), None);
        assert_eq!(Pending::take(&mut pending, None), None);
        assert_eq!(
            Pending::take(&mut pending, Some(&photo)),
            Some(command(RatingChange::Stars(2)))
        );
        assert_eq!(pending, None);
    }
}
