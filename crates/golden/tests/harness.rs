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

//! End-to-end check of the harness itself: a CPU exposure applied to darktable's input
//! render must reproduce darktable's expected render. Needs darktable-cli and the test
//! images, so it only runs on request:
//!
//! ```sh
//! APERTRUST_TEST_IMAGES=… cargo test -p apertrust-golden -- --ignored
//! ```

use golden::cases;
use golden::darktable::DarktableCli;
use golden::delta_e::compare;
use golden::pfm::Image;
use golden::render::{images_dir, render_case};

#[test]
#[ignore = "needs darktable-cli and APERTRUST_TEST_IMAGES"]
fn cpu_exposure_reproduces_darktable() {
    let darktable = DarktableCli::locate().unwrap();
    let case = cases::find("exposure").unwrap();
    let references = render_case(&darktable, case, &images_dir().unwrap()).unwrap();
    let input = Image::read(&references.input).unwrap();
    let expected = Image::read(&references.expected).unwrap();

    // darktable's exposure: out = (in - black) / (2^-exposure - black). The case uses
    // +1 EV with black 0, a gain of 2.
    let pixels = input.pixels.iter().map(|p| p.map(|v| v * 2.0)).collect();
    let actual = Image::new(input.width, input.height, pixels).unwrap();

    let (_, stats) = compare(&expected, &actual, case.tolerance).unwrap();
    assert!(stats.passes(case.tolerance), "{stats:?}");
    let (_, untouched) = compare(&expected, &input, case.tolerance).unwrap();
    assert!(
        !untouched.passes(case.tolerance),
        "the module should change the image: {untouched:?}"
    );
}
