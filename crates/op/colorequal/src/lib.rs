// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2022-2026  darktable developers (ported from src/iop/colorequal.c)
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

//! The color equalizer, ported from darktable (`src/iop/colorequal.c`): hue,
//! saturation and brightness changed by curves over the hue in darktable UCS, eight
//! nodes each, with darktable's guided filter, which first smooths the chromaticity the
//! hue is read from and then spreads the saturation and brightness corrections over
//! areas of one colour, on darktable's recursive Gaussian (`src/common/gaussian.c`,
//! shared in the colour library). The Colour equalizer module (`docs/modules/README.md`).
//! See `docs/ports/colorequal.md`.
//!
//! It is in `OPERATIONS` with `default_enabled` off: an unedited photo does not run
//! it, and the session plans it only while a module's binding sets one of its
//! parameters (decision 28).

mod setup;

pub mod reference;

use apertrust_color::WorkingProfile;
use apertrust_plugin::{
    Domain, ImageContext, ImageDesc, Kernel, Kind, Manifest, NotPorted, Operation, Param,
    ParamValues, Prepared, Provenance, Schema, Slot, Stage, Value,
};
pub use setup::*;

/// Generated from `slang/colorequal.slang`.
pub const KERNEL: Kernel = Kernel {
    name: "colorequal",
    wgsl: include_str!("../generated/colorequal.wgsl"),
    derivatives: true,
};

pub struct ColorEqual;

static MANIFEST: Manifest = Manifest {
    id: "colorequal",
    version: 1,
    // darktable's v5.0 place: after rgb primaries' group (28.5) and colorbalance (41.0),
    // before color balance rgb (41.5) (decision 22).
    slot: Slot {
        stage: Stage::WorkingSpace,
        order: 41.2,
    },
    domain: Domain::Scene,
    // The colour equalizer works on layers.
    layers: true,
    differentiable: true,
    // The guided filter's passes see the whole image and `main` reads only its own
    // pixel and their scratch, so a tile needs no border.
    radius: 0,
    // Planned only while a module sets one of its parameters (decision 28).
    default_enabled: false,
    provenance: Some(Provenance {
        darktable_module: "colorequal",
        release: "5.6.2",
        commit: "20891f6c6f",
        sources: &[
            "src/iop/colorequal.c",
            "src/iop/choleski.h",
            "src/common/gaussian.c",
            "src/common/gaussian.h",
            "src/common/fast_guided_filter.h",
            "src/common/darktable_ucs_22_helpers.h",
            "src/common/colorspaces_inline_conversions.h",
            "src/common/math.h",
            "data/kernels/colorequal.cl",
        ],
    }),
};

/// The colour names of darktable's eight nodes, in its order.
pub const COLOURS: [&str; NODES] = [
    "red", "orange", "yellow", "green", "cyan", "blue", "lavender", "magenta",
];

const fn float(name: &'static str, default: f32, min: f32, max: f32) -> Param {
    Param {
        name,
        kind: Kind::Float { default, min, max },
    }
}

const fn saturation(name: &'static str) -> Param {
    float(name, 1.0, 0.0, 2.0)
}

const fn hue(name: &'static str) -> Param {
    float(name, 0.0, -180.0, 180.0)
}

/// darktable `dt_iop_colorequal_params_t` version 4, by its field names, in its order,
/// with its introspection's ranges and defaults (decision 21).
static SCHEMA: Schema = Schema {
    version: 1,
    params: &[
        float("threshold", 0.1, 0.0, 0.3),
        float("smoothing_hue", 1.0, 0.05, 2.0),
        float("contrast", 0.0, -1.0, 1.0),
        float("white_level", 1.0, -2.0, 16.0),
        float("chroma_size", 1.5, 1.0, 10.0),
        float("param_size", 1.0, 1.0, 128.0),
        Param {
            name: "use_filter",
            kind: Kind::Bool { default: true },
        },
        saturation("sat_red"),
        saturation("sat_orange"),
        saturation("sat_yellow"),
        saturation("sat_green"),
        saturation("sat_cyan"),
        saturation("sat_blue"),
        saturation("sat_lavender"),
        saturation("sat_magenta"),
        hue("hue_red"),
        hue("hue_orange"),
        hue("hue_yellow"),
        hue("hue_green"),
        hue("hue_cyan"),
        hue("hue_blue"),
        hue("hue_lavender"),
        hue("hue_magenta"),
        saturation("bright_red"),
        saturation("bright_orange"),
        saturation("bright_yellow"),
        saturation("bright_green"),
        saturation("bright_cyan"),
        saturation("bright_blue"),
        saturation("bright_lavender"),
        saturation("bright_magenta"),
        float("hue_shift", 0.0, -23.0, 23.0),
    ],
};

/// The name of a node's parameter, `prefix` `sat`, `hue` or `bright`.
pub fn node_name(prefix: &str, k: usize) -> String {
    format!("{prefix}_{}", COLOURS[k])
}

/// The settings from resolved parameter values.
pub fn settings(params: &ParamValues) -> Settings {
    let f = |name: &str| params.float(name);
    let nodes = |prefix: &str| std::array::from_fn(|k| f(&node_name(prefix, k)));
    Settings {
        threshold: f("threshold"),
        smoothing_hue: f("smoothing_hue"),
        contrast: f("contrast"),
        white_level: f("white_level"),
        chroma_size: f("chroma_size"),
        param_size: f("param_size"),
        use_filter: params.flag("use_filter"),
        saturation: nodes("sat"),
        hue: nodes("hue"),
        brightness: nodes("bright"),
        hue_shift: f("hue_shift"),
    }
}

/// The parameter values of `settings`, by the schema's names: the inverse of
/// [`settings`]. They still need checking against the schema ([`Schema::resolve`]).
pub fn values(settings: &Settings) -> ParamValues {
    let s = settings;
    let mut values = ParamValues::new();
    for (name, value) in [
        ("threshold", s.threshold),
        ("smoothing_hue", s.smoothing_hue),
        ("contrast", s.contrast),
        ("white_level", s.white_level),
        ("chroma_size", s.chroma_size),
        ("param_size", s.param_size),
        ("hue_shift", s.hue_shift),
    ] {
        values.set(name, Value::Float(value));
    }
    values.set("use_filter", Value::Bool(s.use_filter));
    for (prefix, nodes) in [
        ("sat", s.saturation),
        ("hue", s.hue),
        ("bright", s.brightness),
    ] {
        for (k, value) in nodes.into_iter().enumerate() {
            values.set(&node_name(prefix, k), Value::Float(value));
        }
    }
    values
}

impl Operation for ColorEqual {
    fn manifest(&self) -> &'static Manifest {
        &MANIFEST
    }

    fn schema(&self) -> &'static Schema {
        &SCHEMA
    }

    fn kernel(&self) -> &'static Kernel {
        &KERNEL
    }

    /// darktable's `commit_params` and the start of its `process`. The working space is
    /// Apertrust's, linear Rec2020.
    fn prepare(
        &self,
        params: &ParamValues,
        image: &ImageContext,
        input: &ImageDesc,
    ) -> Result<Prepared, NotPorted> {
        // The scratch is laid out as the whole image at the pipe's scale (decision 16).
        prepare(
            &settings(params),
            input,
            image.whole_image(input),
            image.scale,
            &WorkingProfile::linear_rec2020(),
        )
    }

    /// `sample`, the one whole-image pass that reads the input, writes the chromaticity,
    /// saturation and lightness of the tile's own pixels at their place in the whole
    /// image (docs/ports/colorequal.md, Neighbourhood and scale). Without the guided
    /// filter there are no whole-image passes.
    fn tiles_whole_image_passes(&self, _params: &ParamValues) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_defaults_are_darktables() {
        assert_eq!(settings(&SCHEMA.defaults()), Settings::default());
    }

    #[test]
    fn the_schema_is_darktables_version_4_in_order() {
        // 32 fields: six floats, the filter's switch, 24 nodes and the node placement.
        assert_eq!(SCHEMA.params.len(), 32);
        assert_eq!(SCHEMA.params[6].name, "use_filter");
        assert_eq!(SCHEMA.params[7].name, "sat_red");
        assert_eq!(SCHEMA.params[15].name, "hue_red");
        assert_eq!(SCHEMA.params[23].name, "bright_red");
        assert_eq!(SCHEMA.params[30].name, "bright_magenta");
        assert_eq!(SCHEMA.params[31].name, "hue_shift");
        let names: std::collections::HashSet<_> = SCHEMA.params.iter().map(|p| p.name).collect();
        assert_eq!(names.len(), SCHEMA.params.len(), "a name repeats");
    }

    #[test]
    fn every_parameter_reaches_its_own_field() {
        let defaults = settings(&SCHEMA.defaults());
        for param in SCHEMA.params {
            let moved = match param.kind {
                Kind::Float { default, min, max } => {
                    Value::Float(if default < max { max } else { min })
                }
                Kind::Bool { default } => Value::Bool(!default),
                other => panic!("{} is {other:?}", param.name),
            };
            let mut one = ParamValues::new();
            one.set(param.name, moved);
            let resolved = SCHEMA.resolve(&one).unwrap();
            let changed = settings(&resolved);
            assert_ne!(changed, defaults, "{} reaches no field", param.name);
            assert_eq!(
                values(&changed),
                resolved,
                "{} does not round-trip",
                param.name
            );
        }
    }

    #[test]
    fn prepare_uses_the_pipes_scale_and_the_input() {
        let params = SCHEMA.defaults();
        let input = ImageDesc::rgba(1954, 1301);
        let image = ImageContext {
            scale: 0.5,
            ..ImageContext::full(3908, 2602)
        };
        let ours = ColorEqual.prepare(&params, &image, &input).unwrap();
        let direct = prepare(
            &settings(&params),
            &input,
            [1954, 1301],
            0.5,
            &WorkingProfile::linear_rec2020(),
        )
        .unwrap();
        assert_eq!(ours, direct);
    }

    #[test]
    fn a_tile_lays_the_scratch_out_as_the_whole_image() {
        // A tile at (200, 160) of its stage, as the planner describes it.
        let params = SCHEMA.defaults();
        let tile = ImageDesc {
            origin: [200, 160],
            extent: [3908, 2602],
            scaled_extent: [3908, 2602],
            ..ImageDesc::rgba(100, 80)
        };
        let image = ImageContext::full(3908, 2602);
        let prepared = ColorEqual.prepare(&params, &image, &tile).unwrap();
        assert_eq!(prepared.output, tile);
        assert_eq!(prepared.scratch[0].bytes, 3908 * 2602 * 8);
        assert!(ColorEqual.tiles_whole_image_passes(&params));
    }

    #[test]
    fn the_operation_is_differentiable() {
        // That its kernel agrees, and its slot, are the operations' own checks.
        assert!(ColorEqual.manifest().differentiable);
        assert_eq!(ColorEqual.manifest().radius, 0);
    }
}
