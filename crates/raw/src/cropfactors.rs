// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2009-2026  darktable developers (ported from src/common/exif.cc)
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

//! The crop factor as darktable works it out as a photo loads, its `exif_crop`
//! (`_exif_decode_exif_data` in `src/common/exif.cc:1492-1610`), from the tags
//! [`crate::tags`] reads, each way tried only while the factor is still unknown, then
//! `dt_cropfactors` overriding it by model:
//!
//! 1. FocalLengthIn35mmFilm over the focal length;
//! 2. the sensor's diagonal from the focal plane's resolution and the pixel dimensions,
//!    snapped to 1 or 2 within 0.1;
//! 3. Olympus's FocalPlaneDiagonal, from its maker note's Equipment directory.

/// darktable's `dt_cropfactors` (`src/common/exif.cc:90-112`): the models whose factor
/// its arithmetic cannot find, by the EXIF Model as the file writes it.
const CROP_FACTORS: &[(&str, f32)] = &[
    ("Canon EOS 5D Mark IV", 1.0),
    ("Canon EOS 6D Mark II", 1.0),
    ("FinePix SL1000", 5.6),
    ("FinePix E550", 4.65),
    ("XF10", 1.5),
    ("FinePix S1", 5.6),
    ("FinePix HS10 HS11", 5.64),
];

/// The diagonal of a full-frame sensor, 36 × 24 mm, in mm.
fn full_frame() -> f32 {
    36.0_f32.hypot(24.0)
}

/// The focal plane's resolution as the Exif IFD records it: FocalPlaneXResolution,
/// FocalPlaneYResolution (0 where it is missing, as darktable reads it) and
/// FocalPlaneResolutionUnit (2 inches, 3 centimetres; 1 where it is missing).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FocalPlane {
    pub x: f32,
    pub y: f32,
    pub unit: u32,
}

/// The pixel dimensions darktable reads the focal plane's resolution against, each pair
/// a width and a height, 0 where a tag is missing, in darktable's order: the Exif IFD's
/// PixelXDimension and PixelYDimension; the first SubIFD's ImageWidth and ImageLength
/// where its NewSubfileType is 0, a DNG's raw; IFD0's ImageWidth and ImageLength.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct Dimensions {
    pub exif: [u32; 2],
    pub sub_image: Option<[u32; 2]>,
    pub image: [u32; 2],
}

/// What the crop factor is worked out from.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct Inputs<'a> {
    /// The focal length, in mm, 0 where none is recorded.
    pub focal_length: f32,
    /// FocalLengthIn35mmFilm, in mm, 0 where missing.
    pub focal_length_35mm: f32,
    pub focal_plane: Option<FocalPlane>,
    pub dimensions: Dimensions,
    /// Olympus's FocalPlaneDiagonal, a rational in mm.
    pub olympus_diagonal: Option<(u32, u32)>,
    /// The EXIF Model as the file writes it, trailing spaces trimmed, as darktable's
    /// `exif_model`.
    pub model: &'a str,
}

/// The crop factor darktable works out from `inputs`; 0 for none.
pub(crate) fn crop_factor(inputs: &Inputs<'_>) -> f32 {
    let mut crop = 0.0;
    if inputs.focal_length_35mm > 0.0 && inputs.focal_length > 0.0 {
        crop = inputs.focal_length_35mm / inputs.focal_length;
    }
    if crop == 0.0
        && let Some(plane) = inputs.focal_plane
    {
        crop = from_focal_plane(plane, inputs.dimensions);
    }
    if crop == 0.0
        && let Some((n, d)) = inputs.olympus_diagonal
        && n != 0
        && d != 0
    {
        crop = full_frame() / (f64::from(n) / f64::from(d)) as f32;
    }
    CROP_FACTORS
        .iter()
        .find(|(model, _)| *model == inputs.model)
        .map_or(crop, |(_, factor)| *factor)
}

/// The sensor's diagonal from the focal plane's resolution, in pixels per mm, and the
/// pixel dimensions, as a crop factor snapped to 1 or 2 within 0.1; 0 without both
/// dimensions.
fn from_focal_plane(plane: FocalPlane, dimensions: Dimensions) -> f32 {
    let per_mm = match plane.unit {
        2 => 25.4,
        3 => 10.0,
        _ => 1.0,
    };
    let (x, y) = (plane.x / per_mm, plane.y / per_mm);
    let [mut width, mut height] = dimensions.exif;
    if width == 0
        && let Some(sub_image) = dimensions.sub_image
    {
        [width, height] = sub_image;
    }
    if width == 0 {
        [width, height] = dimensions.image;
    }
    if width == 0 || height == 0 {
        return 0.0;
    }
    let diagonal = (width as f32 / x).hypot(height as f32 / y);
    let crop = full_frame() / diagonal;
    if (1.0 - crop).abs() < 0.1 {
        1.0
    } else if (2.0 - crop).abs() < 0.1 {
        2.0
    } else {
        crop
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const APS_C: Dimensions = Dimensions {
        exif: [6000, 4000],
        sub_image: None,
        image: [160, 120],
    };

    #[test]
    fn the_35_mm_equivalent_comes_first() {
        let inputs = Inputs {
            focal_length: 18.3,
            focal_length_35mm: 28.0,
            focal_plane: Some(FocalPlane {
                x: 1.0,
                y: 1.0,
                unit: 2,
            }),
            dimensions: APS_C,
            ..Inputs::default()
        };
        assert!((crop_factor(&inputs) - 28.0 / 18.3).abs() < 1e-6);
        // Without a focal length, the 35 mm equivalent says nothing.
        let unknown = Inputs {
            focal_length: 0.0,
            ..inputs
        };
        assert_ne!(crop_factor(&unknown), 28.0 / 18.3);
    }

    #[test]
    fn the_focal_plane_reads_in_each_unit() {
        // A 23.5 × 15.6 mm sensor of 6000 × 4000 pixels: 1.53.
        let (x, y) = (6000.0 / 23.5, 4000.0 / 15.6);
        let factor = |unit: u32, per_mm: f32| {
            crop_factor(&Inputs {
                focal_plane: Some(FocalPlane {
                    x: x * per_mm,
                    y: y * per_mm,
                    unit,
                }),
                dimensions: APS_C,
                ..Inputs::default()
            })
        };
        for (unit, per_mm) in [(1, 1.0), (2, 25.4), (3, 10.0), (7, 1.0)] {
            let crop = factor(unit, per_mm);
            assert!((crop - 1.53).abs() < 0.01, "unit {unit}: {crop}");
        }
    }

    #[test]
    fn the_dimensions_follow_darktable_s_order() {
        let plane = Some(FocalPlane {
            x: 6000.0 / 36.0,
            y: 4000.0 / 24.0,
            unit: 1,
        });
        let with = |dimensions| {
            crop_factor(&Inputs {
                focal_plane: plane,
                dimensions,
                ..Inputs::default()
            })
        };
        // The Exif IFD's, a full-frame sensor.
        assert_eq!(with(APS_C), 1.0);
        // The first SubIFD's where the Exif IFD has none, then IFD0's.
        let sub = Dimensions {
            exif: [0, 0],
            sub_image: Some([3000, 2000]),
            image: [6000, 4000],
        };
        assert_eq!(with(sub), 2.0);
        let image = Dimensions {
            sub_image: None,
            ..sub
        };
        assert_eq!(with(image), 1.0);
        // Neither: none.
        assert_eq!(with(Dimensions::default()), 0.0);
    }

    #[test]
    fn a_factor_near_1_or_2_snaps_to_it() {
        let crop = |diagonal_mm: f32| {
            let side = 1000.0;
            let mm = diagonal_mm / 2.0_f32.sqrt();
            crop_factor(&Inputs {
                focal_plane: Some(FocalPlane {
                    x: side / mm,
                    y: side / mm,
                    unit: 1,
                }),
                dimensions: Dimensions {
                    exif: [side as u32, side as u32],
                    ..Dimensions::default()
                },
                ..Inputs::default()
            })
        };
        assert_eq!(crop(full_frame() / 1.05), 1.0);
        assert_eq!(crop(full_frame() / 1.95), 2.0);
        let free = crop(full_frame() / 1.5);
        assert!((free - 1.5).abs() < 1e-4, "{free}");
    }

    #[test]
    fn olympus_s_diagonal_comes_last() {
        let inputs = Inputs {
            olympus_diagonal: Some((21633, 1000)),
            ..Inputs::default()
        };
        assert!((crop_factor(&inputs) - full_frame() / 21.633).abs() < 1e-4);
        let zero = Inputs {
            olympus_diagonal: Some((21633, 0)),
            ..Inputs::default()
        };
        assert_eq!(crop_factor(&zero), 0.0);
    }

    #[test]
    fn a_model_darktable_lists_overrides_what_is_worked_out() {
        let inputs = Inputs {
            focal_length: 50.0,
            focal_length_35mm: 75.0,
            model: "Canon EOS 5D Mark IV",
            ..Inputs::default()
        };
        assert_eq!(crop_factor(&inputs), 1.0);
        assert_eq!(
            crop_factor(&Inputs {
                model: "XF10",
                ..Inputs::default()
            }),
            1.5
        );
        assert_eq!(crop_factor(&Inputs::default()), 0.0);
    }
}
