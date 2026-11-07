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

//! Golden cases: one darktable module each.
//!
//! Every case renders a raw through darktable twice, with the module under test off and
//! on. The first render is the module's input, the second its expected output. Modules
//! that run after it in darktable's pipeline and change the image (tone mappers) are
//! switched off in both renders, so the two differ only by the module.
//!
//! The sidecars in `cases/` come from darktable's integration tests (the
//! darktable-tests repository, commit 3f2d1ab of 2026-10-22), unchanged unless a case's
//! comment says what was edited.

use crate::delta_e::Tolerance;

#[derive(Debug, Clone, Copy)]
pub struct Case {
    pub name: &'static str,
    /// The module under test, as darktable names its operation.
    pub operation: &'static str,
    /// Raw file name in the darktable integration-test images.
    pub image: &'static str,
    /// The darktable sidecar with the module's history entry.
    pub xmp: &'static str,
    /// Later operations switched off in both renders.
    pub disable: &'static [&'static str],
    /// An unmasked operation whose blend settings replace the module's, when the source
    /// test applies the module through a mask. Masks belong to the layer system; module
    /// cases test the whole image.
    pub unmask_like: Option<&'static str>,
    pub tolerance: Tolerance,
}

pub const CASES: &[Case] = &[
    Case {
        // Test 0001-exposure: +1 EV, zero black level. darktable runs exposure before the
        // input colour matrix; a pure gain commutes with it, so the Rec2020 renders still
        // differ by exactly this module.
        name: "exposure",
        operation: "exposure",
        image: "mire1.cr2",
        xmp: include_str!("../cases/exposure.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0152-rgb-primaries: every setting off its default, the tint hue 33.2° at
        // purity 13.4%, red +9.5° at 84.3%, green -8.2° at 123.5%, blue +13.1° at 82%.
        // Sigmoid runs after it and changes the image.
        name: "primaries",
        operation: "primaries",
        image: "mire1.cr2",
        xmp: include_str!("../cases/primaries.xmp"),
        disable: &["sigmoid"],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0083-colorbalancergb.
        name: "colorbalancergb",
        operation: "colorbalancergb",
        image: "mire1.cr2",
        xmp: include_str!("../cases/colorbalancergb.xmp"),
        disable: &["filmicrgb"],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0093-colorbalancergb-ucs: params version 5 with the darktable UCS
        // saturation formula, chroma, saturation and brilliance per range, vibrance.
        name: "colorbalancergb-ucs",
        operation: "colorbalancergb",
        image: "mire1.cr2",
        xmp: include_str!("../cases/colorbalancergb-ucs.xmp"),
        disable: &["filmicrgb"],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0159-coloreq-guided-filter: params version 1, which darktable
        // migrates to version 4, with the guided filter, darktable's default: a hue
        // analysis radius of 2, whose prefilter runs on the image itself, and an effect
        // radius of 25, whose guide runs on a grid a fourth of it. Sigmoid runs after it
        // and changes the image.
        name: "colorequal",
        operation: "colorequal",
        image: "mire1.cr2",
        xmp: include_str!("../cases/colorequal.xmp"),
        disable: &["sigmoid"],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0158-coloreq-no-guided-filter: params version 1 without the guided
        // filter, so per pixel. The sidecar has two entries for the module; the second
        // is active.
        name: "colorequal-no-filter",
        operation: "colorequal",
        image: "mire1.cr2",
        xmp: include_str!("../cases/colorequal-no-filter.xmp"),
        disable: &["sigmoid"],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0089-blurs, which blurs through drawn masks; here it covers the whole image.
        // A spatial module, so it needs the full-resolution renders.
        name: "blurs",
        operation: "blurs",
        image: "mire1.cr2",
        xmp: include_str!("../cases/blurs.xmp"),
        disable: &["filmicrgb"],
        unmask_like: Some("channelmixerrgb"),
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0175-agx: every look control, hue restoration, Adobe RGB base primaries
        // with custom insets, rotations, outsets and unrotations, linear sections, target
        // black and white, gamma 2.3. The sidecar has two AgX entries; the second is
        // active. No later tone mapper to disable.
        name: "agx",
        operation: "agx",
        image: "mire1.cr2",
        xmp: include_str!("../cases/agx.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0174-agx-default: params version 4, which darktable migrates to its
        // scene-referred defaults.
        name: "agx-default",
        operation: "agx",
        image: "mire1.cr2",
        xmp: include_str!("../cases/agx-default.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0186-agx-sdr-legacy: the version 7 defaults on a raw with clipped
        // highlights. Tests 0187 to 0189 render identically in darktable (HLG
        // output, which the harness replaces with linear Rec2020, and params version 8,
        // which the reference release replaces with its defaults), so they are not cases.
        name: "agx-highlights",
        operation: "agx",
        image: "hlrecovery.arw",
        xmp: include_str!("../cases/agx-highlights.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0174-agx-default, edited: its AgX entry is replaced by version 7 params,
        // the scene-referred defaults of test 0186 with look lift 0.05, slope 0.95,
        // brightness 1.2, saturation 0.85, hue restoration 0, range -2 to +2.5 EV with
        // the pivot at mid-grey (which needs the toe's and the shoulder's fallback
        // curves), sRGB base primaries and "reverse all". It covers the paths no
        // integration test reaches.
        name: "agx-edges",
        operation: "agx",
        image: "mire1.cr2",
        xmp: include_str!("../cases/agx-edges.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    // Sigmoid (docs/ports/sigmoid.md, Test plan). No later tone mapper is on in these
    // sidecars, so none is disabled.
    Case {
        // Test 0097-sigmoid: params version 1, per channel, contrast 1.788, skew 0.36,
        // hue preservation 50.62%, the working profile's primaries unmodified.
        name: "sigmoid",
        operation: "sigmoid",
        image: "mire1.cr2",
        xmp: include_str!("../cases/sigmoid.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0151-sigmoid-no-hue-preservation: test 0097's settings with hue
        // preservation 0. The sidecar has two sigmoid entries; the second is active.
        name: "sigmoid-no-hue-preservation",
        operation: "sigmoid",
        image: "mire1.cr2",
        xmp: include_str!("../cases/sigmoid-no-hue-preservation.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0182-sigmoid-sdr-legacy: params version 3 at darktable's defaults, on a raw
        // with clipped highlights. Tests 0183 to 0185 render identically in the reference
        // release (PQ output, which the harness replaces with linear Rec2020, and params
        // version 4, which it replaces with its defaults), so they are not cases.
        name: "sigmoid-default",
        operation: "sigmoid",
        image: "hlrecovery.arw",
        xmp: include_str!("../cases/sigmoid-default.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0097, edited: its sigmoid entry is params version 3, RGB ratio, contrast
        // 1.35, skew -0.3, target white 95%, target black 0.8%, with primaries set (insets
        // 0.1, 0.1 and 0.15, rotations 2°, -1° and -3°, purity 0.3, sRGB base), which RGB
        // ratio ignores. No integration test covers RGB ratio.
        name: "sigmoid-rgb-ratio",
        operation: "sigmoid",
        image: "mire1.cr2",
        xmp: include_str!("../cases/sigmoid-rgb-ratio.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0097, edited: its sigmoid entry is params version 3, per channel, contrast
        // 1.6, skew -0.15, target black 0.05%, hue preservation 70%, sRGB base primaries
        // with insets 0.12, 0.08 and 0.2, rotations 2°, -1.5° and -3.5°, purity 0.4. No
        // integration test covers the primaries section.
        name: "sigmoid-primaries",
        operation: "sigmoid",
        image: "mire1.cr2",
        xmp: include_str!("../cases/sigmoid-primaries.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0060-rgbcurve-indep: independent channels, a monotone Hermite curve each
        // (9, 5 and 5 nodes), middle grey compensated. No later module to disable.
        name: "rgbcurve",
        operation: "rgbcurve",
        image: "mire1.cr2",
        xmp: include_str!("../cases/rgbcurve.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    // The other modes, each test 0060's sidecar with the rgbcurve entry's curve types,
    // mode, middle grey compensation and preserve colours edited and its nodes kept;
    // the linked modes use the red curve's nine nodes.
    Case {
        // Independent channels, uncompensated: a cubic spline for red, Catmull-Rom for
        // green, monotone Hermite for blue.
        name: "rgbcurve-independent",
        operation: "rgbcurve",
        image: "mire1.cr2",
        xmp: include_str!("../cases/rgbcurve-independent.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Linked, preserve colours by luminance, cubic spline, compensated.
        name: "rgbcurve-luminance",
        operation: "rgbcurve",
        image: "mire1.cr2",
        xmp: include_str!("../cases/rgbcurve-luminance.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Linked, no colour preservation (the curve on each channel), Catmull-Rom,
        // uncompensated.
        name: "rgbcurve-none",
        operation: "rgbcurve",
        image: "mire1.cr2",
        xmp: include_str!("../cases/rgbcurve-none.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Linked, preserve colours by max RGB, monotone Hermite, compensated.
        name: "rgbcurve-max",
        operation: "rgbcurve",
        image: "mire1.cr2",
        xmp: include_str!("../cases/rgbcurve-max.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Linked, preserve colours by average RGB, cubic spline, uncompensated.
        name: "rgbcurve-average",
        operation: "rgbcurve",
        image: "mire1.cr2",
        xmp: include_str!("../cases/rgbcurve-average.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Linked, preserve colours by sum RGB, Catmull-Rom, compensated.
        name: "rgbcurve-sum",
        operation: "rgbcurve",
        image: "mire1.cr2",
        xmp: include_str!("../cases/rgbcurve-sum.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Linked, preserve colours by norm RGB, monotone Hermite, uncompensated.
        name: "rgbcurve-norm",
        operation: "rgbcurve",
        image: "mire1.cr2",
        xmp: include_str!("../cases/rgbcurve-norm.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Linked, preserve colours by basic power, cubic spline, compensated.
        name: "rgbcurve-power",
        operation: "rgbcurve",
        image: "mire1.cr2",
        xmp: include_str!("../cases/rgbcurve-power.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0054-rgblevels-linked: linked, preserve colours by luminance, levels
        // 0.0116, 0.355 and 0.621 (power 1.34). No later module to disable.
        name: "rgblevels",
        operation: "rgblevels",
        image: "mire1.cr2",
        xmp: include_str!("../cases/rgblevels.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0055-rgblevels-indep: test 0054's levels on independent channels (powers
        // 1.34, 1.31 and 1.77). The sidecar has two rgblevels entries; the second is
        // active.
        name: "rgblevels-independent",
        operation: "rgblevels",
        image: "mire1.cr2",
        xmp: include_str!("../cases/rgblevels-independent.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    // The other ways of preserving colours, each test 0054's sidecar with the rgblevels
    // entry's preserve colours and red (linked) levels edited, green's and blue's kept:
    // powers below 1 and above, black at 0 and white at 1 among them. No black point is
    // a value the synthetic sweep holds, where the kink at black would fail
    // `gradient --synthetic` (docs/ports/rgblevels.md, Test plan).
    Case {
        // Linked, no colour preservation (the levels on each channel), levels 0.0165,
        // 0.35 and 0.8 (power 0.71).
        name: "rgblevels-none",
        operation: "rgblevels",
        image: "mire1.cr2",
        xmp: include_str!("../cases/rgblevels-none.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Linked, preserve colours by max RGB, levels 0.02, 0.45 and 0.95 (power 0.84).
        name: "rgblevels-max",
        operation: "rgblevels",
        image: "mire1.cr2",
        xmp: include_str!("../cases/rgblevels-max.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Linked, preserve colours by average RGB, levels 0, 0.35 and 0.9 (power 0.60).
        name: "rgblevels-average",
        operation: "rgblevels",
        image: "mire1.cr2",
        xmp: include_str!("../cases/rgblevels-average.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Linked, preserve colours by sum RGB, levels 0.05, 0.6 and 1 (power 1.44); a
        // sum often passes white.
        name: "rgblevels-sum",
        operation: "rgblevels",
        image: "mire1.cr2",
        xmp: include_str!("../cases/rgblevels-sum.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Linked, preserve colours by norm RGB, levels 0.008, 0.25 and 0.7 (power 0.50).
        name: "rgblevels-norm",
        operation: "rgblevels",
        image: "mire1.cr2",
        xmp: include_str!("../cases/rgblevels-norm.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Linked, preserve colours by basic power, levels 0.03, 0.55 and 0.85 (power
        // 1.85).
        name: "rgblevels-power",
        operation: "rgblevels",
        image: "mire1.cr2",
        xmp: include_str!("../cases/rgblevels-power.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0045-vignetting with dithering off, its only edit: a fixed ratio of 1.172,
        // shape 1.49, the centre at (-0.116, 0.13), brightness -0.968, saturation -0.82,
        // unbound. darktable's 8-bit dithering draws its noise in an order that depends
        // on its thread count, and the noise alone fails the max tolerance; dithering is
        // not ported (docs/ports/vignette.md, Dithering). No later module to disable.
        name: "vignette",
        operation: "vignette",
        image: "mire1.cr2",
        xmp: include_str!("../cases/vignette.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    // The other modes, each test 0045's sidecar with the vignette entry's params edited,
    // dithering off. No band edge or centre falls exactly on a pixel of the test image
    // or the synthetic sweep, no ratio is 1 and no brightness 0, the kinks a finite
    // difference would straddle (docs/ports/vignette.md, Test plan).
    Case {
        // The automatic ratio, shape 0.83, fall-off from 41.3% over 77.7%, brightness
        // -0.613, saturation -0.271, the centre at (0.237, -0.168).
        name: "vignette-autoratio",
        operation: "vignette",
        image: "mire1.cr2",
        xmp: include_str!("../cases/vignette-autoratio.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // A fixed ratio of 0.63 (narrower than tall), shape 0.37 (a rounded rectangle),
        // fall-off from 23.7% over 112.3%, brightness -0.57, saturation +0.29, the centre
        // at (0.071, 0.311).
        name: "vignette-narrow",
        operation: "vignette",
        image: "mire1.cr2",
        xmp: include_str!("../cases/vignette-narrow.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Shape 3.7 (a four-pointed star), a fixed ratio of 1.61, fall-off from 31.7% over
        // 83.6%, brightness -0.81, saturation -0.37, the centre at (-0.29, -0.077).
        name: "vignette-star",
        operation: "vignette",
        image: "mire1.cr2",
        xmp: include_str!("../cases/vignette-star.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Clipped (unbound off), a white vignette: brightness +0.413, saturation +0.372, a
        // fixed ratio of 0.87, shape 1.13, fall-off from 52.9% over 46.3%, the centre at
        // (-0.043, 0.197).
        name: "vignette-clip",
        operation: "vignette",
        image: "mire1.cr2",
        xmp: include_str!("../cases/vignette-clip.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // A white vignette, unclipped: brightness +0.37 (added to every channel),
        // saturation -0.43, a fixed ratio of 1.37, shape 1.71, fall-off from 47.3% over
        // 61.9%, the centre at (0.153, 0.087).
        name: "vignette-white",
        operation: "vignette",
        image: "mire1.cr2",
        xmp: include_str!("../cases/vignette-white.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    // Local contrast (docs/ports/bilat.md, Test plan).
    Case {
        // Test 0002-local-contrast: the local laplacian filter, darktable's default mode,
        // at its defaults (detail 125%, highlights and shadows 50%, mid-tone range 0.5),
        // after exposure +1 EV. No later module to disable; no mask.
        name: "bilat",
        operation: "bilat",
        image: "mire1.cr2",
        xmp: include_str!("../cases/bilat.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0002, edited: the bilat entry's mode is the bilateral grid, with the
        // coarseness (50) and contrast (20) darktable's sliders show when the mode is
        // chosen, and detail 200% (1.0, darktable showing it 100% more), so the module's
        // effect is well above the tolerance; no darktable test covers the mode.
        name: "bilat-bilateral",
        operation: "bilat",
        image: "mire1.cr2",
        xmp: include_str!("../cases/bilat-bilateral.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    // Filmic rgb (docs/ports/filmicrgb.md, Test plan): darktable's own tests, unchanged,
    // each run as a one-instance plan. No later module changes the image in them.
    Case {
        // Test 0014-filmic-rgb: params version 1, migrated to colour science v3 (2019)
        // and spline v3, the power norm, custom grey 9.225 %, reconstruction on but 6 EV
        // over the white, past every pixel.
        name: "filmicrgb",
        operation: "filmicrgb",
        image: "mire1.cr2",
        xmp: include_str!("../cases/filmicrgb.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0037-filmic-reconstruction: params version 4 (spline 2020, converted), v4
        // (2020), the power norm, soft toe and hard shoulder, highlight reconstruction at
        // +0.5 EV with one iteration and no noise, darktable's raw-stage highlight
        // reconstruction off.
        name: "filmicrgb-reconstruction",
        operation: "filmicrgb",
        image: "mire1.cr2",
        xmp: include_str!("../cases/filmicrgb-reconstruction.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0092-filmicrgb-v6: params version 5, v6 (2022) without chrominance
        // preservation, safe curves, reconstruction at +3 EV with one iteration and
        // gaussian noise; its custom module order runs colour correction before it.
        name: "filmicrgb-v6",
        operation: "filmicrgb",
        image: "mire1.cr2",
        xmp: include_str!("../cases/filmicrgb-v6.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0138-filmic-v7: v7 (2023), safe curves, balance 5.08 %, reconstruction at
        // +3 EV with one iteration and poissonian noise.
        name: "filmicrgb-v7",
        operation: "filmicrgb",
        image: "mire1.cr2",
        xmp: include_str!("../cases/filmicrgb-v7.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0139-filmicrgb-v3-nochroma, its second entry: v3 (2019) without
        // chrominance preservation, a safe toe and soft shoulder, custom grey.
        name: "filmicrgb-v3-nochroma",
        operation: "filmicrgb",
        image: "mire1.cr2",
        xmp: include_str!("../cases/filmicrgb-v3-nochroma.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0140-filmicrgb-v4-nochroma, its second entry: v4 (2020) without
        // chrominance preservation, a safe toe and hard shoulder, balance -10 %.
        name: "filmicrgb-v4-nochroma",
        operation: "filmicrgb",
        image: "mire1.cr2",
        xmp: include_str!("../cases/filmicrgb-v4-nochroma.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0141-filmicrgb-v5-nochroma, its third entry: v5 (2021) without chrominance
        // preservation, safe curves, reconstruction off.
        name: "filmicrgb-v5-nochroma",
        operation: "filmicrgb",
        image: "mire1.cr2",
        xmp: include_str!("../cases/filmicrgb-v5-nochroma.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0142-filmicrgb-v6-euclidean: v6 (2022) on the euclidean norm, safe curves,
        // reconstruction at +3 EV with one iteration.
        name: "filmicrgb-v6-euclidean",
        operation: "filmicrgb",
        image: "mire1.cr2",
        xmp: include_str!("../cases/filmicrgb-v6-euclidean.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0050-bloom (docs/ports/bloom.md, Test plan): size 40%, threshold 85%,
        // strength 70%, so a box radius of 104 at full resolution. No later module to
        // disable; no mask.
        name: "bloom",
        operation: "bloom",
        image: "mire1.cr2",
        xmp: include_str!("../cases/bloom.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0051-soften (docs/ports/soften.md, Test plan): size 33%, saturation 100%,
        // brightness 0.5 EV, mix 75%, so a box radius of 15 at full resolution, from
        // mire1's readout. No later module to disable; no mask.
        name: "soften",
        operation: "soften",
        image: "mire1.cr2",
        xmp: include_str!("../cases/soften.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0059-grain: lightness, the coarsest grain (6400 ISO), strength 100%,
        // mid-tones bias 100%. No later module to disable; no mask.
        name: "grain",
        operation: "grain",
        image: "mire1.cr2",
        xmp: include_str!("../cases/grain.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0059, edited: the grain entry's params are 100 ISO, near darktable's finest
        // grain, where the noise's positions reach furthest, strength 37.3%, mid-tones
        // bias 52.7%.
        name: "grain-fine",
        operation: "grain",
        image: "mire1.cr2",
        xmp: include_str!("../cases/grain-fine.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0059, edited: the hue channel, which darktable's process() ignores like
        // every channel, 2300 ISO, strength 52%, mid-tones bias 11.5%.
        name: "grain-channel",
        operation: "grain",
        image: "mire1.cr2",
        xmp: include_str!("../cases/grain-channel.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0059, edited: params version 1 (channel, scale, strength), which darktable
        // migrates with a mid-tones bias of 0: the RGB channel, 1150 ISO, strength 64.5%.
        name: "grain-v1",
        operation: "grain",
        image: "mire1.cr2",
        xmp: include_str!("../cases/grain-v1.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0186-agx-sdr-legacy's sidecar with a grain entry added after AgX, as
        // Apertrust runs grain on the tone mapper's output: another image, so another
        // offset and shape, 3200 ISO, strength 71.3%, mid-tones bias 83%.
        name: "grain-agx",
        operation: "grain",
        image: "hlrecovery.arw",
        xmp: include_str!("../cases/grain-agx.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    // Color calibration (docs/ports/channelmixerrgb.md, Test plan). A sidecar's
    // channelmixerrgb entries are one instance; the last before the history's end is
    // the one darktable renders.
    Case {
        // Test 0085-channelmixerrgb: CAT16, a custom illuminant (0.30981, 0.32762),
        // normalised mixer rows (-0.969, 0.476, 0), (0, -0.479, 0) and (0, 0, 1),
        // colourfulness (0.215, -0.953, -0.544), brightness (0.184, -0.305, 0.138),
        // gamut compression 1, clipping.
        name: "channelmixerrgb",
        operation: "channelmixerrgb",
        image: "mire1.cr2",
        xmp: include_str!("../cases/channelmixerrgb.xmp"),
        disable: &["filmicrgb"],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0126-CAT-linear-Bradford: test 0085's settings with linear Bradford. No
        // later module to disable.
        name: "channelmixerrgb-linear-bradford",
        operation: "channelmixerrgb",
        image: "mire1.cr2",
        xmp: include_str!("../cases/channelmixerrgb-linear-bradford.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0127-CAT-nonlinear-Bradford: the same with non-linear Bradford.
        name: "channelmixerrgb-bradford",
        operation: "channelmixerrgb",
        image: "mire1.cr2",
        xmp: include_str!("../cases/channelmixerrgb-bradford.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0128-CAT-adapt-XYZ: the same in XYZ.
        name: "channelmixerrgb-xyz",
        operation: "channelmixerrgb",
        image: "mire1.cr2",
        xmp: include_str!("../cases/channelmixerrgb-xyz.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0174-agx-default: darktable's scene-referred default chain, "as shot to
        // reference" with colorin's late correction in the input, CAT16, daylight at
        // 3927.06 K, which renders the Planckian model, below 4000 K.
        name: "channelmixerrgb-default",
        operation: "channelmixerrgb",
        image: "mire1.cr2",
        xmp: include_str!("../cases/channelmixerrgb-default.xmp"),
        disable: &["agx"],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0146-lens-metadata-xtransIV-modversion-7: the daylight model at 4422 K, on
        // an X-Trans raw.
        name: "channelmixerrgb-daylight",
        operation: "channelmixerrgb",
        image: "xtransIV.raf",
        xmp: include_str!("../cases/channelmixerrgb-daylight.xmp"),
        disable: &["filmicrgb"],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0095-highlight-opposed: the Planckian model at 4555 K on bright clipped
        // highlights, where the luminance-weighted gamut compression is strongest.
        name: "channelmixerrgb-highlights",
        operation: "channelmixerrgb",
        image: "hlrecovery.arw",
        xmp: include_str!("../cases/channelmixerrgb-highlights.xmp"),
        disable: &["filmicrgb"],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0085, edited: the active channelmixerrgb entry is darktable's "monochrome
        // | ILFORD HP5+" preset, grey weights (0.253, 0.260, 0.487) normalised, the
        // pipeline's illuminant, XYZ, no gamut compression, clipping: Black and white's
        // path.
        name: "channelmixerrgb-grey",
        operation: "channelmixerrgb",
        image: "mire1.cr2",
        xmp: include_str!("../cases/channelmixerrgb-grey.xmp"),
        disable: &["filmicrgb"],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0085, edited: the active entry with adaptation none, so the mixer,
        // colourfulness and brightness work in working RGB.
        name: "channelmixerrgb-bypass",
        operation: "channelmixerrgb",
        image: "mire1.cr2",
        xmp: include_str!("../cases/channelmixerrgb-bypass.xmp"),
        disable: &["filmicrgb"],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0085, edited: the active entry with clipping off and gamut compression 4.
        name: "channelmixerrgb-unclipped",
        operation: "channelmixerrgb",
        image: "mire1.cr2",
        xmp: include_str!("../cases/channelmixerrgb-unclipped.xmp"),
        disable: &["filmicrgb"],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0165-demosaic-markesteijn-vng, edited: the illuminant is the camera's,
        // with E's (1/3, 1/3) as the stored x and y, darktable's fallback, so that a
        // fallback on either side shows. "As shot to reference", on the X-T4, whose
        // matrix and as-shot multipliers rawler reads as darktable does.
        name: "channelmixerrgb-camera",
        operation: "channelmixerrgb",
        image: "mire1-xtrans.raf",
        xmp: include_str!("../cases/channelmixerrgb-camera.xmp"),
        disable: &["sigmoid"],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    // The tone equalizer, first its per-pixel path, "preserve details: no". darktable's
    // tests run it in camera RGB, before colorin, where Apertrust runs it too (decision
    // 22), and keep their own module order. The harness hands the operation darktable's
    // linear Rec2020 render, so each sidecar's colorin is edited to the linear Rec2020
    // input profile, an identity, as raw_stage.rs's is: both renders, and the
    // operation's input, are then camera RGB. The tolerance is the port's own, stricter
    // than darktable's (TONEEQUAL).
    Case {
        // Test 0131-toneeq-RGB-max: HSV value, mask exposure +0.13 EV, smoothing √2.
        // Three toneequal entries of one instance; the third renders.
        name: "toneequal-max",
        operation: "toneequal",
        image: "mire1.cr2",
        xmp: include_str!("../cases/toneequal-max.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: TONEEQUAL,
    },
    Case {
        // Test 0134-toneeq-geomean: RGB geometric mean, mask exposure +0.15 EV,
        // smoothing √2, test 0131's bands.
        name: "toneequal-geomean",
        operation: "toneequal",
        image: "mire1.cr2",
        xmp: include_str!("../cases/toneequal-geomean.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: TONEEQUAL,
    },
    Case {
        // Test 0131, edited: the rendering (third) toneequal entry's smoothing is 1.2
        // rather than √2, where darktable's f32 band solve and an f64 one agree within
        // 3e-5 EV, so the case checks the smoothing, not darktable's rounding.
        name: "toneequal-smoothing",
        operation: "toneequal",
        image: "mire1.cr2",
        xmp: include_str!("../cases/toneequal-smoothing.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: TONEEQUAL,
    },
    // The mask filters, EIGF and averaged EIGF, with the same colorin edit. The harness
    // runs the operation as a one-instance engine plan: its mask's whole-image passes,
    // then main (decision 15).
    Case {
        // Test 0080-toneequal-eigf: EIGF on the euclidean norm, darktable's default
        // filter and estimator, blending 5% (radius 98), feathering 1, contrast +0.79 EV,
        // mask exposure -0.51 EV. Two toneequal entries of one instance; the second
        // renders.
        name: "toneequal-eigf",
        operation: "toneequal",
        image: "mire1.cr2",
        xmp: include_str!("../cases/toneequal-eigf.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: TONEEQUAL,
    },
    Case {
        // Test 0129-toneeq-RGB-average: averaged EIGF on the RGB average, blending 3%
        // (radius 58), feathering 3, mask exposure +0.13 EV.
        name: "toneequal-average",
        operation: "toneequal",
        image: "mire1.cr2",
        xmp: include_str!("../cases/toneequal-average.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: TONEEQUAL,
    },
    Case {
        // Test 0133-toneeq-power: EIGF on the power norm, blending 8% (radius 157),
        // feathering 3, contrast +0.27 EV, mask exposure +0.15 EV, test 0131's bands.
        name: "toneequal-power",
        operation: "toneequal",
        image: "mire1.cr2",
        xmp: include_str!("../cases/toneequal-power.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: TONEEQUAL,
    },
    Case {
        // Test 0080, edited: the rendering (second) toneequal entry takes darktable's
        // "compress shadows/highlights | EIGF | strong" mask settings, blending 2%
        // (radius 38), feathering 20, five iterations, contrast 0 and mask exposure
        // -1.57 EV: EIGF's iterated mask at full resolution. Without quantisation, as the
        // preset: a quantised guide makes darktable's own result depend on its binary's
        // last bits wherever the mask sits on a quantisation step, which iterating
        // spreads (docs/ports/toneequal.md, Results).
        name: "toneequal-eigf-iterations",
        operation: "toneequal",
        image: "mire1.cr2",
        xmp: include_str!("../cases/toneequal-eigf-iterations.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: TONEEQUAL,
    },
    // Haze removal runs in camera RGB, after demosaic, in darktable's orders and in
    // Apertrust's, and does not commute with colorin's matrix, so each sidecar's colorin
    // is edited to the linear Rec2020 input profile, an identity, as the tone
    // equalizer's are: both renders, and the kernel's input, are then camera RGB.
    Case {
        // Test 0026-haze-removal: params version 1, strength and distance 0.2, which
        // darktable migrates to its compatibility mode and fixed windows. basecurve, a
        // display-referred curve after colorin, is off in both renders.
        name: "hazeremoval",
        operation: "hazeremoval",
        image: "mire1.cr2",
        xmp: include_str!("../cases/hazeremoval.xmp"),
        disable: &["basecurve"],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0166-haze-removal-v2: params version 2, strength 0.9, distance 0.8,
        // compatibility mode off. Its exposure (+0.7 EV) runs after haze removal and
        // does not commute with it, so it is off in both renders; and temperature's
        // preset, "as shot to reference" (4), is edited to "user modified" (2), the
        // same coefficients without colorin's late correction, a per-channel gain after
        // haze removal that does not commute with it either.
        name: "hazeremoval-v2",
        operation: "hazeremoval",
        image: "mire1.cr2",
        xmp: include_str!("../cases/hazeremoval-v2.xmp"),
        disable: &["exposure"],
        unmask_like: None,
        tolerance: Tolerance::DARKTABLE,
    },
    Case {
        // Test 0005-orientation-minus-90: rotate 90° anticlockwise, darktable's
        // `ORIENTATION_ROTATE_CCW_90_DEG` (6), after an auto entry. Nothing after
        // orientation but colorin, colorout and gamma, per pixel.
        name: "flip-ccw",
        operation: "flip",
        image: "mire1.cr2",
        xmp: include_str!("../cases/flip-ccw.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: FLIP,
    },
    Case {
        // Test 0006-orientation-plus-90: rotate 90° clockwise, darktable's
        // `ORIENTATION_ROTATE_CW_90_DEG` (5), after an auto entry.
        name: "flip-cw",
        operation: "flip",
        image: "mire1.cr2",
        xmp: include_str!("../cases/flip-cw.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: FLIP,
    },
    Case {
        // Test 0007-orientation-180: rotate 180° (3), the last of three entries, after
        // auto and a quarter turn.
        name: "flip-180",
        operation: "flip",
        image: "mire1.cr2",
        xmp: include_str!("../cases/flip-180.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: FLIP,
    },
    Case {
        // Test 0005-orientation-minus-90, edited: its quarter turn replaced by flip
        // horizontally (2), darktable's preset. darktable's tests mirror only through crop.
        name: "flip-horizontal",
        operation: "flip",
        image: "mire1.cr2",
        xmp: include_str!("../cases/flip-horizontal.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: FLIP,
    },
    Case {
        // Test 0005-orientation-minus-90, edited: its quarter turn replaced by flip
        // vertically (1), darktable's preset.
        name: "flip-vertical",
        operation: "flip",
        image: "mire1.cr2",
        xmp: include_str!("../cases/flip-vertical.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: FLIP,
    },
    Case {
        // Test 0005-orientation-minus-90, edited: its quarter turn replaced by transpose
        // (4), the axes swapped, which EXIF's orientation 5 gives.
        name: "flip-transpose",
        operation: "flip",
        image: "mire1.cr2",
        xmp: include_str!("../cases/flip-transpose.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: FLIP,
    },
    Case {
        // Test 0005-orientation-minus-90, edited: its quarter turn replaced by
        // transverse (7), both flips and the swap, which EXIF's orientation 7 gives.
        name: "flip-transverse",
        operation: "flip",
        image: "mire1.cr2",
        xmp: include_str!("../cases/flip-transverse.xmp"),
        disable: &[],
        unmask_like: None,
        tolerance: FLIP,
    },
];

/// The tone equalizer's tolerance, for smoothing up to 1.5: its maths is darktable's but
/// for the band fit in f64 and the curve evaluated rather than read from a table, which
/// together move the correction by at most about 1e-3 EV, 0.015 ΔE2000 on any colour.
/// The max allows three times that, the mean a fifth of the max
/// (docs/ports/toneequal.md, Test plan).
pub const TONEEQUAL: Tolerance = Tolerance {
    mean: 0.01,
    max: 0.05,
};

/// Orientation's tolerance: none. It moves whole pixels and computes nothing, and every
/// operation after it in its cases is per pixel, so its output is darktable's bit for bit
/// (docs/ports/flip.md, Test plan).
pub const FLIP: Tolerance = Tolerance {
    mean: 0.0,
    max: 0.0,
};

pub fn find(name: &str) -> Option<&'static Case> {
    CASES.iter().find(|case| case.name == name)
}
