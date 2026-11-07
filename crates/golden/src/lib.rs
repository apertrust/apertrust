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

//! Golden-image harness: checks Apertrust modules against darktable-cli.
//!
//! darktable renders each case's module input and expected output as linear Rec2020
//! float images. An Apertrust module run on the input must match the expected output
//! within the case's CIEDE2000 tolerance. The harness needs darktable-cli
//! `darktable::REFERENCE_VERSION` and the darktable integration-test images, so it runs
//! locally rather than on hosted CI.

pub mod cases;
pub mod darktable;
pub mod delta_e;
pub mod heatmap;
pub mod pfm;
pub mod ports;
pub mod raw_stage;
pub mod render;
pub mod synthetic;
pub mod xmp;
