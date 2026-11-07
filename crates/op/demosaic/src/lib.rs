// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2010-2026  darktable developers (ported from src/iop/demosaic.c)
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

//! Demosaic, ported from darktable (`src/iop/demosaic.c` and
//! `src/iop/demosaicing/`): each photosite's two missing colours, from its neighbours.
//! Only RCD for Bayer sensors (inside PPG's edge band, as darktable runs it) and
//! Markesteijn 1-pass for X-Trans are ported, darktable's defaults for them; the other
//! methods, Monochrome and VNG4 among them, darktable's defaults for a raw with no
//! colour filters and a pattern of four colours, which are refused, and green
//! equilibration, colour smoothing, dual demosaicing and capture sharpening return
//! `NotPorted` (decision 51). `docs/ports/demosaic.md` is its spec.
//!
//! The passes run over the whole mosaic, the active area, and write only the declared
//! crop, the manufacturer's image size, so demosaic has real neighbours at its edges
//! (decision 23).

use apertrust_plugin::{
    Binding, Domain, Format, ImageContext, ImageDesc, Kernel, Kind, Manifest, NotPorted, Operation,
    Param, ParamValues, Pass, PassKind, Pattern, Prepared, Provenance, Resource, Schema, Scratch,
    Slot, Stage, Value,
};

/// Generated from `slang/demosaic.slang`.
pub const KERNEL: Kernel = Kernel {
    name: "demosaic",
    wgsl: include_str!("../generated/demosaic.wgsl"),
    derivatives: false,
};

pub struct Demosaic;

static MANIFEST: Manifest = Manifest {
    id: "demosaic",
    version: 1,
    slot: Slot {
        stage: Stage::Raw,
        order: 8.0,
    },
    domain: Domain::Scene,
    layers: false,
    differentiable: false,
    // darktable's tiling border for RCD, 10, and Markesteijn 1-pass, 12.
    radius: 12,
    default_enabled: true,
    provenance: Some(Provenance {
        darktable_module: "demosaic",
        release: "5.6.2",
        commit: "20891f6c6f",
        sources: &[
            "src/iop/demosaic.c",
            "src/iop/demosaicing/ppg.c",
            "src/iop/demosaicing/rcd.c",
            "src/iop/demosaicing/xtrans.c",
            "data/kernels/demosaic_rcd.cl",
            "data/kernels/demosaic_markesteijn.cl",
        ],
    }),
};

/// darktable's demosaicing methods, by Apertrust's choice name and darktable's value.
pub const METHODS: &[(&str, u32)] = &[
    ("ppg", 0),
    ("amaze", 1),
    ("vng4", 2),
    ("passthrough_monochrome", 3),
    ("photosite_color", 4),
    ("rcd", 5),
    ("lmmse", 6),
    ("monochrome", 7),
    ("vng", 1024),
    ("markesteijn", 1025),
    ("markesteijn_3", 1026),
    ("passthrough_monochrome_xtrans", 1027),
    ("fdc", 1028),
    ("photosite_color_xtrans", 1029),
    ("rcd_dual", 2053),
    ("amaze_dual", 2049),
    ("markesteijn_3_dual", 3074),
];

const METHOD_NAMES: &[&str] = &[
    "ppg",
    "amaze",
    "vng4",
    "passthrough_monochrome",
    "photosite_color",
    "rcd",
    "lmmse",
    "monochrome",
    "vng",
    "markesteijn",
    "markesteijn_3",
    "passthrough_monochrome_xtrans",
    "fdc",
    "photosite_color_xtrans",
    "rcd_dual",
    "amaze_dual",
    "markesteijn_3_dual",
];

/// The index of a method in [`METHODS`].
fn method(name: &str) -> u32 {
    METHOD_NAMES
        .iter()
        .position(|&m| m == name)
        .expect("a method of METHODS") as u32
}

const fn float(name: &'static str, default: f32, min: f32, max: f32) -> Param {
    Param {
        name,
        kind: Kind::Float { default, min, max },
    }
}

/// darktable `dt_iop_demosaic_params_t` version 6, with its ranges and defaults.
static SCHEMA: Schema = Schema {
    version: 1,
    params: &[
        Param {
            name: "green_eq",
            kind: Kind::Choice {
                default: 0,
                choices: &["no", "local", "full", "both"],
            },
        },
        float("median_thrs", 0.0, 0.0, 1.0),
        Param {
            name: "color_smoothing",
            kind: Kind::Choice {
                default: 0,
                choices: &["off", "once", "twice", "three", "four", "five"],
            },
        },
        Param {
            name: "demosaicing_method",
            kind: Kind::Choice {
                default: 5,
                choices: METHOD_NAMES,
            },
        },
        Param {
            name: "lmmse_refine",
            kind: Kind::Choice {
                default: 1,
                choices: &[
                    "basic",
                    "median",
                    "3x_median",
                    "refine_medians",
                    "2x_refine_medians",
                ],
            },
        },
        float("dual_thrs", 0.2, 0.0, 1.0),
        float("cs_radius", 0.0, 0.0, 1.5),
        float("cs_thrs", 0.4, 0.0, 1.0),
        float("cs_boost", 0.0, 0.0, 1.5),
        Param {
            name: "cs_iter",
            kind: Kind::Int {
                default: 8,
                min: 1,
                max: 25,
            },
        },
        float("cs_center", 0.0, 0.0, 1.0),
        Param {
            name: "cs_enabled",
            kind: Kind::Bool { default: false },
        },
    ],
};

/// The colour filter pattern a raw's demosaic works with.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Sensor {
    Bayer,
    XTrans,
}

fn sensor(image: &ImageContext) -> Option<Sensor> {
    let cfa = image.camera.as_ref()?.cfa.as_ref()?;
    match (cfa.width, cfa.height) {
        (2, 2) => Some(Sensor::Bayer),
        (6, 6) => Some(Sensor::XTrans),
        _ => None,
    }
}

/// Planes of RCD's, in [`Scratch`] order and at bindings 10 on: cfa, VH_Dir, the
/// half-width lpf and PQ_Dir, P and Q diagonal filters, then red, green and blue.
const RCD_PLANES: usize = 8;
/// darktable RCD_BORDER: an image needs twice this in each direction for RCD.
const RCD_BORDER: u32 = 10;

impl Operation for Demosaic {
    fn manifest(&self) -> &'static Manifest {
        &MANIFEST
    }

    fn schema(&self) -> &'static Schema {
        &SCHEMA
    }

    fn kernel(&self) -> &'static Kernel {
        &KERNEL
    }

    /// darktable's `reload_defaults`, in its order: Monochrome for a raw with no colour
    /// filters, as darktable's monochrome sraw; Markesteijn 1-pass for X-Trans; VNG4 for a
    /// pattern of four colours, whatever its size; RCD otherwise, an image that is not a
    /// raw included. Neither VNG4 nor Monochrome is ported, and `prepare` refuses both
    /// raws. The image context cannot yet tell a monochrome sraw from a linear raw, which
    /// has no colour filters either and takes RCD in darktable (`docs/ports/demosaic.md`,
    /// Open questions).
    fn defaults(&self, image: &ImageContext) -> ParamValues {
        let mut values = SCHEMA.defaults();
        let default = match image.camera.as_ref().map(|camera| camera.cfa.as_ref()) {
            Some(None) => "monochrome",
            Some(Some(cfa)) if (cfa.width, cfa.height) == (6, 6) => "markesteijn",
            Some(Some(cfa)) if cfa.values.iter().any(|&c| c > 2) => "vng4",
            Some(Some(_)) | None => "rcd",
        };
        values.set("demosaicing_method", Value::Choice(method(default)));
        values
    }

    fn prepare(
        &self,
        params: &ParamValues,
        image: &ImageContext,
        input: &ImageDesc,
    ) -> Result<Prepared, NotPorted> {
        let not_ported = |what| NotPorted {
            operation: "demosaic",
            what,
        };
        if input.format != Format::Mosaic {
            return Err(not_ported("images other than a prepared mosaic"));
        }
        let camera = image
            .camera
            .as_ref()
            .ok_or(not_ported("images that are not raws"))?;
        let sensor = sensor(image).ok_or(not_ported("sensors other than Bayer and X-Trans"))?;
        let cfa = camera.cfa.as_ref().ok_or(not_ported("monochrome raws"))?;
        if cfa.values.iter().any(|&c| c > 2) {
            return Err(not_ported("colour filters other than red, green and blue"));
        }
        if params.choice("green_eq") != 0 {
            return Err(not_ported("green equilibration"));
        }
        if params.choice("color_smoothing") != 0 {
            return Err(not_ported("colour smoothing"));
        }
        if params.flag("cs_enabled") {
            return Err(not_ported("capture sharpening"));
        }
        // darktable `commit_params`: a method for the other kind of sensor falls back to
        // that sensor's default.
        let chosen = METHOD_NAMES[params.choice("demosaicing_method") as usize];
        let xtrans_method = METHODS[params.choice("demosaicing_method") as usize].1 & 1024 != 0;
        let chosen = match (sensor, xtrans_method) {
            (Sensor::Bayer, true) => "rcd",
            (Sensor::XTrans, false) => "markesteijn",
            _ => chosen,
        };
        // darktable switches to VNG below 16 photosites in either direction.
        if input.width < 16 || input.height < 16 {
            return Err(not_ported("mosaics smaller than 16 photosites"));
        }
        // The part of the declared image the pipe produces, on the readout: all of it,
        // or a tile (decision 13).
        let [x, y, ..] = camera.declared;
        let [rx, ry, width, height] = image.region;
        let crop = declared_crop([x + rx, y + ry, width, height], input);
        match chosen {
            "rcd" => Ok(rcd(cfa, input, &crop)),
            "markesteijn" => Ok(markesteijn(cfa, input, &crop)),
            _ => Err(not_ported(
                "demosaicing methods other than RCD and Markesteijn 1-pass",
            )),
        }
    }
}

/// The declared crop, or the part of it the pipe produces, in the mosaic's coordinates:
/// x, y, width, height; the whole mosaic when it does not lie inside the mosaic.
fn declared_crop(declared: [u32; 4], input: &ImageDesc) -> [u32; 4] {
    let [x, y, width, height] = declared;
    let [ox, oy] = input.sensor_origin;
    let inside = x >= ox
        && y >= oy
        && x + width <= ox + input.width
        && y + height <= oy + input.height
        && width > 0
        && height > 0;
    if inside {
        [x - ox, y - oy, width, height]
    } else {
        [0, 0, input.width, input.height]
    }
}

/// The output: the declared crop, RGB, at its place on the sensor.
fn output(input: &ImageDesc, crop: &[u32; 4]) -> ImageDesc {
    let [x, y, width, height] = *crop;
    ImageDesc {
        width,
        height,
        format: Format::Rgba,
        sensor_origin: [input.sensor_origin[0] + x, input.sensor_origin[1] + y],
        ..*input
    }
}

/// darktable allhex: eight (x, y) offsets per position of the 3×3 repeat, by row then
/// column.
type Hexagons = [[[(i32, i32); 8]; 3]; 3];

/// `DemosaicParams` in the kernel, for one pass.
struct Params<'a> {
    input: &'a ImageDesc,
    crop: &'a [u32; 4],
    stride: u32,
    step: u32,
    /// The filter colour per position of the pattern from the mosaic's corner: four for
    /// Bayer, 36 for X-Trans.
    colors: &'a [u32],
    /// X-Trans: the green hexagons and the solitary green's position.
    hexes: &'a Hexagons,
    solitary: (u32, u32),
}

impl Params<'_> {
    fn bytes(&self) -> Vec<u8> {
        let maximum = self.input.processed_maximum();
        let mut bytes = Vec::with_capacity(48 + 144 + 576);
        let words = [self.input.width, self.input.height, self.stride, self.step];
        for word in words.iter().chain(self.crop) {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        for value in [1.0 / maximum, maximum] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for word in [self.solitary.0, self.solitary.1] {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        for position in 0..36 {
            let color = self.colors.get(position).copied().unwrap_or(0);
            bytes.extend_from_slice(&color.to_le_bytes());
        }
        for (dx, dy) in self.hexes.iter().flatten().flatten() {
            bytes.extend_from_slice(&dx.to_le_bytes());
            bytes.extend_from_slice(&dy.to_le_bytes());
        }
        bytes
    }
}

/// The filter colour at (col, row) of the mosaic, which starts at its sensor origin.
fn color(cfa: &Pattern<u32>, input: &ImageDesc, col: i64, row: i64) -> u32 {
    let x = (i64::from(input.sensor_origin[0]) + col).rem_euclid(i64::from(cfa.width));
    let y = (i64::from(input.sensor_origin[1]) + row).rem_euclid(i64::from(cfa.height));
    cfa.values[(y * i64::from(cfa.width) + x) as usize]
}

/// darktable `xtrans_markesteijn_interpolate`'s set-up: a hexagon of greens around each
/// position of the 3×3 repeat, as (x, y) offsets, and the solitary green's (x, y), the
/// one with non-green photosites on all four sides.
fn hexagons(pattern: impl Fn(i64, i64) -> u32) -> (Hexagons, (u32, u32)) {
    const ORTH: [i32; 12] = [1, 0, 0, 1, -1, 0, 0, -1, 1, 0, 0, 1];
    const PATT: [[i32; 16]; 2] = [
        [0, 1, 0, -1, 2, 0, -1, 0, 1, 1, 1, -1, 0, 0, 0, 0],
        [0, 1, 0, -2, 1, 0, -2, 0, 1, 1, -2, -2, 1, -1, -1, 1],
    ];
    let mut hexes: Hexagons = [[[(0, 0); 8]; 3]; 3];
    let mut solitary = (0, 0);
    for (row, hex_row) in hexes.iter_mut().enumerate() {
        for (col, hex) in hex_row.iter_mut().enumerate() {
            let g = usize::from(pattern(col as i64, row as i64) == 1);
            let mut ng = 0;
            for d in (0..10).step_by(2) {
                let (dy, dx) = (ORTH[d], ORTH[d + 2]);
                if pattern(col as i64 + i64::from(dx), row as i64 + i64::from(dy)) == 1 {
                    ng = 0;
                } else {
                    ng += 1;
                }
                if ng == 4 {
                    solitary = (col as u32, row as u32);
                }
                if ng == g + 1 {
                    for c in 0..8 {
                        let v = ORTH[d] * PATT[g][c * 2] + ORTH[d + 1] * PATT[g][c * 2 + 1];
                        let h = ORTH[d + 2] * PATT[g][c * 2] + ORTH[d + 3] * PATT[g][c * 2 + 1];
                        hex[c ^ ((g * 2) & d)] = (h, v);
                    }
                }
            }
        }
    }
    (hexes, solitary)
}

/// darktable `rcd_demosaic`: PPG over the whole mosaic, then RCD's steps, whose result
/// replaces PPG's 9 or more photosites from the edges.
fn rcd(cfa: &Pattern<u32>, input: &ImageDesc, crop: &[u32; 4]) -> Prepared {
    // An even row stride, as darktable's tiles have, for its half-width planes.
    let stride = input.width.next_multiple_of(2);
    let colors = [0, 1, 2, 3].map(|position| color(cfa, input, position & 1, position >> 1));
    let params = Params {
        input,
        crop,
        stride,
        step: 0,
        colors: &colors,
        hexes: &[[[(0, 0); 8]; 3]; 3],
        solitary: (0, 0),
    }
    .bytes();
    let size = [input.width, input.height];
    let pass = |entry_point, radius, bindings: &[(u32, Resource)]| Pass {
        entry_point,
        kind: PassKind::Spatial { radius },
        params: params.clone(),
        tables: Vec::new(),
        bindings: [(2, Resource::Uniforms)]
            .iter()
            .chain(bindings)
            .map(|&(binding, resource)| Binding::new(binding, resource))
            .collect(),
        size: Some(size),
        // Demosaic runs whole or in tiles of its own, never in strips (decision 14).
        strips: None,
    };
    let plane = |i: usize| (10 + i as u32, Resource::Scratch(i));
    let [cfa_plane, vh, pq, p, q, r, g, b] = [0, 1, 2, 3, 4, 5, 6, 7].map(plane);
    let input_binding = (0, Resource::Input);
    let output_binding = (1, Resource::Output);
    let mut passes = vec![
        pass("ppg_green", 3, &[input_binding, r, g, b]),
        pass("ppg_redblue", 1, &[output_binding, r, g, b]),
    ];
    // darktable runs RCD only with RCD_BORDER to spare on every side.
    if input.width >= 2 * RCD_BORDER && input.height >= 2 * RCD_BORDER {
        passes.extend([
            pass("rcd_populate", 0, &[input_binding, cfa_plane, r, g, b]),
            pass("rcd_step_1", 4, &[cfa_plane, vh]),
            pass("rcd_step_2", 1, &[cfa_plane, pq]),
            pass("rcd_step_3", 4, &[cfa_plane, vh, pq, g]),
            pass("rcd_step_4_0", 3, &[cfa_plane, p, q]),
            pass("rcd_step_4_1", 1, &[pq, p, q]),
            pass("rcd_step_4_2", 3, &[pq, r, g, b]),
            pass("rcd_step_4_3", 3, &[vh, r, g, b]),
            pass("rcd_write", 0, &[output_binding, r, g, b]),
        ]);
    }
    let full = u64::from(stride) * u64::from(input.height) * 4;
    let half = full / 2;
    let scratch = (0..RCD_PLANES)
        .map(|i| Scratch {
            bytes: if (2..=4).contains(&i) { half } else { full },
        })
        .collect();
    Prepared {
        output: output(input, crop),
        passes,
        scratch,
    }
}

/// darktable `xtrans_markesteijn_interpolate`, one pass, in four directions, with
/// darktable's linear VNG within 12 photosites of the edges.
fn markesteijn(cfa: &Pattern<u32>, input: &ImageDesc, crop: &[u32; 4]) -> Prepared {
    let colors: Vec<u32> = (0..36)
        .map(|position| color(cfa, input, position % 6, position / 6))
        .collect();
    let (hexes, solitary) = hexagons(|col, row| color(cfa, input, col, row));
    let size = [input.width, input.height];
    let pass = |entry_point, step, radius, bindings: &[(u32, Resource)]| Pass {
        entry_point,
        kind: PassKind::Spatial { radius },
        params: Params {
            input,
            crop,
            stride: input.width,
            step,
            colors: &colors,
            hexes: &hexes,
            solitary,
        }
        .bytes(),
        tables: Vec::new(),
        bindings: [(2, Resource::Uniforms)]
            .iter()
            .chain(bindings)
            .map(|&(binding, resource)| Binding::new(binding, resource))
            .collect(),
        size: Some(size),
        // Demosaic runs whole or in tiles of its own, never in strips (decision 14).
        strips: None,
    };
    let plane = |i: usize| (18 + i as u32, Resource::Scratch(i));
    let [r0, r1, r2, r3, aux, work] = [0, 1, 2, 3, 4, 5].map(plane);
    let input_binding = (0, Resource::Input);
    let output_binding = (1, Resource::Output);
    let rgb = [r0, r1, r2, r3];
    let with = |extra: &[(u32, Resource)]| -> Vec<(u32, Resource)> {
        rgb.iter().chain(extra).copied().collect()
    };
    let mut passes = vec![
        pass("xt_border", 0, 1, &[input_binding, output_binding]),
        pass("xt_initial", 0, 0, &with(&[input_binding])),
        pass("xt_green_bounds", 0, 3, &[r0, work]),
        pass("xt_green", 0, 3, &with(&[work])),
    ];
    for step in 0..6 {
        passes.push(pass("xt_solitary", step, 2, &with(&[aux])));
    }
    for step in 0..4 {
        passes.push(pass("xt_red_blue", step, 3, &rgb));
    }
    for step in [0, 2] {
        passes.push(pass("xt_two_by_two", step, 2, &rgb));
    }
    for step in 0..4 {
        passes.push(pass("xt_yuv", step, 0, &with(&[aux])));
        passes.push(pass("xt_derivative", step, 1, &[aux, work]));
    }
    passes.extend([
        pass("xt_homogeneity", 0, 1, &[work, aux]),
        pass("xt_homogeneity_sum", 0, 2, &[aux, work]),
        pass("xt_final", 0, 0, &with(&[work, output_binding])),
    ]);
    let plane_bytes = u64::from(input.width) * u64::from(input.height) * 16;
    Prepared {
        output: output(input, crop),
        passes,
        scratch: vec![Scratch { bytes: plane_bytes }; 6],
    }
}

#[cfg(test)]
mod tests {
    use apertrust_plugin::Camera;

    use super::*;

    fn raw(cfa: Pattern<u32>) -> ImageContext {
        ImageContext {
            camera: Some(Camera {
                make: String::new(),
                model: String::new(),
                alias: String::new(),
                width: 64,
                height: 48,
                cfa: Some(cfa),
                black: Pattern {
                    width: 1,
                    height: 1,
                    values: vec![0.0],
                },
                white: 4095.0,
                white_balance: Some(vec![2.0, 1.0, 1.5]),
                xyz_to_camera: None,
                crop: [2, 2, 60, 44],
                declared: [5, 4, 52, 38],
                exposure_bias: 0.0,
                iso: 0.0,
                linear_response_limit: 1.0,
                lens_correction: None,
                orientation: 0,
            }),
            ..ImageContext::full(52, 38)
        }
    }

    fn bayer() -> Pattern<u32> {
        Pattern {
            width: 2,
            height: 2,
            values: vec![0, 1, 1, 2],
        }
    }

    /// Fujifilm's X-Trans pattern, as rawler reports it for the X-T4.
    fn xtrans() -> Pattern<u32> {
        Pattern {
            width: 6,
            height: 6,
            values: vec![
                1, 1, 0, 1, 1, 2, //
                1, 1, 2, 1, 1, 0, //
                2, 0, 1, 0, 2, 1, //
                1, 1, 2, 1, 1, 0, //
                1, 1, 0, 1, 1, 2, //
                0, 2, 1, 2, 0, 1, //
            ],
        }
    }

    fn mosaic() -> ImageDesc {
        ImageDesc {
            sensor_origin: [2, 2],
            maximum: [2.0, 1.0, 1.5],
            ..ImageDesc::new(60, 44, Format::Mosaic)
        }
    }

    fn resolved(image: &ImageContext) -> ParamValues {
        SCHEMA
            .resolve_over(&Demosaic.defaults(image), &ParamValues::new())
            .unwrap()
    }

    #[test]
    fn the_default_method_follows_the_sensor() {
        let rcd = resolved(&raw(bayer()));
        assert_eq!(
            METHOD_NAMES[rcd.choice("demosaicing_method") as usize],
            "rcd"
        );
        let xtrans = resolved(&raw(xtrans()));
        assert_eq!(
            METHOD_NAMES[xtrans.choice("demosaicing_method") as usize],
            "markesteijn"
        );
        // A Bayer sensor of four colours, and a raw with none, as darktable's sraw.
        let mut four = bayer();
        four.values = vec![0, 1, 3, 2];
        let vng4 = resolved(&raw(four));
        assert_eq!(
            METHOD_NAMES[vng4.choice("demosaicing_method") as usize],
            "vng4"
        );
        // Whatever the pattern's size, as darktable's `DT_IMAGE_4BAYER` follows its colours.
        let wide = Pattern {
            width: 2,
            height: 4,
            values: vec![0, 1, 3, 2, 1, 0, 2, 3],
        };
        let vng4 = resolved(&raw(wide));
        assert_eq!(
            METHOD_NAMES[vng4.choice("demosaicing_method") as usize],
            "vng4"
        );
        let mut mono = raw(bayer());
        mono.camera.as_mut().unwrap().cfa = None;
        let mono = resolved(&mono);
        assert_eq!(
            METHOD_NAMES[mono.choice("demosaicing_method") as usize],
            "monochrome"
        );
        assert_eq!(METHODS.len(), METHOD_NAMES.len());
        for (&(name, _), &choice) in METHODS.iter().zip(METHOD_NAMES) {
            assert_eq!(name, choice);
        }
    }

    #[test]
    fn rcd_writes_the_declared_crop_from_planes_of_the_whole_mosaic() {
        let image = raw(bayer());
        let prepared = Demosaic
            .prepare(&resolved(&image), &image, &mosaic())
            .unwrap();
        assert_eq!(
            prepared.output,
            ImageDesc {
                format: Format::Rgba,
                sensor_origin: [5, 4],
                ..ImageDesc {
                    maximum: [2.0, 1.0, 1.5],
                    ..ImageDesc::new(52, 38, Format::Mosaic)
                }
            }
        );
        assert_eq!(prepared.passes.len(), 11);
        assert!(prepared.passes.iter().all(|p| p.size == Some([60, 44])));
        assert_eq!(prepared.scratch.len(), RCD_PLANES);
        assert_eq!(prepared.scratch[0].bytes, 60 * 44 * 4);
        assert_eq!(prepared.scratch[2].bytes, 60 * 44 * 2);

        let bytes = &prepared.passes[0].params;
        let word = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        let float = |at: usize| f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        assert_eq!([word(0), word(4), word(8)], [60, 44, 60]);
        // The declared crop at (5, 4) on the sensor is (3, 2) in the mosaic at (2, 2).
        assert_eq!([word(16), word(20), word(24), word(28)], [3, 2, 52, 38]);
        assert_eq!([float(32), float(36)], [0.5, 2.0]);
        // From (2, 2) on the sensor, the pattern starts at red again.
        assert_eq!([word(48), word(52), word(56), word(60)], [0, 1, 1, 2]);
    }

    #[test]
    fn the_pattern_follows_the_mosaics_place_on_the_sensor() {
        let image = raw(bayer());
        let odd = ImageDesc {
            sensor_origin: [3, 2],
            ..mosaic()
        };
        let prepared = Demosaic.prepare(&resolved(&image), &image, &odd).unwrap();
        let bytes = &prepared.passes[0].params;
        let colors: Vec<u32> = (48..64)
            .step_by(4)
            .map(|at| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()))
            .collect();
        assert_eq!(colors, [1, 0, 2, 1]);
    }

    #[test]
    fn each_hexagon_finds_greens_around_the_solitary_green() {
        let cfa = xtrans();
        let input = ImageDesc::new(60, 44, Format::Mosaic);
        let at = |col: i64, row: i64| color(&cfa, &input, col, row);
        let (hexes, solitary) = hexagons(at);
        // The green with red or blue on all four sides.
        assert_eq!(solitary, (2, 2));
        for row in 0..3 {
            for col in 0..3 {
                if at(col, row) == 1 {
                    continue;
                }
                // Red and blue take their green from the hexagon's first six.
                for &(dx, dy) in &hexes[row as usize][col as usize][..6] {
                    assert_eq!(
                        at(col + i64::from(dx), row + i64::from(dy)),
                        1,
                        "({col}, {row})"
                    );
                }
            }
        }
        // Where the mosaic starts one row lower on the sensor, everything moves up.
        let lower = ImageDesc {
            sensor_origin: [0, 1],
            ..input
        };
        let (_, solitary) = hexagons(|col, row| color(&cfa, &lower, col, row));
        assert_eq!(solitary, (2, 1));
    }

    #[test]
    fn markesteijn_runs_its_steps_over_the_whole_mosaic() {
        let image = raw(xtrans());
        let prepared = Demosaic
            .prepare(&resolved(&image), &image, &mosaic())
            .unwrap();
        assert_eq!(prepared.passes.len(), 27);
        assert!(prepared.passes.iter().all(|p| p.size == Some([60, 44])));
        assert_eq!(prepared.scratch.len(), 6);
        assert_eq!(prepared.output.sensor_origin, [5, 4]);
    }

    #[test]
    fn modes_beyond_the_defaults_are_not_ported() {
        let image = raw(bayer());
        for (name, value) in [
            ("green_eq", Value::Choice(1)),
            ("color_smoothing", Value::Choice(1)),
            ("cs_enabled", Value::Bool(true)),
            ("demosaicing_method", Value::Choice(method("amaze"))),
        ] {
            let mut values = ParamValues::new();
            values.set(name, value);
            let params = SCHEMA
                .resolve_over(&Demosaic.defaults(&image), &values)
                .unwrap();
            assert!(
                Demosaic.prepare(&params, &image, &mosaic()).is_err(),
                "{name}"
            );
        }
    }
}
