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

//! Color calibration, ported from darktable (`src/iop/channelmixerrgb.c`):
//! chromatic adaptation from the illuminant to the pipeline's D50 in every adaptation
//! darktable offers, from every illuminant it offers, then the mixer, gamut
//! compression, colourfulness, brightness and grey output, with params version 3's
//! colourfulness. White balance and Black and white drive it (decision 2).
//! See `docs/ports/channelmixerrgb.md`.
//!
//! It is in `OPERATIONS` with `default_enabled` on: every colour raw runs it, adapting the
//! camera's as-shot white to D50 after temperature leaves the image at the camera's
//! reference, with darktable's defaults, which snap that white to daylight or Planckian
//! where one lies close, as darktable's scene-referred default does (decision 27).

mod setup;

use apertrust_color::{Adaptation, Illuminant, WorkingProfile};
use apertrust_plugin::{
    Domain, Format, ImageContext, ImageDesc, Kernel, Kind, Manifest, NotPorted, Operation, Param,
    ParamValues, Provenance, Schema, Slot, Stage, Value,
};
pub use setup::*;

/// Generated from `slang/channelmixerrgb.slang`.
pub const KERNEL: Kernel = Kernel {
    name: "channelmixerrgb",
    wgsl: include_str!("../generated/channelmixerrgb.wgsl"),
    derivatives: true,
};

pub struct ChannelMixerRgb;

static MANIFEST: Manifest = Manifest {
    id: "channelmixerrgb",
    version: 1,
    // darktable's v5.0 order: immediately after input colour (28), first of the
    // operations that share 28.5, before color balance rgb (41.5) and the tone mappers
    // (decision 22).
    slot: Slot {
        stage: Stage::WorkingSpace,
        order: 28.5,
    },
    domain: Domain::Scene,
    // White balance works on layers (decision 2).
    layers: true,
    differentiable: true,
    radius: 0,
    // On every photo, from darktable's defaults for it (decision 27).
    default_enabled: true,
    provenance: Some(Provenance {
        darktable_module: "channelmixerrgb",
        release: "5.6.2",
        commit: "20891f6c6f",
        sources: &[
            "src/iop/channelmixerrgb.c",
            "src/common/chromatic_adaptation.h",
            "src/common/illuminants.h",
            "src/common/colorspaces_inline_conversions.h",
            "src/common/math.h",
            "data/kernels/channelmixer.cl",
        ],
    }),
};

const fn float(name: &'static str, default: f32, min: f32, max: f32) -> Param {
    Param {
        name,
        kind: Kind::Float { default, min, max },
    }
}

/// darktable's `COLOR_MIN` to `COLOR_MAX` for the mixer, and the range of colourfulness,
/// brightness and grey.
const fn coefficient(name: &'static str, default: f32) -> Param {
    float(name, default, -2.0, 2.0)
}

const fn flag(name: &'static str, default: bool) -> Param {
    Param {
        name,
        kind: Kind::Bool { default },
    }
}

const fn choice(name: &'static str, default: u32, choices: &'static [&'static str]) -> Param {
    Param {
        name,
        kind: Kind::Choice { default, choices },
    }
}

/// darktable's illuminants, by their values (`dt_illuminant_t`): `detect_surfaces` and
/// `detect_edges`, compiled out in darktable, are not ported.
const ILLUMINANTS: &[&str] = &[
    "pipe",
    "a",
    "d",
    "e",
    "f",
    "led",
    "bb",
    "custom",
    "detect_surfaces",
    "detect_edges",
    "camera",
];

/// darktable `dt_iop_channelmixer_rgb_params_t` version 3, by its field names, arrays
/// with an `_<index>` suffix, with its ranges and the defaults before it looks at the
/// image ([`Settings::DEFAULT`]). The illuminant's x runs from −0.1 to 1.2 and its y
/// from 0.0001 to 2, which darktable leaves unbounded: they hold every light darktable's
/// hue and chroma reach, chroma to 300 % giving x −0.050 to 1.096 and y 0.112 to 1.879
/// (decision 27); `prepare` keeps x + y below 1.
static SCHEMA: Schema = Schema {
    version: 1,
    params: &[
        coefficient("red_0", 1.0),
        coefficient("red_1", 0.0),
        coefficient("red_2", 0.0),
        coefficient("green_0", 0.0),
        coefficient("green_1", 1.0),
        coefficient("green_2", 0.0),
        coefficient("blue_0", 0.0),
        coefficient("blue_1", 0.0),
        coefficient("blue_2", 1.0),
        coefficient("saturation_0", 0.0),
        coefficient("saturation_1", 0.0),
        coefficient("saturation_2", 0.0),
        coefficient("lightness_0", 0.0),
        coefficient("lightness_1", 0.0),
        coefficient("lightness_2", 0.0),
        coefficient("grey_0", 0.0),
        coefficient("grey_1", 0.0),
        coefficient("grey_2", 0.0),
        flag("normalize_R", false),
        flag("normalize_G", false),
        flag("normalize_B", false),
        flag("normalize_sat", false),
        flag("normalize_light", false),
        flag("normalize_grey", false),
        choice("illuminant", 2, ILLUMINANTS),
        choice(
            "illum_fluo",
            2,
            &[
                "f1", "f2", "f3", "f4", "f5", "f6", "f7", "f8", "f9", "f10", "f11", "f12",
            ],
        ),
        choice(
            "illum_led",
            4,
            &["b1", "b2", "b3", "b4", "b5", "bh1", "rgb1", "v1", "v2"],
        ),
        choice(
            "adaptation",
            1,
            &["linear_bradford", "cat16", "full_bradford", "xyz", "rgb"],
        ),
        float("x", 0.333, -0.1, 1.2),
        float("y", 0.333, 0.0001, 2.0),
        float("temperature", 5003.0, 1667.0, 25000.0),
        float("gamut", 1.0, 0.0, 12.0),
        flag("clip", true),
        choice("version", 2, &["v1", "v2", "v3"]),
    ],
};

/// The settings from resolved parameter values.
pub fn settings(params: &ParamValues) -> Settings {
    let array = |name: &str| [0, 1, 2].map(|i| params.float(&format!("{name}_{i}")));
    Settings {
        red: array("red"),
        green: array("green"),
        blue: array("blue"),
        saturation: array("saturation"),
        lightness: array("lightness"),
        grey: array("grey"),
        normalize_r: params.flag("normalize_R"),
        normalize_g: params.flag("normalize_G"),
        normalize_b: params.flag("normalize_B"),
        normalize_sat: params.flag("normalize_sat"),
        normalize_light: params.flag("normalize_light"),
        normalize_grey: params.flag("normalize_grey"),
        illuminant: Illuminant::from_darktable(params.choice("illuminant") as i32)
            .expect("the schema's illuminants are darktable's"),
        illum_fluo: params.choice("illum_fluo"),
        illum_led: params.choice("illum_led"),
        adaptation: Adaptation::from_darktable(params.choice("adaptation") as i32)
            .expect("the schema's adaptations are darktable's"),
        x: params.float("x"),
        y: params.float("y"),
        temperature: params.float("temperature"),
        gamut: params.float("gamut"),
        clip: params.flag("clip"),
        version: Version::from_darktable(params.choice("version") as i32)
            .expect("the schema's versions are darktable's"),
    }
}

/// Parameter values for these settings, the inverse of [`settings`].
pub fn values(settings: &Settings) -> ParamValues {
    let mut values = ParamValues::new();
    let arrays = [
        ("red", settings.red),
        ("green", settings.green),
        ("blue", settings.blue),
        ("saturation", settings.saturation),
        ("lightness", settings.lightness),
        ("grey", settings.grey),
    ];
    for (name, array) in arrays {
        for (i, value) in array.into_iter().enumerate() {
            values.set(&format!("{name}_{i}"), Value::Float(value));
        }
    }
    for (name, flag) in [
        ("normalize_R", settings.normalize_r),
        ("normalize_G", settings.normalize_g),
        ("normalize_B", settings.normalize_b),
        ("normalize_sat", settings.normalize_sat),
        ("normalize_light", settings.normalize_light),
        ("normalize_grey", settings.normalize_grey),
        ("clip", settings.clip),
    ] {
        values.set(name, Value::Bool(flag));
    }
    let version = Version::ALL
        .iter()
        .position(|&v| v == settings.version)
        .unwrap();
    for (name, choice) in [
        ("illuminant", settings.illuminant.to_darktable() as u32),
        ("illum_fluo", settings.illum_fluo),
        ("illum_led", settings.illum_led),
        ("adaptation", settings.adaptation.to_darktable() as u32),
        ("version", version as u32),
    ] {
        values.set(name, Value::Choice(choice));
    }
    for (name, value) in [
        ("x", settings.x),
        ("y", settings.y),
        ("temperature", settings.temperature),
        ("gamut", settings.gamut),
    ] {
        values.set(name, Value::Float(value));
    }
    values
}

/// What the camera's illuminant needs from the image: its matrix and as-shot
/// multipliers, and the multipliers the image carries here.
fn camera_white(image: &ImageContext, applied: [f32; 3]) -> Option<CameraWhite> {
    let camera = image.camera.as_ref()?;
    let xyz_to_camera = camera.xyz_to_camera?;
    let as_shot = camera.white_balance.as_deref()?;
    Some(CameraWhite {
        xyz_to_camera,
        as_shot: [*as_shot.first()?, *as_shot.get(1)?, *as_shot.get(2)?],
        applied,
    })
}

impl Operation for ChannelMixerRgb {
    fn manifest(&self) -> &'static Manifest {
        &MANIFEST
    }

    fn schema(&self) -> &'static Schema {
        &SCHEMA
    }

    fn kernel(&self) -> &'static Kernel {
        &KERNEL
    }

    /// darktable's `reload_defaults` ([`image_defaults`]): on a colour raw, the camera's
    /// light under CAT16, as daylight or Planckian where one lies within 0.005 of it.
    fn defaults(&self, image: &ImageContext) -> ParamValues {
        let camera = camera_white(image, [1.0; 3]);
        let kind = match &image.camera {
            None => DefaultsFor::NotRaw,
            Some(raw) if raw.cfa.is_none() => DefaultsFor::MonochromeRaw,
            Some(_) => DefaultsFor::ColourRaw(camera.as_ref()),
        };
        let mut defaults = SCHEMA.defaults();
        for (name, value) in values(&image_defaults(kind)).iter() {
            defaults.set(name, value);
        }
        defaults
    }

    fn prepare(
        &self,
        params: &ParamValues,
        image: &ImageContext,
        input: &ImageDesc,
    ) -> Result<apertrust_plugin::Prepared, NotPorted> {
        if input.format != Format::Rgba {
            return Err(NotPorted {
                operation: "channelmixerrgb",
                what: "images other than RGB",
            });
        }
        let camera = camera_white(image, input.white_balance);
        // Apertrust's working space is fixed: linear Rec2020.
        let prepared = prepare(
            &settings(params),
            &WorkingProfile::linear_rec2020(),
            camera.as_ref(),
        )?;
        Ok(apertrust_plugin::Prepared::per_pixel(
            input,
            prepared.params(),
            Vec::new(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apertrust_plugin::{Camera, Pattern};

    #[test]
    fn schema_defaults_are_darktables() {
        assert_eq!(settings(&SCHEMA.defaults()), Settings::DEFAULT);
    }

    #[test]
    fn settings_round_trip_through_values() {
        let moved = Settings {
            red: [0.5, 0.25, -0.25],
            grey: [0.2, 0.3, 0.5],
            normalize_g: true,
            illuminant: Illuminant::Camera,
            illum_led: 7,
            adaptation: Adaptation::FullBradford,
            x: 0.4,
            clip: false,
            version: Version::V2,
            ..Settings::DEFAULT
        };
        let values = SCHEMA.resolve(&values(&moved)).unwrap();
        assert_eq!(settings(&values), moved);
    }

    #[test]
    fn every_field_has_its_own_parameter() {
        // Move every parameter off its default, and check the settings change.
        for param in SCHEMA.params {
            let mut values = ParamValues::new();
            let moved = match param.kind {
                Kind::Float { default, min, max } => {
                    Value::Float(if default < max { max } else { min })
                }
                Kind::Int { default, min, max } => {
                    Value::Int(if default < max { max } else { min })
                }
                Kind::Choice { default, .. } => Value::Choice(if default == 0 { 1 } else { 0 }),
                Kind::Bool { default } => Value::Bool(!default),
            };
            values.set(param.name, moved);
            let changed = settings(&SCHEMA.resolve(&values).unwrap());
            assert_ne!(
                changed,
                Settings::DEFAULT,
                "{} reaches no field",
                param.name
            );
        }
        let names: std::collections::HashSet<_> = SCHEMA.params.iter().map(|p| p.name).collect();
        assert_eq!(names.len(), SCHEMA.params.len(), "a name repeats");
    }

    fn raw(cfa: bool) -> ImageContext {
        let mut image = ImageContext::full(4, 4);
        image.camera = Some(Camera {
            make: String::new(),
            model: String::new(),
            alias: String::new(),
            width: 4,
            height: 4,
            cfa: cfa.then(|| Pattern {
                width: 2,
                height: 2,
                values: vec![0, 1, 1, 2],
            }),
            black: Pattern {
                width: 1,
                height: 1,
                values: vec![0.0],
            },
            white: 16383.0,
            white_balance: Some(vec![1.9139073, 1.0, 2.1887417]),
            xyz_to_camera: Some([
                [1.3426, -0.6334, -0.1177],
                [-0.4244, 1.2136, 0.2371],
                [-0.058, 0.1303, 0.598],
            ]),
            crop: [0, 0, 4, 4],
            declared: [0, 0, 4, 4],
            exposure_bias: 0.0,
            iso: 0.0,
            linear_response_limit: 1.0,
            lens_correction: None,
            orientation: 0,
        });
        image
    }

    #[test]
    fn a_raw_defaults_to_its_cameras_illuminant() {
        let defaults = settings(&ChannelMixerRgb.defaults(&raw(true)));
        assert_eq!(defaults.illuminant, Illuminant::Custom);
        assert_eq!(defaults.adaptation, Adaptation::Cat16);
        assert!((defaults.x - 0.3892037).abs() < 2e-7);
        let monochrome = settings(&ChannelMixerRgb.defaults(&raw(false)));
        assert_eq!(monochrome.adaptation, Adaptation::Rgb);
        let other = settings(&ChannelMixerRgb.defaults(&ImageContext::full(4, 4)));
        assert_eq!(other, Settings::DEFAULT);
        // A colour raw whose camera records no white balance: darktable's defaults for
        // its stored 0.333, 0.333.
        let mut unrecorded = raw(true);
        unrecorded.camera.as_mut().unwrap().white_balance = None;
        let unrecorded = settings(&ChannelMixerRgb.defaults(&unrecorded));
        assert_eq!(
            (unrecorded.illuminant, unrecorded.x),
            (Illuminant::Custom, 0.333)
        );
        // The defaults are within the schema.
        assert!(
            SCHEMA
                .resolve(&ChannelMixerRgb.defaults(&raw(true)))
                .is_ok()
        );
    }

    #[test]
    fn prepare_gives_one_per_pixel_pass() {
        let image = raw(true);
        let params = ChannelMixerRgb.defaults(&image);
        let prepared = ChannelMixerRgb
            .prepare(&params, &image, &ImageDesc::rgba(4, 4))
            .unwrap();
        assert_eq!(prepared.passes.len(), 1);
        assert_eq!(prepared.passes[0].params.len(), PARAMS_SIZE);
        assert_eq!(prepared.output, ImageDesc::rgba(4, 4));
        let mosaic = ImageDesc::new(4, 4, Format::Mosaic);
        assert!(ChannelMixerRgb.prepare(&params, &image, &mosaic).is_err());
    }

    #[test]
    fn the_camera_illuminant_reads_the_image_description() {
        // After "as shot to reference", colorin leaves the image at the camera's D65
        // multipliers, where the camera's illuminant is the as-shot one.
        let image = raw(true);
        let mut params = SCHEMA.defaults();
        params.set("illuminant", Value::Choice(10));
        let camera = camera_white(&image, [1.0; 3]).unwrap();
        let d65 = apertrust_color::daylight_multipliers(&camera.xyz_to_camera).map(|v| v as f32);
        let mut input = ImageDesc::rgba(4, 4);
        input.white_balance = d65;
        let at_reference = ChannelMixerRgb.prepare(&params, &image, &input).unwrap();
        let expected = prepare(
            &Settings {
                illuminant: Illuminant::Custom,
                x: 0.3892037,
                y: 0.41662326,
                ..Settings::DEFAULT
            },
            &WorkingProfile::linear_rec2020(),
            None,
        )
        .unwrap();
        let x = |bytes: &[u8]| f32::from_le_bytes(bytes[352..356].try_into().unwrap());
        assert!((x(&at_reference.passes[0].params) - x(&expected.params())).abs() < 2e-7);
    }
}
