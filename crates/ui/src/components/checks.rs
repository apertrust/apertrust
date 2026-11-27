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

//! A sheet's choosing by check boxes (`docs/gui.md`, Choosing photos; decision 86): which
//! photos are checked, the sheet's active photo and the last touched, as plain data, with
//! the gestures the sheet's grid reports applied to it.
//!
//! The checks are what goes, never the window's selection: a plain click on a picture
//! only makes it the sheet's active photo, and every gesture that checks a range never
//! unchecks one or sets them as the last touched was set. Photos are by their place in the
//! grid's order.

/// What a person did to the sheet's checks, as the grid reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckGesture {
    /// A click on a picture, or an arrow key, Home or End to it: the sheet's active photo,
    /// the checks unchanged.
    Activate(usize),
    /// Ctrl-click (Cmd-click on the Mac) on a picture: its check turned and it the active
    /// photo, and the last touched.
    ToggleActivate(usize),
    /// Shift-click on a picture: every photo from the active one to it checked, none
    /// unchecked, the active photo staying.
    CheckRange(usize),
    /// A click on a check box, or Space on a cell: its check turned, it the last touched.
    Toggle(usize),
    /// Shift-click on a check box, or Shift+Space on a cell: every cell from the last
    /// touched to it set as the last touched was set.
    SetRange(usize),
    /// Shift with an arrow key: the focus moved from the cell focused, the first, to the
    /// second, every cell passed checked, both ends among them, and the second the active
    /// photo. The cell focused need not be the active one: a click on a check box focuses
    /// its cell and leaves the active photo.
    MoveChecking(usize, usize),
    /// Ctrl+A (Cmd+A), or the caller's Select all: every photo checked, none the last
    /// touched.
    All,
    /// Ctrl+Shift+A (Cmd+Shift+A), or the caller's Select none: none checked, none the last
    /// touched.
    None,
}

/// The checks of a sheet of `len` photos.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Checks {
    checked: Vec<bool>,
    count: usize,
    active: Option<usize>,
    last: Option<usize>,
}

impl Checks {
    /// `len` photos, none checked, none active.
    pub fn new(len: usize) -> Self {
        Checks {
            checked: vec![false; len],
            count: 0,
            active: None,
            last: None,
        }
    }

    /// How many photos it holds.
    pub fn len(&self) -> usize {
        self.checked.len()
    }

    pub fn is_empty(&self) -> bool {
        self.checked.is_empty()
    }

    pub fn is_checked(&self, photo: usize) -> bool {
        self.checked.get(photo).copied().unwrap_or(false)
    }

    /// How many are checked.
    pub fn checked_count(&self) -> usize {
        self.count
    }

    /// The photos checked, in the grid's order.
    pub fn checked(&self) -> impl Iterator<Item = usize> + '_ {
        (self.checked.iter().enumerate()).filter_map(|(photo, on)| on.then_some(photo))
    }

    /// The sheet's active photo, its own, never the window's.
    pub fn active(&self) -> Option<usize> {
        self.active
    }

    /// The photo whose check was turned last, which Shift on a check box sets from.
    pub fn last_touched(&self) -> Option<usize> {
        self.last
    }

    /// Checks `photo` or not, touching nothing else: the caller's own setting, as at
    /// opening, where the new photos are checked.
    pub fn set(&mut self, photo: usize, on: bool) {
        if let Some(check) = self.checked.get_mut(photo)
            && *check != on
        {
            *check = on;
            if on {
                self.count += 1;
            } else {
                self.count -= 1;
            }
        }
    }

    /// Makes `photo` the sheet's active photo, the checks unchanged.
    pub fn set_active(&mut self, photo: Option<usize>) {
        self.active = photo.filter(|p| *p < self.len());
    }

    /// Applies what the person did.
    pub fn apply(&mut self, gesture: CheckGesture) {
        let len = self.len();
        let within = |photo: usize| photo < len;
        match gesture {
            CheckGesture::Activate(photo) if within(photo) => self.active = Some(photo),
            CheckGesture::ToggleActivate(photo) if within(photo) => {
                self.toggle(photo);
                self.active = Some(photo);
            }
            CheckGesture::CheckRange(photo) if within(photo) => {
                let from = self.active.unwrap_or(photo);
                for p in span(from, photo) {
                    self.set(p, true);
                }
                self.active.get_or_insert(photo);
            }
            CheckGesture::Toggle(photo) if within(photo) => self.toggle(photo),
            CheckGesture::SetRange(photo) if within(photo) => match self.last {
                Some(last) => {
                    let on = self.is_checked(last);
                    for p in span(last, photo) {
                        self.set(p, on);
                    }
                    self.last = Some(photo);
                }
                None => self.toggle(photo),
            },
            CheckGesture::MoveChecking(from, to) if within(from) && within(to) => {
                for p in span(from, to) {
                    self.set(p, true);
                }
                self.active = Some(to);
            }
            // Every check set at once leaves no last touched: the next Shift gesture on a
            // check box acts as a plain one.
            CheckGesture::All => {
                self.checked.fill(true);
                self.count = len;
                self.last = None;
            }
            CheckGesture::None => {
                self.checked.fill(false);
                self.count = 0;
                self.last = None;
            }
            // A photo past the end, as a gesture from a grid since emptied.
            _ => {}
        }
    }

    /// Turns `photo`'s check and makes it the last touched.
    fn toggle(&mut self, photo: usize) {
        self.set(photo, !self.is_checked(photo));
        self.last = Some(photo);
    }
}

/// The photos from `a` to `b`, both included, whichever comes first.
fn span(a: usize, b: usize) -> std::ops::RangeInclusive<usize> {
    a.min(b)..=a.max(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ten photos, `on` checked, `active` the active one.
    fn checks(on: &[usize], active: Option<usize>) -> Checks {
        let mut checks = Checks::new(10);
        for photo in on {
            checks.set(*photo, true);
        }
        checks.set_active(active);
        checks
    }

    fn on(checks: &Checks) -> Vec<usize> {
        checks.checked().collect()
    }

    #[test]
    fn a_click_on_a_picture_makes_it_active_and_leaves_the_checks() {
        let mut c = checks(&[1, 2], Some(1));
        c.apply(CheckGesture::Activate(5));
        assert_eq!(c.active(), Some(5));
        assert_eq!(on(&c), [1, 2]);
        assert_eq!(c.last_touched(), None);
    }

    #[test]
    fn ctrl_click_on_a_picture_turns_its_check_and_makes_it_active() {
        let mut c = checks(&[1], Some(1));
        c.apply(CheckGesture::ToggleActivate(4));
        assert_eq!((on(&c), c.active()), (vec![1, 4], Some(4)));
        c.apply(CheckGesture::ToggleActivate(1));
        assert_eq!((on(&c), c.active()), (vec![4], Some(1)));
        assert_eq!(c.last_touched(), Some(1));
    }

    #[test]
    fn shift_click_on_a_picture_checks_from_the_active_one_both_ways_and_never_unchecks() {
        let mut c = checks(&[5], Some(3));
        c.apply(CheckGesture::CheckRange(6));
        assert_eq!(on(&c), [3, 4, 5, 6]);
        assert_eq!(c.active(), Some(3));
        // Backwards too, the active photo staying and nothing unchecked.
        c.apply(CheckGesture::CheckRange(1));
        assert_eq!(on(&c), [1, 2, 3, 4, 5, 6]);
        assert_eq!(c.active(), Some(3));
        // With none active, the photo alone, made active.
        let mut c = checks(&[], None);
        c.apply(CheckGesture::CheckRange(2));
        assert_eq!((on(&c), c.active()), (vec![2], Some(2)));
    }

    #[test]
    fn a_click_on_a_check_box_turns_it_and_makes_it_the_last_touched() {
        let mut c = checks(&[2], Some(0));
        c.apply(CheckGesture::Toggle(2));
        c.apply(CheckGesture::Toggle(7));
        assert_eq!(on(&c), [7]);
        assert_eq!(c.last_touched(), Some(7));
        // The active photo is the picture's, never the check box's.
        assert_eq!(c.active(), Some(0));
    }

    #[test]
    fn shift_on_a_check_box_sets_the_range_as_the_last_touched_was_set_both_ways() {
        // Checked last: the range from it is checked.
        let mut c = checks(&[], None);
        c.apply(CheckGesture::Toggle(2));
        c.apply(CheckGesture::SetRange(5));
        assert_eq!(on(&c), [2, 3, 4, 5]);
        assert_eq!(c.last_touched(), Some(5));
        // Unchecked last: the range back from it is unchecked.
        c.apply(CheckGesture::Toggle(4));
        c.apply(CheckGesture::SetRange(1));
        assert_eq!(on(&c), [5]);
        assert_eq!(c.last_touched(), Some(1));
        // With nothing touched yet, it turns the one cell.
        let mut c = checks(&[3], None);
        c.apply(CheckGesture::SetRange(3));
        assert_eq!(on(&c), Vec::<usize>::new());
        assert_eq!(c.last_touched(), Some(3));
    }

    #[test]
    fn shift_with_the_arrows_checks_the_cells_passed_and_moves_the_active_photo() {
        let mut c = checks(&[], Some(4));
        c.apply(CheckGesture::MoveChecking(4, 5));
        c.apply(CheckGesture::MoveChecking(5, 6));
        assert_eq!((on(&c), c.active()), (vec![4, 5, 6], Some(6)));
        c.apply(CheckGesture::MoveChecking(6, 2));
        assert_eq!((on(&c), c.active()), (vec![2, 3, 4, 5, 6], Some(2)));
        // From a cell focused that is not the active photo, as after a check box's click:
        // the range runs from the cell focused, never from the active photo.
        let mut c = checks(&[], Some(0));
        c.apply(CheckGesture::MoveChecking(7, 8));
        assert_eq!((on(&c), c.active()), (vec![7, 8], Some(8)));
    }

    #[test]
    fn select_all_and_none_keep_the_active_photo_and_forget_the_last_touched() {
        let mut c = checks(&[1], Some(3));
        c.apply(CheckGesture::Toggle(8));
        c.apply(CheckGesture::All);
        assert_eq!(c.checked_count(), 10);
        assert_eq!((c.active(), c.last_touched()), (Some(3), None));
        c.apply(CheckGesture::Toggle(8));
        c.apply(CheckGesture::None);
        assert_eq!(c.checked_count(), 0);
        assert_eq!((c.active(), c.last_touched()), (Some(3), None));
        // The next Shift on a check box turns its cell alone, as a plain click, never
        // clearing a range from a check Select none left unchecked.
        c.apply(CheckGesture::SetRange(5));
        assert_eq!(on(&c), [5]);
        assert_eq!(c.last_touched(), Some(5));
        c.apply(CheckGesture::All);
        c.apply(CheckGesture::SetRange(2));
        assert_eq!(c.checked_count(), 9);
        assert!(!c.is_checked(2));
    }

    #[test]
    fn a_gesture_past_the_end_does_nothing_and_the_count_follows_the_checks() {
        let mut c = checks(&[0, 9], Some(0));
        let before = c.clone();
        for gesture in [
            CheckGesture::Activate(10),
            CheckGesture::Toggle(12),
            CheckGesture::SetRange(10),
            CheckGesture::CheckRange(11),
        ] {
            c.apply(gesture);
        }
        assert_eq!(c, before);
        c.set(9, true);
        c.set(4, true);
        assert_eq!(c.checked_count(), 3);
    }
}
