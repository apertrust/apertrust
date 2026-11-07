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

//! The exposure values at the end of the caption under the photo (`docs/gui.md`, The
//! caption under the photo): the shutter, the aperture, the focal length and the ISO of the
//! shot, as the session reports them in its `Photo`, read from the raw's EXIF as it decodes
//! the photo (the app reads no raw): this only writes them in the language's words and
//! numbers.

use apertrust_engine::session::Photo;
use apertrust_ui::Translator;
use apertrust_ui::i18n::Args;

/// The shooting data the caption shows; what the photo does not record is `None`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Exposure {
    /// The exposure time, in seconds.
    pub shutter: Option<f32>,
    /// The f-number.
    pub aperture: Option<f32>,
    /// The focal length, in mm.
    pub focal_length: Option<f32>,
    /// The ISO, from the photo's image context.
    pub iso: Option<f32>,
}

/// A value the photo records: more than 0, and a number.
fn recorded(value: Option<f32>) -> Option<f32> {
    value.filter(|v| v.is_finite() && *v > 0.0)
}

impl Exposure {
    /// The shooting data `photo` carries; the ISO is its image context's, 0 where the file
    /// records none.
    pub fn of(photo: &Photo) -> Exposure {
        Exposure {
            shutter: recorded(photo.exposure_time),
            aperture: recorded(photo.aperture),
            focal_length: recorded(photo.focal_length),
            iso: recorded(photo.image.camera.as_ref().map(|camera| camera.iso)),
        }
    }

    /// The values the photo records, written in `words`' language in the order they drop
    /// from the caption's end: the shutter, the aperture, the focal length, the ISO.
    pub fn parts(&self, words: &Translator) -> Vec<String> {
        let say = |id: &str, value: String| {
            let mut args = Args::new();
            args.set("value", value);
            words.text_with(id, Some(&args))
        };
        let number = |value: f32, decimals| words.format_number(f64::from(value), decimals, false);
        [
            self.shutter
                .map(|seconds| say("photo-shutter", shutter(seconds, words))),
            self.aperture
                .map(|f_number| say("photo-aperture", number(f_number, 1))),
            self.focal_length
                .map(|mm| say("photo-focal-length", number(mm, 0))),
            self.iso.map(|iso| say("photo-iso", number(iso, 0))),
        ]
        .into_iter()
        .flatten()
        .collect()
    }
}

/// An exposure time by darktable's rule (`dt_util_format_exposure`), with `s` for its ″: a
/// fraction of a second below 0.29 s or where the fraction is whole, else in seconds.
fn shutter(seconds: f32, words: &Translator) -> String {
    let number = |value: f32, decimals| words.format_number(f64::from(value), decimals, false);
    if seconds >= 1.0 {
        if seconds.round() == seconds {
            number(seconds, 0)
        } else {
            number(seconds, 1)
        }
    } else if seconds < 0.29 || (1.0 / seconds).round() == 1.0 / seconds {
        format!("1/{}", number(1.0 / seconds, 0))
    } else if 10.0 * (10.0 / seconds).round() == (100.0 / seconds).round() {
        format!("1/{}", number(1.0 / seconds, 1))
    } else {
        number(seconds, 1)
    }
}

#[cfg(test)]
mod tests {
    use apertrust_ui::Locale;

    use super::*;

    /// A photo with the shooting data a decode read.
    fn photo(
        exposure_time: Option<f32>,
        aperture: Option<f32>,
        focal_length: Option<f32>,
        iso: f32,
    ) -> Photo {
        let mut image = apertrust_modules::testing::image();
        if let Some(camera) = &mut image.camera {
            camera.iso = iso;
        }
        Photo {
            path: std::path::PathBuf::from("IMG_0001.ARW"),
            open: 1,
            camera: String::new(),
            width: 84,
            height: 62,
            decode_ms: 0.0,
            upload_ms: 0.0,
            view_only: None,
            exposure_time,
            aperture,
            focal_length,
            image: Box::new(image),
        }
    }

    #[test]
    fn the_values_are_written_in_the_language_s_numbers_in_the_order_they_drop() {
        let shot = Exposure::of(&photo(Some(1.0 / 1250.0), Some(6.3), Some(35.0), 200.0));
        let en = Translator::new(Locale::EnGb);
        assert_eq!(
            shot.parts(&en),
            ["1/1250 s", "f/6.3", "35 mm", "ISO 200"].map(str::to_owned)
        );
        let uk = Translator::new(Locale::Uk);
        assert_eq!(
            shot.parts(&uk),
            ["1/1250 с", "f/6,3", "35 мм", "ISO 200"].map(str::to_owned)
        );
        let pl = Translator::new(Locale::Pl);
        assert_eq!(shot.parts(&pl)[1], "f/6,3");
    }

    #[test]
    fn what_the_photo_does_not_record_is_left_out() {
        let en = Translator::new(Locale::EnGb);
        assert!(Exposure::default().parts(&en).is_empty());
        // A time of 0, a NaN and an infinity are not recorded values; the ISO of 0, which
        // the image context holds for none, neither.
        assert_eq!(recorded(Some(0.0)), None);
        assert_eq!(recorded(Some(f32::NAN)), None);
        assert_eq!(recorded(Some(f32::INFINITY)), None);
        let shot = Exposure::of(&photo(None, None, Some(50.0), 0.0));
        assert_eq!(shot.parts(&en), ["50 mm".to_owned()]);
    }

    /// darktable's `dt_util_format_exposure`.
    #[test]
    fn a_shutter_is_written_as_darktable_writes_it() {
        let en = Translator::new(Locale::EnGb);
        let text = |seconds| shutter(seconds, &en);
        assert_eq!(text(1.0 / 250.0), "1/250");
        assert_eq!(text(0.5), "1/2");
        assert_eq!(text(2.0), "2");
        assert_eq!(text(2.5), "2.5");
        assert_eq!(text(30.0), "30");
        // Between 0.29 s and 1 s, where one over it is not whole: a tenth of a second or
        // a fraction with a decimal.
        assert_eq!(text(0.4), "1/2.5");
        assert_eq!(text(0.7), "0.7");
        let uk = Translator::new(Locale::Uk);
        assert_eq!(shutter(2.5, &uk), "2,5");
    }
}
