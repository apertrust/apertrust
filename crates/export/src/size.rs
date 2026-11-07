// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2009-2026  darktable developers (ported from src/imageio/imageio.c, src/imageio/imageio_module.c, src/libs/export.c, src/control/jobs/control_jobs.c and src/cli/main.c)
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

//! darktable's export size (decision 56; `docs/ports/finalscale.md`): what
//! `dt_imageio_export_with_flags` makes of the export's largest width and height or its
//! scale, Allow upscaling and High quality resampling, as darktable's export module
//! (`src/libs/export.c`, `src/control/jobs/control_jobs.c`) and darktable-cli
//! (`src/cli/main.c`) hand them to it: in double where darktable computes in double,
//! in float where in float, so the size is darktable's to the pixel.
//!
//! Not mirrored (decision 56): darktable's largest scale shrinks with the computer's
//! memory, and its export module takes the format's largest side as a limit left at 0.

use crate::{Error, Format};

/// darktable's `MAX_ASPECT_RATIO`: with Allow upscaling, a side left at 0 becomes this
/// many times the other (`_export_with_current_settings`).
const MAX_ASPECT_RATIO: u64 = 100;

/// The largest scale with Allow upscaling: darktable's `fmin(100, …)` without its memory
/// term (decision 56).
const LARGEST_SCALE: f64 = 100.0;

/// darktable's `INCH_TO_CM`.
const INCH_TO_CM: f32 = 2.54;

/// The size an export asks for, as darktable's export settings hold it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Request {
    /// The largest width and height in pixels, 0 for no limit: darktable's `width` and
    /// `height`, which hold pixels whatever unit the dialog shows them in
    /// ([`cm_to_pixels`], [`inches_to_pixels`]).
    pub width: u32,
    pub height: u32,
    /// By scale, darktable's scale text (`resizing_factor`), read by [`scale_factor`];
    /// `None` in the other modes, where the largest size limits instead.
    pub scale: Option<String>,
    /// Allow upscaling.
    pub upscale: bool,
    /// High quality resampling: every operation before output colour at full
    /// resolution, and the resample just before it.
    pub high_quality: bool,
    /// Whether the export module's rule applies, as in the dialog: with Allow upscaling,
    /// a side left at 0 becomes 100 times the other (`_export_with_current_settings`).
    /// darktable-cli has no such rule, `false`: a side at 0 then takes the photo's own,
    /// so the scale stays at most 1 along it.
    pub dialog: bool,
}

/// The size darktable exports at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    /// The output, `floor(scale × W)` by `floor(scale × H)`.
    pub width: u32,
    pub height: u32,
    /// darktable's scale, in double; the operations see it in float, as darktable's
    /// region of interest holds it.
    pub scale: f64,
    /// Whether the resample runs at darktable's finalscale place, before output colour,
    /// rather than after demosaic: darktable's `hq_process`, `high_quality || scale > 1`.
    pub finalscale: bool,
}

impl Size {
    /// Whether the export differs from the photo at full size: any scale but 1. At a
    /// scale of 1 in float but not in double, such as 0.99999999, the output is a pixel
    /// short of the image and darktable's resample copies its part (`copymode`), which
    /// the engine mirrors.
    pub fn resamples(&self) -> bool {
        self.scale != 1.0
    }
}

/// The size of an export of an image of `extent`, W × H at full resolution after its
/// geometry (darktable's `processed_width` and `processed_height`), to `format`, as
/// darktable computes it: an error, before any rendering, where a side would be larger
/// than the format's largest or smaller than a pixel (D8).
pub fn size(request: &Request, format: &Format, extent: [u32; 2]) -> Result<Size, Error> {
    let largest = format.largest();
    let by_scale = request.scale.as_deref().map(scale_factor);
    let [mut max_width, mut max_height] = [request.width, request.height].map(u64::from);
    // The export module, before the job: with upscaling, a side at 0 becomes 100 times
    // the other.
    if request.upscale && request.dialog {
        if max_width == 0 && max_height != 0 {
            max_width = max_height * MAX_ASPECT_RATIO;
        } else if max_height == 0 && max_width != 0 {
            max_height = max_width * MAX_ASPECT_RATIO;
        }
    }
    // darktable-cli: the format's largest side caps a limit that is set; 0 stays none.
    let [max_width, max_height] =
        [max_width, max_height].map(|side| side.min(u64::from(largest)) as u32);

    // `dt_imageio_export_with_flags`.
    let [processed_width, processed_height] = extent;
    let (mut width, mut height) = (max_width, max_height);
    if width == 0 && height == 0 {
        (width, height) = (processed_width, processed_height);
    }
    let doscale = request.upscale && (width > 0 || height > 0 || by_scale.is_some());
    let max_scale = if doscale { LARGEST_SCALE } else { 1.0 };
    // darktable computes the scale once more after its point back-transform, which
    // succeeds, with a side still at 0 taking the image's: only that scale counts.
    if width == 0 {
        width = processed_width;
    }
    if height == 0 {
        height = processed_height;
    }
    let mut scale = pipe_scale(extent, [width, height], max_scale);
    if let Some(factor) = by_scale {
        scale = factor.min(max_scale);
    }
    let output = [processed_width, processed_height].map(|side| (scale * f64::from(side)).floor());
    if output.iter().any(|&side| side < 1.0) {
        return Err(Error::Empty {
            width: output[0].max(0.0) as u32,
            height: output[1].max(0.0) as u32,
        });
    }
    if output.iter().any(|&side| side > f64::from(largest)) {
        return Err(Error::TooLarge {
            format: format.name(),
            largest,
            width: output[0].min(f64::from(u32::MAX)) as u32,
            height: output[1].min(f64::from(u32::MAX)) as u32,
        });
    }
    Ok(Size {
        width: output[0] as u32,
        height: output[1] as u32,
        scale,
        finalscale: request.high_quality || scale > 1.0,
    })
}

/// darktable's `_get_pipescale`: the scale that fits `extent` within `limit`, a side of
/// 0 setting no limit, and at most `max_scale`, in double.
fn pipe_scale(extent: [u32; 2], limit: [u32; 2], max_scale: f64) -> f64 {
    let along = |axis: usize| {
        if limit[axis] > 0 {
            (f64::from(limit[axis]) / f64::from(extent[axis])).min(max_scale)
        } else {
            max_scale
        }
    };
    along(0).min(along(1))
}

/// The factor darktable reads from its scale text, as
/// `dt_imageio_resizing_factor_get_and_parsing` reads it: `.` and `,` are both the
/// decimal point; `a/b` is a / b, `/b` is 1 / b and `a` is a, each number read as C's
/// `atof` reads it; a numerator or denominator of 0 is 1, so empty text is 1.
pub fn scale_factor(text: &str) -> f64 {
    let text = text.replace(',', ".");
    let (mut num, mut denum) = match text.split_once('/') {
        None => (atof(&text), 1.0),
        Some(("", denum)) => (1.0, atof(denum)),
        Some((_, denum)) => (atof(&text), atof(denum)),
    };
    if num == 0.0 {
        num = 1.0;
    }
    if denum == 0.0 {
        denum = 1.0;
    }
    num / denum
}

/// C's `atof`: the longest decimal number at the start of `text`, after white space,
/// with a sign, a fraction and an exponent, or 0 when there is none. strtod's
/// hexadecimal, infinity and NaN forms read as no number (`docs/ports/finalscale.md`).
fn atof(text: &str) -> f64 {
    let text = text.trim_start_matches([' ', '\t', '\n', '\u{b}', '\u{c}', '\r']);
    let bytes = text.as_bytes();
    let digits = |from: usize| {
        bytes[from..]
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .count()
    };
    let mut end = usize::from(matches!(bytes.first(), Some(b'+' | b'-')));
    let whole = digits(end);
    end += whole;
    let mut fraction = 0;
    if bytes.get(end) == Some(&b'.') {
        fraction = digits(end + 1);
        if whole + fraction > 0 {
            end += 1 + fraction;
        }
    }
    if whole + fraction == 0 {
        return 0.0;
    }
    if matches!(bytes.get(end), Some(b'e' | b'E')) {
        let sign = usize::from(matches!(bytes.get(end + 1), Some(b'+' | b'-')));
        let exponent = digits(end + 1 + sign);
        if exponent > 0 {
            end += 1 + sign + exponent;
        }
    }
    text[..end].parse().unwrap_or(0.0)
}

/// darktable's `cm2pixels`: a length in centimetres at `dpi` as whole pixels, rounded
/// up in float.
pub fn cm_to_pixels(cm: f32, dpi: u32) -> u32 {
    ((cm * dpi as f32) / INCH_TO_CM).ceil() as u32
}

/// darktable's `inch2pixels`.
pub fn inches_to_pixels(inches: f32, dpi: u32) -> u32 {
    (inches * dpi as f32).ceil() as u32
}

/// darktable's `pixels2cm`, which the dialog shows at two decimals.
pub fn pixels_to_cm(pixels: u32, dpi: u32) -> f32 {
    (pixels as f32 * INCH_TO_CM) / dpi as f32
}

/// darktable's `pixels2inch`.
pub fn pixels_to_inches(pixels: u32, dpi: u32) -> f32 {
    pixels as f32 / dpi as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{jpeg, png};

    const JPEG: Format = Format::Jpeg(jpeg::Options {
        quality: 95,
        subsample: jpeg::Subsample::Auto,
        dpi: 300,
    });
    const PNG: Format = Format::Png(png::Options {
        bit_depth: png::BitDepth::Eight,
        compression: 5,
    });

    fn limit(width: u32, height: u32) -> Request {
        Request {
            width,
            height,
            ..Request::default()
        }
    }

    fn by_scale(text: &str) -> Request {
        Request {
            scale: Some(text.to_owned()),
            ..Request::default()
        }
    }

    fn upscaled(request: Request) -> Request {
        Request {
            upscale: true,
            ..request
        }
    }

    /// As the dialog asks: the export module's rule for a side at 0 applies.
    fn in_dialog(request: Request) -> Request {
        Request {
            dialog: true,
            ..request
        }
    }

    fn sized(request: &Request, extent: [u32; 2]) -> (u32, u32) {
        let size = size(request, &JPEG, extent).unwrap();
        (size.width, size.height)
    }

    #[test]
    fn a_largest_size_fits_the_image_in_it_by_one_scale() {
        // The 7R V's 9504 × 6336 at 2048 × 2048: the width limits.
        let size = size(&limit(2048, 2048), &JPEG, [9504, 6336]).unwrap();
        assert_eq!((size.width, size.height), (2048, 1365));
        assert_eq!(size.scale, 2048.0 / 9504.0);
        assert!(!size.finalscale && size.resamples());
        // The height limits.
        assert_eq!(sized(&limit(4000, 1000), [6000, 4000]), (1500, 1000));
        // One side alone, the other at 0, limits.
        assert_eq!(sized(&limit(0, 2000), [6000, 4000]), (3000, 2000));
        assert_eq!(sized(&limit(1500, 0), [6000, 4000]), (1500, 1000));
    }

    #[test]
    fn the_output_is_the_scale_times_the_image_rounded_down_in_double() {
        // 1024 / 1122 × 1122 is 1023.9999999999999 in double: darktable exports 1023.
        let size = size(&limit(1024, 1024), &JPEG, [1122, 800]).unwrap();
        assert_eq!((size.width, size.height), (1023, 730));
        assert_eq!(size.scale, 1024.0 / 1122.0);
    }

    #[test]
    fn no_limit_or_a_larger_one_exports_at_full_size_without_upscaling() {
        for request in [
            limit(0, 0),
            limit(8000, 8000),
            limit(8000, 0),
            by_scale("2"),
            upscaled(limit(0, 0)),
        ] {
            let size = size(&request, &JPEG, [6000, 4000]).unwrap();
            assert_eq!((size.width, size.height, size.scale), (6000, 4000, 1.0));
            assert!(!size.resamples(), "{request:?}");
        }
    }

    #[test]
    fn upscaling_grows_the_image_to_its_limit_at_finalscale_s_place() {
        let size = size(&upscaled(limit(8000, 8000)), &JPEG, [6000, 4000]).unwrap();
        assert_eq!((size.width, size.height), (8000, 5333));
        assert_eq!(size.scale, 8000.0 / 6000.0);
        assert!(size.finalscale, "above 1, as darktable's hq_process");
    }

    #[test]
    fn upscaling_with_a_side_at_0_in_the_dialog_takes_100_times_the_other_capped_by_the_format() {
        // Height 6000 alone: the width 600000, capped at JPEG's 65535, so the height
        // limits.
        assert_eq!(
            sized(&in_dialog(upscaled(limit(0, 6000))), [6000, 4000]),
            (9000, 6000)
        );
        assert_eq!(
            sized(&in_dialog(upscaled(limit(9000, 0))), [6000, 4000]),
            (9000, 6000)
        );
        // Without upscaling the side at 0 is the image's, which a scale above 1 never
        // passes.
        assert_eq!(
            sized(&in_dialog(limit(0, 6000)), [6000, 4000]),
            (6000, 4000)
        );
    }

    #[test]
    fn upscaling_with_a_side_at_0_from_the_cli_never_upscales() {
        // darktable-cli has no rule for a side at 0: it takes the photo's own, so the
        // scale along it is at most 1, and so is the export's.
        let size = size(&upscaled(limit(0, 6000)), &JPEG, [6000, 4000]).unwrap();
        assert_eq!((size.width, size.height, size.scale), (6000, 4000, 1.0));
        assert!(!size.resamples());
        assert_eq!(sized(&upscaled(limit(9000, 0)), [6000, 4000]), (6000, 4000));
        // With both sides set it upscales as the dialog does.
        assert_eq!(
            sized(&upscaled(limit(9000, 9000)), [6000, 4000]),
            (9000, 6000)
        );
    }

    #[test]
    fn a_scale_of_1_in_float_but_not_in_double_cuts_a_pixel_off() {
        // 0.99999999 is 1 in float, so darktable's resample copies (`copymode`), but
        // floor(0.99999999 × 6000) is 5999 in double: its output is the image's first
        // 5999 × 3999 pixels.
        let size = size(&by_scale("0.99999999"), &JPEG, [6000, 4000]).unwrap();
        assert_eq!((size.width, size.height), (5999, 3999));
        assert_eq!(size.scale as f32, 1.0);
        assert!(size.resamples() && !size.finalscale);
    }

    #[test]
    fn the_format_s_largest_side_caps_a_limit_and_refuses_a_larger_output() {
        // A limit over 65535 fits the image into 65535 instead.
        let wide = [70_000, 1000];
        assert_eq!(sized(&limit(80_000, 0), wide), (65535, 936));
        // At full size the 70000 pixels do not fit a JPEG, and do a PNG.
        assert!(matches!(
            size(&limit(0, 0), &JPEG, wide),
            Err(Error::TooLarge {
                format: "JPEG",
                largest: 65535,
                width: 70_000,
                height: 1000
            })
        ));
        let png = size(&limit(0, 0), &PNG, wide).unwrap();
        assert_eq!((png.width, png.height), (70_000, 1000));
        // Nor does an upscale by a factor past it.
        assert!(size(&upscaled(by_scale("20")), &JPEG, [6000, 4000]).is_err());
    }

    #[test]
    fn an_output_under_a_pixel_is_refused() {
        assert!(matches!(
            size(&limit(1, 1), &JPEG, [6000, 4000]),
            Err(Error::Empty {
                width: 1,
                height: 0
            })
        ));
        assert!(size(&by_scale("-2"), &JPEG, [6000, 4000]).is_err());
    }

    #[test]
    fn a_scale_is_at_most_1_without_upscaling_and_100_with_it() {
        assert_eq!(sized(&by_scale("0.5"), [6000, 4000]), (3000, 2000));
        assert_eq!(sized(&by_scale("1/3"), [6000, 4000]), (2000, 1333));
        assert_eq!(sized(&by_scale("/4"), [6000, 4000]), (1500, 1000));
        assert_eq!(sized(&by_scale("2"), [6000, 4000]), (6000, 4000));
        let doubled = size(&upscaled(by_scale("2")), &PNG, [6000, 4000]).unwrap();
        assert_eq!((doubled.width, doubled.height), (12_000, 8000));
        assert!(doubled.finalscale);
        let largest = size(&upscaled(by_scale("1000")), &PNG, [60, 40]).unwrap();
        assert_eq!(
            (largest.width, largest.height, largest.scale),
            (6000, 4000, 100.0)
        );
        // By scale, the largest size is not a limit.
        let request = Request {
            width: 100,
            height: 100,
            ..by_scale("0.5")
        };
        assert_eq!(sized(&request, [6000, 4000]), (3000, 2000));
    }

    #[test]
    fn the_scale_text_reads_as_darktable_reads_it() {
        for (text, factor) in [
            ("1", 1.0),
            ("0.5", 0.5),
            ("0,5", 0.5),
            (".25", 0.25),
            ("1/2", 0.5),
            ("2/3", 2.0 / 3.0),
            ("/4", 0.25),
            ("1,5/3", 0.5),
            ("3/0", 3.0),
            ("0", 1.0),
            ("", 1.0),
            ("/", 1.0),
            ("abc", 1.0),
            (" 2", 2.0),
            ("2x", 2.0),
            ("1.5.5", 1.5),
            ("1e1", 10.0),
            ("1e", 1.0),
            ("-2", -2.0),
            ("1/2/4", 0.5),
        ] {
            assert_eq!(scale_factor(text), factor, "{text:?}");
        }
    }

    #[test]
    fn print_sizes_become_pixels_rounded_up_in_float() {
        // 10 cm at 300 dpi is 1181.10 pixels, so 1182.
        assert_eq!(cm_to_pixels(10.0, 300), 1182);
        assert_eq!(inches_to_pixels(8.5, 300), 2550);
        assert_eq!(inches_to_pixels(8.501, 300), 2551);
        assert_eq!(cm_to_pixels(0.0, 300), 0);
        assert_eq!(format!("{:.2}", pixels_to_cm(1182, 300)), "10.01");
        assert_eq!(format!("{:.2}", pixels_to_inches(2550, 300)), "8.50");
    }

    #[test]
    fn high_quality_resamples_at_finalscale_s_place() {
        let request = Request {
            high_quality: true,
            ..limit(2048, 2048)
        };
        let size = size(&request, &JPEG, [6000, 4000]).unwrap();
        assert_eq!((size.width, size.height), (2048, 1365));
        assert!(size.finalscale);
    }
}
