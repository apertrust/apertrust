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

//! The export's sRGB profile against darktable's, byte for byte (decision 56).
//!
//! `tests/darktable/srgb.icc` is the profile darktable-cli embedded in a JPEG it
//! exported with its defaults on macOS (`darktable-cli 00.nikon.12b-lossy-raw.nef out.jpg
//! --core --library :memory:`, the raw.pixls.us corpus's Nikon D5100), cut from its APP2
//! marker; the PNG it exported carried the same profile but for its creation time.

use apertrust_export::profile;

const DARKTABLE: &[u8] = include_bytes!("darktable/srgb.icc");

/// The header's creation time, which each profile takes from the clock.
const CREATED: std::ops::Range<usize> = 24..36;

/// The header's preferred CMM, platform and creator, which darktable's name lcms2 and
/// `APPL` and Apertrust's leave as none.
const NAMES: [std::ops::Range<usize>; 3] = [4..8, 40..44, 80..84];

/// A profile's tags, each its signature and the range of its data.
fn tags(profile: &[u8]) -> Vec<([u8; 4], std::ops::Range<usize>)> {
    let u32_at = |at: usize| u32::from_be_bytes(profile[at..at + 4].try_into().unwrap()) as usize;
    (0..u32_at(128))
        .map(|i| {
            let entry = 132 + 12 * i;
            let (offset, size) = (u32_at(entry + 4), u32_at(entry + 8));
            (
                profile[entry..entry + 4].try_into().unwrap(),
                offset..offset + size,
            )
        })
        .collect()
}

/// `profile` with its creation time zeroed, no CMM, platform or creator named, and `from`
/// replaced by `to` in its device maker's tag, in ASCII and in Unicode.
fn with_maker(profile: &[u8], from: &str, to: &str) -> Vec<u8> {
    assert_eq!(from.len(), to.len());
    let mut profile = profile.to_vec();
    profile[CREATED].fill(0);
    for name in NAMES {
        profile[name].fill(0);
    }
    let (_, maker) = tags(&profile)
        .into_iter()
        .find(|(signature, _)| signature == b"dmnd")
        .expect("a device maker");
    let utf16 =
        |text: &str| -> Vec<u8> { text.encode_utf16().flat_map(u16::to_be_bytes).collect() };
    for (from, to) in [
        (from.as_bytes().to_vec(), to.as_bytes().to_vec()),
        (utf16(from), utf16(to)),
    ] {
        let data = &mut profile[maker.clone()];
        let at = data
            .windows(from.len())
            .position(|w| w == from)
            .unwrap_or_else(|| panic!("no {from:?} in the device maker"));
        data[at..at + from.len()].copy_from_slice(&to);
    }
    profile
}

#[test]
fn is_darktables_byte_for_byte_but_its_names_and_creation_time() {
    let mut ours = profile::srgb().to_vec();
    ours[CREATED].fill(0);
    // "Apertrust" and "darktable" have nine letters each, so the sizes and offsets agree.
    assert_eq!(ours, with_maker(DARKTABLE, "darktable", "Apertrust"));
    let names = |profile: &[u8]| NAMES.map(|name| profile[name].to_vec());
    assert_eq!(
        names(DARKTABLE),
        [b"lcms", b"APPL", b"lcms"].map(|n| n.to_vec())
    );
    assert_eq!(names(&ours), [[0; 4]; 3].map(|n| n.to_vec()));
}

#[test]
fn tags_lie_on_four_byte_boundaries_and_the_curves_are_one() {
    let ours = profile::srgb();
    assert_eq!(ours.len() % 4, 0);
    let tags = tags(ours);
    assert!(tags.iter().all(|(_, data)| data.start % 4 == 0));
    let curve = |signature: &[u8; 4]| {
        tags.iter()
            .find(|(s, _)| s == signature)
            .map(|(_, data)| data.clone())
            .unwrap()
    };
    assert_eq!(curve(b"rTRC"), curve(b"gTRC"));
    assert_eq!(curve(b"rTRC"), curve(b"bTRC"));
    // 4096 samples after the type, its reserved bytes and the count.
    assert_eq!(curve(b"rTRC").len(), 12 + 2 * 4096);
}

#[test]
fn is_stamped_once_a_process_with_the_time_it_was_first_made() {
    let profile = profile::srgb();
    let year = u16::from_be_bytes([profile[CREATED.start], profile[CREATED.start + 1]]);
    assert!(year >= 2026, "{year}");
    // Every export of a run embeds the same bytes, as darktable builds its profiles once.
    assert!(std::ptr::eq(profile, profile::srgb()));
}
