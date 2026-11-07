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

//! The camera's identity as darktable has it (decision 24): the camera an alias is
//! another name for, and the ISO by darktable's rule.
//!
//! rawler's loader copies a camera once for each of its aliases and gives each copy the
//! alias's clean name, so a decoded alias no longer says which camera it is. rawler's
//! camera files, which it embeds as `CAMERAS_TOML`, still do.

use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Instant;

use rawler::exif::Exif;

/// The clean model of the camera rawler matched by `make` and `model`, the names the
/// file gives: for an alias, the camera it is another name for. `None` for a camera
/// rawler's data does not know, such as a DNG's from another maker's software.
pub(crate) fn main_model(make: &str, model: &str) -> Option<&'static str> {
    static MODELS: OnceLock<HashMap<(String, String), String>> = OnceLock::new();
    MODELS
        .get_or_init(|| {
            let start = Instant::now();
            let models = main_models(rawler::decoders::CAMERAS_TOML);
            tracing::debug!(
                target: "perf",
                "read rawler's {} camera names in {:.1} ms",
                models.len(),
                start.elapsed().as_secs_f64() * 1e3
            );
            models
        })
        .get(&(make.trim_end().to_owned(), model.trim_end().to_owned()))
        .map(String::as_str)
}

/// Every make and model rawler's camera files match a file by, a camera's own and its
/// aliases', with the camera's clean model. A name two cameras give is the later one's,
/// as in rawler's loader. Empty if the files do not parse, which rawler's loader, parsing
/// them before any file decodes, would not survive.
fn main_models(cameras: &str) -> HashMap<(String, String), String> {
    let cameras = match cameras.parse::<toml::Table>() {
        Ok(cameras) => cameras,
        Err(error) => {
            tracing::warn!(
                target: "raw",
                "rawler's camera data: {}, aliases not resolved",
                error.message()
            );
            return HashMap::new();
        }
    };
    let list = cameras.get("cameras").and_then(toml::Value::as_array);
    let mut models = HashMap::new();
    for camera in list.into_iter().flatten().filter_map(toml::Value::as_table) {
        let text = |key: &str| camera.get(key).and_then(toml::Value::as_str);
        let (Some(make), Some(model), Some(clean_model)) =
            (text("make"), text("model"), text("clean_model"))
        else {
            continue;
        };
        // Each alias is the name its files give and its clean name.
        let aliases = camera
            .get("model_aliases")
            .and_then(toml::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|alias| alias.as_array()?.first()?.as_str());
        for name in std::iter::once(model).chain(aliases) {
            models.insert((make.to_owned(), name.to_owned()), clean_model.to_owned());
        }
    }
    models
}

/// The ISO darktable reads into `exif_iso` (`src/common/exif.cc`), as far as rawler's
/// EXIF reaches; 0 where it finds none. `second_rating` is ISOSpeedRatings' second value
/// where the file holds two, and `make` the maker as the file's EXIF spells it.
pub(crate) fn iso(exif: &Exif, second_rating: Option<u16>, make: &str) -> f32 {
    // Exiv2's `isoSpeed` (0.28, `easyaccess.cpp`) takes the first ISO tag above 0:
    // ISOSpeedRatings, then maker-note tags, which rawler does not read. darktable reads
    // ISOSpeedRatings' second value where it holds two (Nikon's Lo and Hi modes).
    let rating = exif.iso_speed_ratings.filter(|&rating| rating > 0);
    let mut iso = rating.map(|first| f32::from(second_rating.unwrap_or(first)));
    // At 65535, an overflow, or with no such tag, the first present of the tags
    // SensitivityType names, whatever its value. For its values 1 to 7 they are
    // StandardOutputSensitivity, which rawler does not read; RecommendedExposureIndex;
    // ISOSpeed; the index or the sensitivity; ISOSpeed or the sensitivity; ISOSpeed or
    // the index; and all three.
    if rating.is_none_or(|first| first == u16::MAX) {
        let (speed, index) = (exif.iso_speed, exif.recommended_exposure_index);
        let named = match exif.sensitivity_type {
            Some(2 | 4) => index,
            Some(3 | 5) => speed,
            Some(6 | 7) => speed.or(index),
            _ => None,
        };
        iso = named.map(|value| value as f32).or(iso);
    }
    let iso = iso.unwrap_or(0.0);
    // darktable's own fallback, for two makers only.
    match exif.recommended_exposure_index {
        Some(index) if (iso == 0.0 || iso == 65535.0) && matches!(make, "SONY" | "Canon") => {
            index as f32
        }
        _ => iso,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_alias_resolves_to_its_camera_and_the_later_camera_wins_a_name() {
        let cameras = r#"
            [[cameras]]
            make = "Canon"
            model = "Canon EOS 400D DIGITAL"
            clean_make = "Canon"
            clean_model = "EOS 400D"
            model_aliases = [["Canon EOS DIGITAL REBEL XTi", "EOS Digital Rebel XTi"]]

            [[cameras]]
            make = "Panasonic"
            model = "DMC-FZ2000"
            clean_make = "Panasonic"
            clean_model = "DMC-FZ2000"

            [[cameras]]
            make = "Panasonic"
            model = "DMC-FZ2500"
            clean_make = "Panasonic"
            clean_model = "DMC-FZ2500"
            model_aliases = [["DMC-FZ2000", "DMC-FZ2000"]]
        "#;
        let models = main_models(cameras);
        let model = |make: &str, model: &str| {
            models
                .get(&(make.to_owned(), model.to_owned()))
                .map(String::as_str)
        };
        assert_eq!(model("Canon", "Canon EOS 400D DIGITAL"), Some("EOS 400D"));
        assert_eq!(
            model("Canon", "Canon EOS DIGITAL REBEL XTi"),
            Some("EOS 400D")
        );
        // rawler's loader, too, decodes an FZ2000 as the FZ2500's alias.
        assert_eq!(model("Panasonic", "DMC-FZ2000"), Some("DMC-FZ2500"));
        assert_eq!(model("Canon", "EOS 400D"), None);
        assert!(main_models("not = [toml").is_empty());
    }

    #[test]
    fn rawler_s_own_data_resolves_aliases_as_darktable_names_them() {
        // darktable's names from rawspeed's cameras.xml at darktable's commit.
        assert_eq!(
            main_model("Canon", "Canon EOS DIGITAL REBEL XTi"),
            Some("EOS 400D")
        );
        assert_eq!(main_model("Canon", "Canon EOS Kiss X7"), Some("EOS 100D"));
        assert_eq!(main_model("Panasonic", "DC-S5D"), Some("DC-S5"));
        // A DNG's make and model can end in spaces.
        assert_eq!(main_model("Canon ", "Canon EOS 100D  "), Some("EOS 100D"));
        assert_eq!(main_model("Apple", "iPhone 12 Pro"), None);
    }

    fn exif(ratings: Option<u16>) -> Exif {
        Exif {
            iso_speed_ratings: ratings,
            ..Exif::default()
        }
    }

    #[test]
    fn iso_speed_ratings_comes_first_and_its_second_value_where_it_has_two() {
        assert_eq!(iso(&exif(Some(400)), None, "NIKON CORPORATION"), 400.0);
        assert_eq!(iso(&exif(Some(100)), Some(50), "NIKON CORPORATION"), 50.0);
        // Above 0 it is taken whatever SensitivityType names.
        let named = Exif {
            sensitivity_type: Some(3),
            iso_speed: Some(800),
            ..exif(Some(400))
        };
        assert_eq!(iso(&named, None, "Canon"), 400.0);
        assert_eq!(iso(&Exif::default(), None, "NIKON CORPORATION"), 0.0);
    }

    #[test]
    fn at_65535_or_missing_the_tag_sensitivity_type_names_is_taken() {
        let tags = |ratings, sensitivity_type| Exif {
            sensitivity_type,
            iso_speed: Some(102_400),
            recommended_exposure_index: Some(51_200),
            ..exif(ratings)
        };
        let make = "FUJIFILM";
        assert_eq!(iso(&tags(Some(65535), Some(2)), None, make), 51_200.0);
        assert_eq!(iso(&tags(None, Some(3)), None, make), 102_400.0);
        assert_eq!(iso(&tags(Some(0), Some(4)), None, make), 51_200.0);
        assert_eq!(iso(&tags(None, Some(6)), None, make), 102_400.0);
        // StandardOutputSensitivity, which rawler does not read, leaves the rating.
        assert_eq!(iso(&tags(Some(65535), Some(1)), None, make), 65535.0);
        assert_eq!(iso(&tags(None, None), None, make), 0.0);
        // Type 7 takes the index where ISOSpeed is missing.
        let without_speed = Exif {
            iso_speed: None,
            ..tags(None, Some(7))
        };
        assert_eq!(iso(&without_speed, None, make), 51_200.0);
    }

    #[test]
    fn recommended_exposure_index_is_the_last_resort_for_sony_and_canon_only() {
        let tags = |ratings| Exif {
            recommended_exposure_index: Some(204_800),
            ..exif(ratings)
        };
        assert_eq!(iso(&tags(Some(65535)), None, "SONY"), 204_800.0);
        assert_eq!(iso(&tags(None), None, "Canon"), 204_800.0);
        assert_eq!(iso(&tags(Some(65535)), None, "NIKON CORPORATION"), 65535.0);
        assert_eq!(iso(&tags(None), None, "Sony"), 0.0);
        // A named tag of 0 falls back too.
        let zero = Exif {
            sensitivity_type: Some(3),
            iso_speed: Some(0),
            ..tags(None)
        };
        assert_eq!(iso(&zero, None, "SONY"), 204_800.0);
    }
}
