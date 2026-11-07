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

//! `apertrust-golden`: render darktable references, check Apertrust's kernels against
//! them, time the kernels, and check gradient kernels against finite differences.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail, ensure};
use apertrust_engine::compute::{GpuImage, GpuTimer, ImageKernel};
use apertrust_engine::edit::{self, Edit};
use apertrust_engine::export::{BUDGET, Tiling, export_at};
use apertrust_engine::gpu::{GpuContext, GpuOptions};
use apertrust_engine::pipeline::{self, Derivative, Instance, Pipe, Pipeline, Resample};
use apertrust_engine::resample::input_length;
use apertrust_operations::demosaic::METHODS;
use apertrust_operations::rawprepare::BLACK;
use apertrust_operations::{agx, channelmixerrgb, colorout, exposure, grain};
use apertrust_plugin::{Format, ImageContext, ImageDesc, ParamValues, Stage, Value};
use apertrust_raw::Pixels;
use clap::{Parser, Subcommand};
use golden::cases::{self, CASES, Case};
use golden::darktable::{DarktableCli, Size};
use golden::delta_e::{self, Stats, Tolerance};
use golden::ports::KernelInputs;
use golden::raw_stage::{RawStage, Skip, SkipKind, skip};
use golden::{heatmap, pfm::Image, ports, raw_stage, render, synthetic};

/// Largest relative difference between a gradient kernel and finite differences that
/// still passes.
const GRADIENT_TOLERANCE: f64 = 1e-2;
/// Largest per-pixel difference between forward and reverse mode that still passes,
/// relative to reverse mode's. Both are exact derivatives of the same function, so they
/// should differ only by rounding.
const MODE_TOLERANCE: f64 = 1e-4;

#[derive(Debug, Parser)]
#[command(version, about = "Golden-image harness against darktable-cli")]
struct Cli {
    #[command(subcommand)]
    command: Command,
    /// Switch on a debug domain, as darktable's -d, on stderr: gpu, raw, pipe, perf,
    /// memory or golden; `all` for every one, `verbose` for more detail. Repeat for
    /// more. With a domain on, a banner naming the build comes first.
    #[arg(short = 'd', long = "debug", global = true, value_name = "DOMAIN")]
    debug: Vec<String>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// List the cases.
    List,
    /// Render (or reuse cached) references for the named cases, or all of them.
    Render { cases: Vec<String> },
    /// Run Apertrust's kernels for the named cases, or all of them, check each against
    /// darktable's expected render, then time them.
    Run {
        cases: Vec<String>,
        /// Timed dispatches in each round; at most 2048 with GPU timestamps.
        #[arg(long, default_value_t = 20)]
        iterations: u32,
        /// Timing rounds.
        #[arg(long, default_value_t = 3)]
        rounds: u32,
        /// Passes of the per-pixel function in each timed dispatch. Above 1, arithmetic
        /// rather than memory dominates, so code quality shows in the timings.
        /// Correctness is always checked with a single pass.
        #[arg(long, default_value_t = 1)]
        repeat: u32,
        /// Milliseconds of untimed dispatches before timing, so the GPU is back at full
        /// clocks after the idle ΔE step.
        #[arg(long, default_value_t = 500)]
        warmup_ms: u64,
        /// Write a difference heatmap PNG per kernel into this directory.
        #[arg(long)]
        heatmaps: Option<PathBuf>,
    },
    /// Check each Slang kernel's derivative entry points, with the sum of the output's
    /// RGB as the loss: reverse mode against central finite differences of the forward
    /// pass, and against forward mode.
    Gradient {
        cases: Vec<String>,
        /// Step of the finite differences, in each parameter's own units, for every
        /// case: by default 1e-3, or the case's own (`ports::gradient_step`).
        #[arg(long)]
        step: Option<f32>,
        /// Show how the gap between reverse mode and finite differences is spread
        /// across pixels.
        #[arg(long)]
        explain: bool,
        /// Run on a synthetic hue, saturation and level sweep instead of the case's
        /// image, keeping the case's parameters. Needs no darktable.
        #[arg(long)]
        synthetic: bool,
    },
    /// Check darktable's raw stage against rawler's decode with demosaic's
    /// photosite-colour passthrough, which needs no port: crop alignment, black and
    /// white scaling, white balance and an identity colorin. Images are paths or names
    /// in APERTRUST_TEST_IMAGES; all the raws there by default.
    RawPassthrough { images: Vec<PathBuf> },
    /// Run the raw stage's ported operations and colorin on the GPU with their defaults
    /// for each image, and compare the result with darktable's render of the same
    /// parameters over the declared crop. Images as for raw-passthrough.
    RawRun {
        images: Vec<PathBuf>,
        /// Set temperature's preset to "as shot" on both sides, so colorin applies no
        /// late correction and highlight reconstruction does not weigh by it. The
        /// default, "as shot to reference" wherever the camera has a matrix, applies both.
        #[arg(long)]
        as_shot: bool,
    },
    /// Compare darktable's own raw-stage defaults for each image, recovered from its
    /// renders without the operation in the history, with Apertrust's: rawprepare's
    /// crop, black levels and white point, and temperature's white balance. Images as
    /// for raw-passthrough.
    RawDefaults { images: Vec<PathBuf> },
    /// Phase 1's exit test (decisions 22, 23 and 62): each raw through Apertrust's
    /// whole pipeline with its defaults, as `apertrust-cli export` renders it before
    /// rounding to 8 bits, and through darktable-cli with Apertrust's crop, levels and
    /// parameters written into its history, compared by CIEDE2000 in the output profile
    /// within the declared image. Needs the raw.pixls.us corpus, not darktable's
    /// integration images: test-images/raw.pixls.us (tools/fetch-raw-corpus.sh), or
    /// APERTRUST_RAW_CORPUS. Images are paths or corpus names, MAKE/MODEL/FILE; every
    /// sample in the corpus's manifest by default. Only the samples the harness expects
    /// to skip may skip.
    ///
    /// With --width or --height, an export at a size instead (decision 56): both sides
    /// crop the raw to its declared image, so both scale the same image, Apertrust as
    /// `apertrust-cli export` sizes and resamples it and darktable-cli with the same
    /// flags, its finalscale before colorout, compared at the same size and reported
    /// without a bound.
    Pipeline {
        images: Vec<PathBuf>,
        /// Export within this width, as darktable-cli's --width.
        #[arg(long)]
        width: Option<u32>,
        /// Export within this height, as darktable-cli's --height.
        #[arg(long)]
        height: Option<u32>,
        /// High quality resampling at a size, as darktable-cli's --hq: true or false.
        #[arg(long, value_name = "BOOL", default_value_t = true, action = clap::ArgAction::Set)]
        hq: bool,
        /// darktable's side's --hq at a size, true or false, so an export without high
        /// quality can be measured against darktable's with it; --hq's value by default.
        #[arg(long, value_name = "BOOL", action = clap::ArgAction::Set)]
        darktable_hq: Option<bool>,
        /// Allow upscaling at a size, as darktable-cli's --upscale: true or false.
        #[arg(long, value_name = "BOOL", default_value_t = false, action = clap::ArgAction::Set)]
        upscale: bool,
        /// Add a grain, vignetting, tone equalizer, haze removal, colour equalizer, colour
        /// balance rgb, local contrast, bloom or soften case's module to both sides, as
        /// the case's sidecar sets it, grain at darktable's offset for the width it
        /// renders at; repeat for more.
        #[arg(long = "add", value_name = "CASE")]
        adds: Vec<String>,
    },
    /// Render a raw as `apertrust-cli export` does, through the default pipeline and any
    /// `--add` cases' modules, to an 8-bit PNG, and write darktable's history for the same
    /// render, for timing the two against each other (tools/compare-cpu.sh): Apertrust's
    /// crop and levels on both sides, the declared image, as `pipeline` has them. Size
    /// options as for `pipeline`. Time it with `-d perf`: the engine's lines are
    /// `apertrust-cli export`'s. Takes one raw's path, not a corpus name.
    Export {
        image: PathBuf,
        /// The PNG to write.
        #[arg(short, long)]
        output: PathBuf,
        /// Write darktable's history for this render here, a sidecar for darktable-cli.
        #[arg(long, value_name = "FILE")]
        darktable_xmp: Option<PathBuf>,
        /// Export within this width, as darktable-cli's --width.
        #[arg(long)]
        width: Option<u32>,
        /// Export within this height, as darktable-cli's --height.
        #[arg(long)]
        height: Option<u32>,
        /// High quality resampling at a size, as darktable-cli's --hq: true or false.
        #[arg(long, value_name = "BOOL", default_value_t = true, action = clap::ArgAction::Set)]
        hq: bool,
        /// Add a case's module to both sides, as for `pipeline`; repeat for more.
        #[arg(long = "add", value_name = "CASE")]
        adds: Vec<String>,
    },
    /// Compare an image against an expected one and report CIEDE2000.
    Compare {
        expected: PathBuf,
        actual: PathBuf,
        /// Use this case's tolerance instead of darktable's default.
        #[arg(long)]
        case: Option<String>,
        /// Write a difference heatmap PNG here.
        #[arg(long)]
        heatmap: Option<PathBuf>,
    },
}

#[derive(Debug, Clone, Copy)]
struct Timing {
    iterations: u32,
    rounds: u32,
    repeat: u32,
    warmup: Duration,
}

/// The banner's line for the darktable-cli the harness would run: its version, and
/// whether it is the one the references are pinned to.
fn darktable_item() -> apertrust_base::log::Item {
    use golden::darktable::REFERENCE_VERSION;
    const NAME: &str = "darktable-cli";
    match DarktableCli::locate().map(|cli| cli.version()) {
        Err(_) => apertrust_base::log::Item::new(NAME, "not found")
            .detail(format!("references need {REFERENCE_VERSION}")),
        Ok(Err(_)) => apertrust_base::log::Item::new(NAME, "unknown").detail("no version reported"),
        Ok(Ok(version)) if version == REFERENCE_VERSION => {
            apertrust_base::log::Item::new(NAME, version).detail("the reference")
        }
        Ok(Ok(version)) => apertrust_base::log::Item::new(NAME, version)
            .detail(format!("not the reference, {REFERENCE_VERSION}")),
    }
}

/// What the banner and `--version` list the harness as built with and running against.
fn build(program: &apertrust_base::log::Program) -> Vec<apertrust_base::log::Item> {
    vec![
        apertrust_base::log::wgpu(program, apertrust_engine::gpu::backends()),
        apertrust_base::log::rawler(program),
        apertrust_base::log::kernels(program),
        darktable_item(),
        apertrust_base::log::onnx_runtime(),
    ]
}

fn main() -> Result<ExitCode> {
    if apertrust_base::log::version_asked() {
        let program = apertrust_base::program!();
        apertrust_base::log::print_version(program, &build(&program));
        return Ok(ExitCode::SUCCESS);
    }
    let cli = Cli::parse();
    let _logging = apertrust_base::log::init(&cli.debug)?;
    apertrust_base::log::starting(apertrust_base::program!(), build);
    match cli.command {
        Command::List => {
            for case in CASES {
                println!("{:<18} {} on {}", case.name, case.operation, case.image);
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Render { cases } => render(&cases),
        Command::Run {
            cases,
            iterations,
            rounds,
            repeat,
            warmup_ms,
            heatmaps,
        } => run(
            &cases,
            Timing {
                iterations: iterations.max(1),
                rounds: rounds.max(1),
                repeat: repeat.max(1),
                warmup: Duration::from_millis(warmup_ms),
            },
            heatmaps.as_deref(),
        ),
        Command::Gradient {
            cases,
            step,
            explain,
            synthetic,
        } => gradient(&cases, step, explain, synthetic),
        Command::RawPassthrough { images } => raw_passthrough(&images),
        Command::RawRun { images, as_shot } => raw_run(&images, as_shot),
        Command::RawDefaults { images } => raw_defaults(&images),
        Command::Pipeline {
            images,
            width,
            height,
            hq,
            darktable_hq,
            upscale,
            adds,
        } => {
            let size = (width.is_some() || height.is_some()).then_some(Size {
                width: width.unwrap_or(0),
                height: height.unwrap_or(0),
                high_quality: hq,
                upscale,
            });
            let darktable_size = size.map(|size| Size {
                high_quality: darktable_hq.unwrap_or(hq),
                ..size
            });
            let adds = adds
                .iter()
                .map(|name| find(name))
                .collect::<Result<Vec<_>>>()?;
            whole_pipeline(&images, size, darktable_size, &adds)
        }
        Command::Export {
            image,
            output,
            darktable_xmp,
            width,
            height,
            hq,
            adds,
        } => {
            let size = (width.is_some() || height.is_some()).then_some(Size {
                width: width.unwrap_or(0),
                height: height.unwrap_or(0),
                high_quality: hq,
                upscale: false,
            });
            let adds = adds
                .iter()
                .map(|name| find(name))
                .collect::<Result<Vec<_>>>()?;
            export(&image, &output, darktable_xmp.as_deref(), size, &adds)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Compare {
            expected,
            actual,
            case,
            heatmap,
        } => {
            let tolerance = match case {
                Some(name) => find(&name)?.tolerance,
                None => Tolerance::DARKTABLE,
            };
            let expected_image = Image::read(&expected)?;
            let (differences, stats) =
                delta_e::compare(&expected_image, &Image::read(&actual)?, tolerance)?;
            if let Some(path) = heatmap {
                heatmap::write(
                    &path,
                    expected_image.width,
                    expected_image.height,
                    &differences,
                    tolerance,
                )?;
            }
            println!("{}", summary(&stats, tolerance));
            Ok(exit_code(stats.passes(tolerance)))
        }
    }
}

fn render(names: &[String]) -> Result<ExitCode> {
    let darktable = DarktableCli::locate()?;
    let images = render::images_dir()?;

    for case in select(names)? {
        let references = render::render_case(&darktable, case, &images)?;
        // The module's own effect: if this is near zero the case isolates nothing.
        let input = Image::read(&references.input)?;
        let expected = Image::read(&references.expected)?;
        println!("{}", case.name);
        println!("  input:    {}", references.input.display());
        println!("  expected: {}", references.expected.display());
        // A module that moves pixels can change the image's size (decision 16), which is
        // its effect.
        if (input.width, input.height) != (expected.width, expected.height) {
            println!(
                "  module effect: {}×{} to {}×{}",
                input.width, input.height, expected.width, expected.height
            );
            continue;
        }
        let (_, effect) = delta_e::compare(&expected, &input, case.tolerance)?;
        println!(
            "  module effect: mean {:.3}, p95 {:.3}, max {:.3}",
            effect.mean, effect.p95, effect.max
        );
    }
    Ok(ExitCode::SUCCESS)
}

/// A case's references, with its input uploaded to the GPU.
struct CaseImages {
    input: GpuImage,
    expected: Image,
}

impl CaseImages {
    fn load(
        gpu: &GpuContext,
        darktable: &DarktableCli,
        images: &Path,
        case: &Case,
    ) -> Result<Self> {
        let references = render::render_case(darktable, case, images)?;
        Ok(Self {
            input: upload(gpu, &Image::read(&references.input)?)?,
            expected: Image::read(&references.expected)?,
        })
    }
}

/// An RGB image on the GPU, with alpha 1.
fn upload(gpu: &GpuContext, image: &Image) -> Result<GpuImage> {
    let rgba: Vec<[f32; 4]> = image
        .pixels
        .iter()
        .map(|&[r, g, b]| [r, g, b, 1.0])
        .collect();
    let (width, height) = (u32::try_from(image.width)?, u32::try_from(image.height)?);
    Ok(GpuImage::upload(gpu, width, height, &rgba)?)
}

fn run(names: &[String], timing: Timing, heatmaps: Option<&Path>) -> Result<ExitCode> {
    let darktable = DarktableCli::locate()?;
    let images = render::images_dir()?;
    let gpu = pollster::block_on(GpuContext::new(&GpuOptions::default()))?;
    let timer = GpuTimer::new(&gpu, timing.iterations);
    let iterations = timer.as_ref().map_or(timing.iterations, |timer| {
        timing.iterations.min(timer.slots())
    });
    println!(
        "GPU: {} ({} timing)",
        gpu.adapter.get_info().name,
        if timer.is_some() {
            "GPU timestamp"
        } else {
            "wall-clock"
        }
    );
    let mut all_passed = true;

    for case in select(names)? {
        if let Some(values) = ports::plan_values(case)? {
            all_passed &= run_plan(&gpu, &darktable, &images, case, values, timing, heatmaps)?;
            continue;
        }
        let kernel = apertrust_operations::kernel(case.operation);
        let (Some(kernel), Some(inputs)) = (kernel, ports::kernel_inputs(case)?) else {
            println!("{}: not ported yet", case.name);
            continue;
        };
        let tables: Vec<&[u8]> = inputs.tables.iter().map(Vec::as_slice).collect();
        let images = CaseImages::load(&gpu, &darktable, &images, case)?;
        let (width, height) = (images.input.width(), images.input.height());
        let output = GpuImage::new(&gpu, width, height);
        println!("{} ({width}×{height})", case.name);

        // Correctness first: the ΔE step runs on the CPU while the GPU idles, so it stays
        // out of the timing below.
        let compiled = pollster::block_on(ImageKernel::new(&gpu, kernel.name, kernel.wgsl))?;
        compiled.dispatch(&gpu, &images.input, &output, &inputs.params, &tables);
        let pixels = output
            .download(&gpu)?
            .into_iter()
            .map(|[r, g, b, _]| [r, g, b])
            .collect();
        let actual = Image::new(width as usize, height as usize, pixels)?;
        let stats = check(case, &images.expected, &actual, heatmaps)?;
        all_passed &= stats.passes(case.tolerance);

        // Speed: keep the GPU busy for the warm-up so its clocks are up, then time it.
        let dispatch = |slot: Option<(&GpuTimer, u32)>| {
            compiled.dispatch_repeated(
                &gpu,
                &images.input,
                &output,
                &inputs.params,
                &tables,
                timing.repeat,
                slot,
            );
        };
        let warmup_start = Instant::now();
        loop {
            for _ in 0..4 {
                dispatch(None);
            }
            gpu.wait()?;
            if warmup_start.elapsed() >= timing.warmup {
                break;
            }
        }
        let mut times = Vec::new();
        for _ in 0..timing.rounds {
            let start = Instant::now();
            for slot in 0..iterations {
                dispatch(timer.as_ref().map(|timer| (timer, slot)));
            }
            gpu.wait()?;
            match &timer {
                Some(timer) => times.extend(timer.read(&gpu, iterations)?),
                None => times.push(start.elapsed().as_secs_f64() * 1e3 / f64::from(iterations)),
            }
        }

        times.sort_by(f64::total_cmp);
        let per_pass = |ms: f64| ms / f64::from(timing.repeat);
        let median = per_pass(times[times.len() / 2]);
        let min = per_pass(times[0]);
        println!(
            "  {median:>8.3} ms/pass (min {min:.3})  {}",
            summary(&stats, case.tolerance)
        );
    }
    Ok(exit_code(all_passed))
}

fn gradient(
    names: &[String],
    step: Option<f32>,
    explain: bool,
    synthetic: bool,
) -> Result<ExitCode> {
    // darktable is only needed to render a case's own image.
    let references = if synthetic {
        None
    } else {
        Some((DarktableCli::locate()?, render::images_dir()?))
    };
    let gpu = pollster::block_on(GpuContext::new(&GpuOptions::default()))?;
    println!("GPU: {}", gpu.adapter.get_info().name);
    let mut all_passed = true;

    for case in select(names)? {
        let step = step.or(ports::gradient_step(case)).unwrap_or(1e-3);
        if let Some(values) = ports::plan_values(case)? {
            all_passed &= gradient_plan(&gpu, references.as_ref(), case, values, step, explain)?;
            continue;
        }
        let parameters = ports::gradient_parameters(case);
        let kernel = apertrust_operations::kernel(case.operation);
        let (Some(kernel), Some(inputs)) = (kernel, ports::kernel_inputs(case)?) else {
            println!("{}: no Slang kernel", case.name);
            continue;
        };
        if parameters.is_empty() {
            println!("{}: no gradient port", case.name);
            continue;
        }
        let tables: Vec<&[u8]> = inputs.tables.iter().map(Vec::as_slice).collect();
        let (source, describe) = gradient_source(references.as_ref(), case)?;
        let input = upload(&gpu, &source)?;
        let (width, height) = (input.width(), input.height());
        let pixels = source.pixels.len();

        let label = kernel.name;
        let forward = pollster::block_on(ImageKernel::new(&gpu, label, kernel.wgsl))?;
        let derivative = |suffix: &str| {
            let entry_point = format!("{}_{suffix}", case.operation);
            pollster::block_on(ImageKernel::with_entry_point(
                &gpu,
                label,
                kernel.wgsl,
                &entry_point,
            ))
        };
        let reverse_mode = derivative("backward")?;
        let forward_mode = derivative("forward_diff")?;
        let derivative_out = GpuImage::new(&gpu, width, height);

        // Reverse mode: the loss is the sum of the output's RGB, so dLoss/dOutput is 1
        // for each channel; the parameter gradient is the per-pixel gradients summed.
        let ones = GpuImage::upload(&gpu, width, height, &vec![[1.0, 1.0, 1.0, 0.0]; pixels])?;
        reverse_mode.dispatch_derivative(
            &gpu,
            &input,
            &ones,
            &derivative_out,
            &inputs.params,
            &tables,
        );
        let per_pixel_reverse = derivative_out.download(&gpu)?;

        // Forward mode, one parameter at a time: the loss's derivative along the
        // parameter is the sum of the output's RGB derivative.
        let forward_along = |index: usize| -> Result<Vec<f64>> {
            let mut direction = [0.0; 4];
            direction[index] = 1.0;
            let directions = GpuImage::upload(&gpu, width, height, &vec![direction; pixels])?;
            forward_mode.dispatch_derivative(
                &gpu,
                &input,
                &directions,
                &derivative_out,
                &inputs.params,
                &tables,
            );
            Ok(rgb_sums(&derivative_out.download(&gpu)?))
        };

        // Finite differences of the same loss through the forward kernel, per pixel.
        let output = GpuImage::new(&gpu, width, height);
        let loss = |inputs: &KernelInputs| -> Result<Vec<f64>> {
            let tables: Vec<&[u8]> = inputs.tables.iter().map(Vec::as_slice).collect();
            forward.dispatch(&gpu, &input, &output, &inputs.params, &tables);
            Ok(rgb_sums(&output.download(&gpu)?))
        };

        println!(
            "{} ({width}×{height}{}, step {step})",
            case.name,
            if synthetic { " synthetic sweep" } else { "" }
        );
        for (index, name) in parameters.iter().enumerate() {
            if synthetic && let Some(reason) = ports::sweep_blind(case, name) {
                println!("  {name:<18} skipped: sweep-blind, {reason}");
                continue;
            }
            let reverse: Vec<f64> = per_pixel_reverse
                .iter()
                .map(|pixel| f64::from(pixel[index]))
                .collect();
            let forward = forward_along(index)?;
            let plus = loss(&ports::perturbed_inputs(case, index, step)?)?;
            let minus = loss(&ports::perturbed_inputs(case, index, -step)?)?;
            let numeric: Vec<f64> = plus
                .iter()
                .zip(&minus)
                .map(|(p, m)| (p - m) / (2.0 * f64::from(step)))
                .collect();
            let explained = explain.then_some((source.pixels.as_slice(), &*describe));
            all_passed &= report_gradient(name, &reverse, &forward, &numeric, explained);
        }
    }
    Ok(exit_code(all_passed))
}

/// The ΔE2000 step: `actual` against darktable's `expected`, with a heatmap in
/// `heatmaps` when asked.
fn check(case: &Case, expected: &Image, actual: &Image, heatmaps: Option<&Path>) -> Result<Stats> {
    let (differences, stats) = delta_e::compare(expected, actual, case.tolerance)?;
    if let Some(dir) = heatmaps {
        std::fs::create_dir_all(dir)?;
        let path = dir.join(format!("{}.png", case.name));
        heatmap::write(
            &path,
            actual.width,
            actual.height,
            &differences,
            case.tolerance,
        )?;
    }
    Ok(stats)
}

/// A case's one-instance engine plan (decision 15): its operation alone over an image of
/// `input` in `image`, the export pipe at full resolution.
fn case_plan(
    gpu: &GpuContext,
    case: &Case,
    params: ParamValues,
    image: &ImageContext,
    input: ImageDesc,
) -> Result<pipeline::Plan> {
    let operation = apertrust_operations::operation(case.operation)
        .with_context(|| format!("no operation {}", case.operation))?;
    let instances = [Instance { operation, params }];
    let limits = pipeline::Limits::of(gpu);
    Ok(pipeline::plan(
        &instances,
        image,
        input,
        Pipe::Export,
        &limits,
    )?)
}

/// The RGB of a rendered image.
fn rgb(pixels: Vec<[f32; 4]>, width: u32, height: u32) -> Result<Image> {
    let pixels = pixels.into_iter().map(|[r, g, b, _]| [r, g, b]).collect();
    Image::new(width as usize, height as usize, pixels)
}

/// [`run`] for a case whose operation runs as a one-instance plan (decision 15): checked
/// by its render, then timed by whole renders, every pass of the plan in one submission.
/// `--repeat` repeats no pass here. Whether it passes.
fn run_plan(
    gpu: &GpuContext,
    darktable: &DarktableCli,
    images: &Path,
    case: &Case,
    values: ParamValues,
    timing: Timing,
    heatmaps: Option<&Path>,
) -> Result<bool> {
    let images = CaseImages::load(gpu, darktable, images, case)?;
    let (width, height) = (images.input.width(), images.input.height());
    let context = ports::plan_context(case, width, height)?;
    let plan = case_plan(gpu, case, values, &context, images.input.desc())?;
    let passes = plan.passes().count();
    let mut pipeline = pollster::block_on(Pipeline::new(gpu, plan, images.input))?;
    println!(
        "{} ({width}×{height}, a plan of {passes} passes)",
        case.name
    );
    pipeline.render(gpu)?;
    // The output's own size, which an operation that moves pixels changes (decision 16).
    let output = pipeline.output();
    let (out_width, out_height) = (output.width(), output.height());
    let actual = rgb(output.download(gpu)?, out_width, out_height)?;
    let stats = check(case, &images.expected, &actual, heatmaps)?;

    // Speed: renders, each timed by its passes' GPU timestamps, or by the wall clock
    // without them.
    let mut render = || -> Result<f64> {
        let timings = pipeline.render(gpu)?;
        let gpu_ms: f64 = timings.passes.iter().map(|(_, ms)| ms).sum();
        Ok(if gpu_ms.is_nan() {
            timings.wall_ms
        } else {
            gpu_ms
        })
    };
    let warmup_start = Instant::now();
    while warmup_start.elapsed() < timing.warmup {
        render()?;
    }
    let mut times = Vec::new();
    for _ in 0..timing.rounds * timing.iterations {
        times.push(render()?);
    }
    times.sort_by(f64::total_cmp);
    println!(
        "  {:>8.3} ms/render (min {:.3})  {}",
        times[times.len() / 2],
        times[0],
        summary(&stats, case.tolerance)
    );
    Ok(stats.passes(case.tolerance))
}

/// How a pixel of the gradient check's image is named.
type Describe = Box<dyn Fn(usize) -> String>;

/// An image's pixels and how a pixel of it is named, for explaining a gradient's gap.
type Explained<'a> = (&'a [[f32; 3]], &'a dyn Fn(usize) -> String);

/// The gradient check's image: the case's input as darktable renders it, or the
/// synthetic sweep, with how to name a pixel of it.
fn gradient_source(
    references: Option<&(DarktableCli, PathBuf)>,
    case: &Case,
) -> Result<(Image, Describe)> {
    Ok(match references {
        None => (synthetic::sweep(), Box::new(synthetic::describe)),
        Some((darktable, images)) => {
            let image = Image::read(&render::render_case(darktable, case, images)?.input)?;
            let width = image.width;
            let describe = move |i| format!("pixel ({}, {})", i % width, i / width);
            (image, Box::new(describe))
        }
    })
}

/// [`gradient`] for a case whose operation runs as a one-instance plan (decision 15): its
/// output pass's derivative entry points, over the scratch its render left, against
/// finite differences of whole renders. Whether every parameter passes.
fn gradient_plan(
    gpu: &GpuContext,
    references: Option<&(DarktableCli, PathBuf)>,
    case: &Case,
    values: ParamValues,
    step: f32,
    explain: bool,
) -> Result<bool> {
    let parameters = ports::gradient_parameters(case);
    if parameters.is_empty() {
        println!("{}: no gradient port", case.name);
        return Ok(true);
    }
    let (source, describe) = gradient_source(references, case)?;
    let input = upload(gpu, &source)?;
    let (width, height, desc) = (input.width(), input.height(), input.desc());
    let pixels = source.pixels.len();
    // The synthetic sweep is no raw: its radius is measured on itself.
    let context = match references {
        Some(_) => ports::plan_context(case, width, height)?,
        None => ImageContext::full(width, height),
    };
    let mut pipeline = pollster::block_on(Pipeline::new(
        gpu,
        case_plan(gpu, case, values, &context, desc)?,
        input,
    ))?;
    pipeline.render(gpu)?;
    let derivative_out = GpuImage::new(gpu, width, height);

    // Reverse mode: dLoss/dOutput is 1 for each channel of every pixel.
    let ones = GpuImage::upload(gpu, width, height, &vec![[1.0, 1.0, 1.0, 0.0]; pixels])?;
    pollster::block_on(pipeline.derivative(gpu, Derivative::Backward, &ones, &derivative_out))?;
    let per_pixel_reverse = derivative_out.download(gpu)?;
    // Forward mode, one parameter at a time, before the finite differences move them.
    let mut forward = Vec::new();
    for index in 0..parameters.len() {
        let mut direction = [0.0; 4];
        direction[index] = 1.0;
        let directions = GpuImage::upload(gpu, width, height, &vec![direction; pixels])?;
        pollster::block_on(pipeline.derivative(
            gpu,
            Derivative::ForwardDiff,
            &directions,
            &derivative_out,
        ))?;
        forward.push(rgb_sums(&derivative_out.download(gpu)?));
    }

    println!(
        "{} ({width}×{height}{}, step {step}, a plan of {} passes)",
        case.name,
        if references.is_none() {
            " synthetic sweep"
        } else {
            ""
        },
        pipeline.plan().passes().count()
    );
    let mut passed = true;
    for (index, name) in parameters.iter().enumerate() {
        let mut losses = Vec::new();
        for delta in [step, -step] {
            let values = ports::perturbed_values(case, index, delta)?;
            let plan = case_plan(gpu, case, values, &context, desc)?;
            pipeline = pollster::block_on(pipeline.update(gpu, plan))?;
            pipeline.render(gpu)?;
            losses.push(rgb_sums(&pipeline.output().download(gpu)?));
        }
        let numeric: Vec<f64> = losses[0]
            .iter()
            .zip(&losses[1])
            .map(|(p, m)| (p - m) / (2.0 * f64::from(step)))
            .collect();
        let reverse: Vec<f64> = per_pixel_reverse
            .iter()
            .map(|pixel| f64::from(pixel[index]))
            .collect();
        let explained = explain.then_some((source.pixels.as_slice(), &*describe));
        passed &= report_gradient(name, &reverse, &forward[index], &numeric, explained);
    }
    Ok(passed)
}

/// One parameter's gradient check, printed: reverse mode summed over the pixels against
/// central finite differences, and against forward mode pixel by pixel, the gap
/// explained over `explain`'s input when given. Whether both pass.
fn report_gradient(
    name: &str,
    reverse: &[f64],
    forward: &[f64],
    numeric: &[f64],
    explain: Option<Explained>,
) -> bool {
    if let Some((input, describe)) = explain {
        explain_gap(input, describe, reverse, forward, numeric);
    }
    let (reverse_total, forward_total, numeric_total) = (
        reverse.iter().sum::<f64>(),
        forward.iter().sum::<f64>(),
        numeric.iter().sum::<f64>(),
    );
    let error = relative_error(reverse_total, numeric_total);
    // Per pixel, so differences of opposite sign do not cancel.
    let modes_apart = reverse
        .iter()
        .zip(forward)
        .map(|(r, f)| (r - f).abs())
        .sum::<f64>()
        / reverse
            .iter()
            .map(|r| r.abs())
            .sum::<f64>()
            .max(f64::MIN_POSITIVE);
    let passed = error <= GRADIENT_TOLERANCE;
    let modes_agree = modes_apart <= MODE_TOLERANCE;
    println!(
        "  {name:<18} reverse {reverse_total:>13.6e}  finite differences {numeric_total:>13.6e}  relative error {error:.1e}  {}",
        pass_fail(passed)
    );
    println!(
        "  {:<18} forward {forward_total:>13.6e}  per-pixel difference from reverse {modes_apart:.1e}  {}",
        "",
        pass_fail(modes_agree)
    );
    passed && modes_agree
}

/// Largest difference from the CPU computation that still passes the passthrough
/// check: both sides do the same f32 arithmetic, so only rounding should differ.
const PASSTHROUGH_TOLERANCE: f32 = 1e-5;

/// A raw to check, and what its check is expected to come to.
#[derive(Debug, Clone, PartialEq)]
struct Raw {
    path: PathBuf,
    expected: Expected,
}

/// What a raw's check is expected to come to.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Expected {
    /// The raw-stage checks: any raw may skip, for any kind of [`Skip`]; other errors fail.
    AnySkip,
    /// `pipeline`: the raw must be checked.
    Checked,
    /// `pipeline`: one of [`EXPECTED_SKIPS`], skipped for this reason.
    Skip(SkipKind),
}

/// A name as a path when something is there, then as a name in `dir`, and otherwise as
/// given, so an error names what was asked for.
fn resolve(name: &Path, dir: Option<&Path>) -> PathBuf {
    if name.exists() {
        return name.to_owned();
    }
    match dir.map(|dir| dir.join(name)) {
        Some(path) if path.exists() => path,
        _ => name.to_owned(),
    }
}

/// darktable's integration-test images to check: paths, or names in
/// APERTRUST_TEST_IMAGES; every raw at its top level if none is named. Any may be
/// skipped.
fn test_images(names: &[PathBuf]) -> Result<Vec<Raw>> {
    let paths = if names.is_empty() {
        let raw = |path: &Path| {
            path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
                [
                    "cr2", "cr3", "crw", "nef", "arw", "raf", "orf", "rw2", "dng", "pef", "srw",
                ]
                .contains(&e.to_ascii_lowercase().as_str())
            })
        };
        let mut found: Vec<PathBuf> = std::fs::read_dir(render::images_dir()?)?
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|path| raw(path))
            .collect();
        found.sort();
        found
    } else {
        let dir = render::images_dir().ok();
        names
            .iter()
            .map(|name| resolve(name, dir.as_deref()))
            .collect()
    };
    Ok(paths
        .into_iter()
        .map(|path| Raw {
            path,
            expected: Expected::AnySkip,
        })
        .collect())
}

/// The raw.pixls.us corpus's manifest, which `tools/fetch-raw-corpus.sh` fetches.
const MANIFEST: &str = include_str!("../../raw/tests/corpus/files.sha256");

/// The corpus samples `pipeline` expects to skip, by name, and why. Any other sample
/// that skips fails the run, as does one of these skipping for another reason.
const EXPECTED_SKIPS: &[(&str, SkipKind)] = &[
    // rawler 0.8 cannot decode them.
    ("GoPro/HERO5 Black/GOPR0020.GPR", SkipKind::Undecodable),
    (
        "Hasselblad/CFV-50c/20200215_BERLIN_OldNew_WestHafen_B0000420.3FR",
        SkipKind::Undecodable,
    ),
    ("Sigma/DP1 Merrill/SDIM0555.X3F", SkipKind::Undecodable),
    (
        "Nikon/Z 9/Nikon_-_NIKON_Z_9_-_14bit_compressed_(Lossy_High_Efficiency).NEF",
        SkipKind::Undecodable,
    ),
    // Linear or monochrome raws, which the pipeline does not take yet.
    (
        "Adobe DNG Converter/Canon EOS 5D Mark III/5G4A9394-compressed-lossy.DNG",
        SkipKind::NotColourMosaic,
    ),
    (
        "Apple/iPhone 12 Pro/IMG_1361.DNG",
        SkipKind::NotColourMosaic,
    ),
    (
        "Canon/EOS 5D Mark II/09.canon.sraw1.cr2",
        SkipKind::NotColourMosaic,
    ),
    (
        "Sony/ILCE-7RM5/7RM5-LosslessCompressedSmall.ARW",
        SkipKind::NotColourMosaic,
    ),
    (
        "Leica/M Monochrom (Typ 246)/M2462362.DNG",
        SkipKind::NotColourMosaic,
    ),
    // darktable-cli's loader rejects them.
    (
        "Phase One/IQ3 100MP/Phase_One_IQ3_100MP_IIQ_S.IIQ",
        SkipKind::DarktableCannotLoad,
    ),
    (
        "Fujifilm/X-E4/fuji_xe4_32_lossy.RAF",
        SkipKind::DarktableCannotLoad,
    ),
];

/// The sample names a `sha256sum` manifest lists, `<Make>/<Model>/<file>`, sorted.
fn manifest_names(manifest: &str) -> Result<Vec<&str>> {
    let mut names = manifest
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
        .map(|line| {
            let parsed = line.split_once(' ').and_then(|(digest, name)| {
                // Two spaces, or a space and sha256sum's binary-mode `*`.
                let name = name.strip_prefix([' ', '*'])?;
                let hex = digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit());
                (hex && !name.is_empty()).then_some(name)
            });
            parsed.with_context(|| format!("not a sha256sum line: {line:?}"))
        })
        .collect::<Result<Vec<_>>>()?;
    names.sort_unstable();
    Ok(names)
}

/// `path`'s name in the corpus, `<Make>/<Model>/<file>`, if it lies inside it.
fn corpus_name(path: &Path, corpus: &Path) -> Option<String> {
    let relative = path
        .strip_prefix(corpus)
        .map(Path::to_path_buf)
        .ok()
        .or_else(|| {
            let (path, corpus) = (path.canonicalize().ok()?, corpus.canonicalize().ok()?);
            path.strip_prefix(&corpus).map(Path::to_path_buf).ok()
        })?;
    let parts: Vec<_> = relative
        .components()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect();
    Some(parts.join("/"))
}

/// What a corpus sample's check is expected to come to.
fn expected_in_corpus(name: Option<&str>) -> Expected {
    EXPECTED_SKIPS
        .iter()
        .find(|(listed, _)| Some(*listed) == name)
        .map_or(Expected::Checked, |&(_, kind)| Expected::Skip(kind))
}

/// The corpus samples `pipeline` checks: paths, or names in the corpus,
/// `<Make>/<Model>/<file>`; every sample its manifest lists if none is named, those
/// missing from disk included, so they fail rather than vanish. Each must be checked
/// unless [`EXPECTED_SKIPS`] lists it.
fn corpus_images(names: &[PathBuf]) -> Result<Vec<Raw>> {
    let corpus = render::corpus_dir().ok();
    let paths: Vec<PathBuf> = if names.is_empty() {
        let dir = corpus.clone().map_or_else(render::corpus_dir, Ok)?;
        manifest_names(MANIFEST)?
            .into_iter()
            .map(|name| dir.join(name))
            .collect()
    } else {
        names
            .iter()
            .map(|name| resolve(name, corpus.as_deref()))
            .collect()
    };
    Ok(paths
        .into_iter()
        .map(|path| {
            let name = corpus.as_deref().and_then(|dir| corpus_name(&path, dir));
            let expected = expected_in_corpus(name.as_deref());
            Raw { path, expected }
        })
        .collect())
}

/// How one raw's result counts.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Verdict {
    Pass,
    Fail,
    Skip,
}

/// What a raw's check came to, given what was expected of it: the verdict, the line to
/// print after it, and a note when [`EXPECTED_SKIPS`] lists a raw that was checked.
fn judge(result: Result<(bool, String)>, expected: Expected) -> (Verdict, String, Option<String>) {
    let stale = match expected {
        Expected::Skip(kind) if result.is_ok() => Some(format!(
            "expected to skip ({kind}) but checked: its entry in the expected skips is stale"
        )),
        _ => None,
    };
    let (verdict, line) = match result {
        Ok((true, line)) => (Verdict::Pass, line),
        Ok((false, line)) => (Verdict::Fail, line),
        Err(error) => match (error.downcast_ref::<Skip>().map(|skip| skip.kind), expected) {
            (Some(_), Expected::AnySkip) => (Verdict::Skip, format!("{error:#}")),
            (Some(kind), Expected::Skip(listed)) if kind == listed => {
                (Verdict::Skip, format!("{error:#}"))
            }
            (Some(_), Expected::Skip(listed)) => (
                Verdict::Fail,
                format!("skipped for another reason than the expected one ({listed}): {error:#}"),
            ),
            (Some(_), Expected::Checked) => {
                (Verdict::Fail, format!("skipped unexpectedly: {error:#}"))
            }
            (None, _) => (Verdict::Fail, format!("{error:#}")),
        },
    };
    (verdict, line, stale)
}

/// A run's counts.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct Tally {
    passed: usize,
    failed: usize,
    skipped: usize,
}

impl Tally {
    /// A run passes when it checked at least one raw and none failed.
    fn passes(&self) -> bool {
        self.failed == 0 && self.passed > 0
    }
}

/// Runs `check` on each raw and prints its line, judged by [`judge`], then the counts.
fn each_raw(raws: &[Raw], check: impl FnMut(&Path) -> Result<(bool, String)>) -> Tally {
    each_raw_as(raws, Held::Passed, check)
}

/// What a raw's check that holds is called: it passed a bound, or it was measured, with
/// no bound to pass yet (decision 56's export at a size).
#[derive(Debug, Clone, Copy, PartialEq)]
enum Held {
    Passed,
    Measured,
}

/// [`each_raw`], a check that holds called as `held` says.
fn each_raw_as(
    raws: &[Raw],
    held: Held,
    mut check: impl FnMut(&Path) -> Result<(bool, String)>,
) -> Tally {
    let (word, counted) = match held {
        Held::Passed => ("PASS", "passed"),
        Held::Measured => ("measured", "measured"),
    };
    let mut tally = Tally::default();
    for raw in raws {
        let name = raw
            .path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into());
        let (verdict, line, note) = judge(check(&raw.path), raw.expected);
        let word = match verdict {
            Verdict::Pass => {
                tally.passed += 1;
                word
            }
            Verdict::Fail => {
                tally.failed += 1;
                "FAIL"
            }
            Verdict::Skip => {
                tally.skipped += 1;
                "skipped,"
            }
        };
        println!("{name}: {word} {line}");
        if let Some(note) = note {
            println!("{name}: note: {note}");
        }
    }
    println!(
        "{} checked: {} {counted}, {} failed; {} skipped",
        tally.passed + tally.failed,
        tally.passed,
        tally.failed,
        tally.skipped
    );
    tally
}

fn raw_passthrough(names: &[PathBuf]) -> Result<ExitCode> {
    let darktable = DarktableCli::locate()?;
    let tally = each_raw(&test_images(names)?, |image| {
        let raw = raw_stage::decode(image)?;
        let metadata = raw_stage::Metadata::of(&raw)?;
        let xmp = raw_stage::passthrough_xmp(&metadata.passthrough(&raw))?;
        let expected = raw_stage::expected_passthrough(&raw, &metadata)?;
        let actual = Image::read(&raw_stage::render(&darktable, image, &xmp, &[])?)?;
        let found = raw_stage::compare(&expected, &actual, 4, PASSTHROUGH_TOLERANCE);
        let aligned = found.offset == (0, 0)
            && (actual.width, actual.height) == (expected.width, expected.height);
        let passed = aligned && found.max_abs <= PASSTHROUGH_TOLERANCE;
        Ok((
            passed,
            format!(
                "{}: darktable {}×{}, rawler's active area {}×{}; offset {:?}, max |Δ| {:.1e}, mean {:.1e}, {:.2}% of pixels over {PASSTHROUGH_TOLERANCE:.0e}",
                raw.camera.model,
                actual.width,
                actual.height,
                expected.width,
                expected.height,
                found.offset,
                found.max_abs,
                found.mean_abs,
                100.0 * found.over as f64 / (found.compared.0 * found.compared.1).max(1) as f64
            ),
        ))
    });
    Ok(exit_code(tally.passes()))
}

/// The raw stage's operations, each with its defaults for the raw resolved, as the
/// engine plans them.
fn raw_stage_defaults(raw: &apertrust_raw::RawImage) -> Result<Vec<Instance<'static>>> {
    let mut instances = all_defaults(&apertrust_engine::raw::image_context(raw))?;
    // The raw stage and the input profile, which `raw-run` compares with darktable.
    instances.retain(|instance| {
        let manifest = instance.operation.manifest();
        manifest.slot.stage == Stage::Raw || manifest.id == "colorin"
    });
    Ok(instances)
}

/// The parameters of the instance of operation `id`.
fn params_of<'a>(instances: &'a [Instance], id: &str) -> Result<&'a ParamValues> {
    instances
        .iter()
        .find(|instance| instance.operation.manifest().id == id)
        .map(|instance| &instance.params)
        .with_context(|| format!("no {id} among the operations checked"))
}

/// What darktable's history gets for the raw stage and colorin, from Apertrust's
/// resolved instances: the crop and levels, the white balance and its preset, highlight
/// reconstruction, demosaic's method and darktable's own input profile for the image.
fn darktable_stage(
    image: &Path,
    metadata: &raw_stage::Metadata,
    instances: &[Instance],
) -> Result<RawStage> {
    let rawprepare = params_of(instances, "rawprepare")?;
    // darktable's demosaic method, as darktable numbers it, when demosaic is ported.
    let demosaic = instances
        .iter()
        .find(|instance| instance.operation.manifest().id == "demosaic")
        .map(|instance| METHODS[instance.params.choice("demosaicing_method") as usize].1);
    let temperature = params_of(instances, "temperature")?;
    // darktable's own input profile for the image: the file's matrix for a DNG.
    let dng = image
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("dng"));
    let colorin = instances
        .iter()
        .any(|instance| instance.operation.manifest().id == "colorin")
        .then_some(if dng {
            raw_stage::EMBEDDED_MATRIX
        } else {
            raw_stage::STANDARD_MATRIX
        });
    // Highlight reconstruction's method and threshold, when it is ported.
    let highlights = instances
        .iter()
        .find(|instance| instance.operation.manifest().id == "highlights")
        .map(|instance| {
            (
                instance.params.choice("mode") as i32,
                instance.params.float("clip"),
            )
        });
    Ok(RawStage {
        demosaic,
        // Temperature's preset as Apertrust has it, so "as shot to reference" asks
        // colorin for the late correction on both sides.
        temperature_preset: METHODS_PRESET[temperature.choice("preset") as usize],
        colorin,
        highlights,
        ..RawStage::passthrough(
            Some(raw_stage::rawprepare_from(rawprepare)),
            Some(coefficients(temperature)),
            metadata.is_xtrans(),
        )
    })
}

/// darktable's temperature presets' values, in the order of Apertrust's choices.
const METHODS_PRESET: [i32; 5] = [0, 1, 2, 3, 4];

/// Temperature's coefficients for red, green and blue.
fn coefficients(params: &ParamValues) -> [f32; 3] {
    ["red", "green", "blue"].map(|name| params.float(name))
}

/// darktable's PNG writer at 16 bits a channel.
const PNG16: &str = "plugins/imageio/format/png/bpp=16";

/// Phase 1's exit tolerance (decision 62, agreed with the user): a mean CIEDE2000 ten
/// times what the pipeline gives where the decoders agree, and at most this share of
/// pixels over darktable's own maximum, where rounding tips a demosaic direction choice.
const PIPELINE_MEAN: f64 = 0.01;
const PIPELINE_OVER: f64 = 1e-4;

/// The default pipeline, what an empty edit renders, each operation with its defaults for
/// the raw resolved, as `apertrust-cli export` renders a raw without an edit.
fn all_defaults(image: &ImageContext) -> Result<Vec<Instance<'static>>> {
    let edit = Edit::default();
    edit::instances(
        &edit,
        apertrust_operations::OPERATIONS,
        apertrust_modules::MODULES,
        image,
    )
    .all()?
    .into_iter()
    .map(|Instance { operation, params }| {
        let params = operation
            .schema()
            .resolve_over(&operation.defaults(image), &params)?;
        Ok(Instance { operation, params })
    })
    .collect()
}

/// A conversion of an image's pixels to CIELAB.
type ToLab = fn([f32; 3]) -> [f64; 3];

/// darktable-cli's `--icc-type` for colorout's output profile, and how CIEDE2000 reads
/// the pixels both sides write in it. Any other profile is refused.
fn output_profile(colorout: &ParamValues) -> Result<(&'static str, ToLab)> {
    let profile = colorout::PROFILES[colorout.choice("type") as usize];
    Ok(match profile {
        "srgb" => ("SRGB", delta_e::srgb_to_lab),
        "lin_rec2020" => ("LIN_REC2020", delta_e::rec2020_to_lab),
        "lin_rec709" => ("LIN_REC709", delta_e::rec709_to_lab),
        other => bail!("the whole-pipeline test cannot compare colorout's {other} profile"),
    })
}

/// The default pipeline with rawprepare cropping the raw to its declared image, which
/// darktable's processed image then is too, so an export at a size scales the same image
/// on both sides.
fn cropped_to_declared(instances: &mut [Instance], camera: &apertrust_plugin::Camera) {
    let [x, y, width, height] = camera.declared;
    let margins = [
        ("left", x),
        ("top", y),
        ("right", camera.width - x - width),
        ("bottom", camera.height - y - height),
    ];
    for instance in instances
        .iter_mut()
        .filter(|instance| instance.operation.manifest().id == "rawprepare")
    {
        for (name, margin) in margins {
            instance.params.set(name, Value::Int(i64::from(margin)));
        }
    }
}

/// Apertrust's resample to darktable-cli's `size` of an image of `extent`, as
/// `apertrust-cli export` sizes a PNG; `None` at full size.
fn resample_to(size: Size, extent: [u32; 2]) -> Result<Option<Resample>> {
    let request = apertrust_export::size::Request {
        width: size.width,
        height: size.height,
        scale: None,
        upscale: size.upscale,
        high_quality: size.high_quality,
        dialog: false,
    };
    let format = apertrust_export::Format::Png(apertrust_export::png::Options::default());
    let size = apertrust_export::size::size(&request, &format, extent)?;
    Ok(size.resamples().then_some(Resample {
        output: [size.width, size.height],
        scale: size.scale as f32,
        finalscale: size.finalscale,
    }))
}

/// Grain at a fixed offset: the operation renders at its provisional one, and only the
/// harness gives it darktable's, a hash of the file name modulo the width darktable
/// renders it at (`docs/ports/grain.md`, Determinism).
struct GrainAt {
    offset: u32,
}

impl apertrust_plugin::Operation for GrainAt {
    fn manifest(&self) -> &'static apertrust_plugin::Manifest {
        grain::Grain.manifest()
    }

    fn schema(&self) -> &'static apertrust_plugin::Schema {
        grain::Grain.schema()
    }

    fn kernel(&self) -> &'static apertrust_plugin::Kernel {
        grain::Grain.kernel()
    }

    fn prepare(
        &self,
        params: &ParamValues,
        _image: &ImageContext,
        input: &ImageDesc,
    ) -> Result<apertrust_plugin::Prepared, apertrust_plugin::NotPorted> {
        let settings = grain::Settings {
            offset: self.offset,
            ..grain::settings(params)
        };
        let prepared = grain::prepare(
            &settings,
            &apertrust_color::WorkingProfile::linear_rec2020(),
        )?;
        Ok(apertrust_plugin::Prepared::per_pixel(
            input,
            prepared.params(),
            vec![grain::permutation_table()],
        ))
    }
}

/// `declared`, the image's size, as the photo shows: its sides swapped where the
/// orientation `instances` render on `image` swaps its axes.
fn shown(declared: [u32; 2], instances: &[Instance], image: &ImageContext) -> Result<[u32; 2]> {
    let bits = orientation(instances, image)?;
    Ok(if bits & apertrust_operations::flip::SWAP_XY != 0 {
        [declared[1], declared[0]]
    } else {
        declared
    })
}

/// The flip bits orientation renders with `instances` on `image` (decision 16), 0 where
/// it does not run.
fn orientation(instances: &[Instance], image: &ImageContext) -> Result<u8> {
    let Some(instance) = instances
        .iter()
        .find(|i| i.operation.manifest().id == "flip")
    else {
        return Ok(0);
    };
    let operation = instance.operation;
    let params = operation
        .schema()
        .resolve_over(&operation.defaults(image), &instance.params)?;
    Ok(apertrust_operations::flip::orientation(&params, image))
}

/// `region`, x, y, width and height of an image as it was before orientation's flip
/// bits `bits`, in `rendered`, the image after them: darktable's buffer flip of a
/// rectangle, the columns flipped, the rows, then the axes swapped.
fn turned(region: [usize; 4], rendered: &Image, bits: u8) -> [usize; 4] {
    use apertrust_operations::flip::{FLIP_X, FLIP_Y, SWAP_XY};
    let [x, y, width, height] = region;
    let swap = bits & SWAP_XY != 0;
    let (columns, rows) = if swap {
        (rendered.height, rendered.width)
    } else {
        (rendered.width, rendered.height)
    };
    let x = if bits & FLIP_X != 0 {
        columns - x - width
    } else {
        x
    };
    let y = if bits & FLIP_Y != 0 {
        rows - y - height
    } else {
        y
    };
    if swap {
        [y, x, height, width]
    } else {
        [x, y, width, height]
    }
}

/// The width darktable renders grain at in an export of an image `width` wide: the
/// whole image's at full size, its output's after a resample after demosaic, and at
/// finalscale's place the region its resample reads (decision 56).
fn grain_width(width: u32, resample: Option<Resample>) -> u32 {
    match resample {
        None => width,
        Some(resample) if resample.finalscale => {
            input_length(resample.output[0], resample.scale, width, true).min(width)
        }
        Some(resample) => resample.output[0],
    }
}

/// The operations whose cases `--add` puts on both sides of the whole pipeline.
/// Those with whole-image passes, the tone equalizer, haze removal, the colour equalizer,
/// local contrast, bloom and soften, measure an export at a size (decision 15).
const ADDABLE: [&str; 9] = [
    "grain",
    "vignette",
    "toneequal",
    "hazeremoval",
    "colorequal",
    "colorbalancergb",
    "bilat",
    "bloom",
    "soften",
];

/// Fails unless every one of `adds` is a case of an operation in [`ADDABLE`].
fn check_adds(adds: &[&Case]) -> Result<()> {
    for case in adds {
        ensure!(
            ADDABLE.contains(&case.operation),
            "the whole pipeline adds cases of {}, not {} (a {} case)",
            ADDABLE.join(", "),
            case.name,
            case.operation
        );
    }
    Ok(())
}

/// Puts each of `adds` in `instances` as the case's sidecar sets it, grain with
/// `grain_at`'s offset.
fn add_cases<'a>(
    instances: &mut Vec<Instance<'a>>,
    adds: &[&Case],
    grain_at: &'a GrainAt,
) -> Result<()> {
    for case in adds {
        let operation: &dyn apertrust_plugin::Operation = match case.operation {
            "grain" => grain_at,
            id => {
                apertrust_operations::operation(id).with_context(|| format!("no operation {id}"))?
            }
        };
        instances.push(Instance {
            operation,
            params: ports::pipeline_values(case)?,
        });
    }
    Ok(())
}

/// darktable's history for the whole pipeline of `instances` on `image`: Apertrust's
/// parameters for the modules it has, as [`raw_stage::pipeline_xmp`] writes them, the
/// resample before colorout at a `size`, and each of `adds`' entries.
fn darktable_history(
    image: &Path,
    metadata: &raw_stage::Metadata,
    context: &ImageContext,
    instances: &[Instance],
    size: Option<Size>,
    adds: &[&Case],
) -> Result<String> {
    // Exposure at Apertrust's effective value, color calibration with Apertrust's
    // illuminant and AgX at Apertrust's parameters.
    let stage = darktable_stage(image, metadata, instances)?;
    let exposure_params = params_of(instances, "exposure")?;
    let channel_mixer = channelmixerrgb::settings(params_of(instances, "channelmixerrgb")?);
    let agx_params = ports::agx_params_v7(&agx::settings(params_of(instances, "agx")?));
    let xmp = raw_stage::pipeline_xmp(
        &stage,
        exposure_params.float("black"),
        exposure::effective_exposure(exposure_params, context),
        &ports::channel_mixer_params_v3(&channel_mixer),
        &agx_params,
    )?;
    // At a size, darktable resamples where Apertrust does, before colorout.
    let mut xmp = match size {
        Some(_) => raw_stage::finalscale_before_colorout(&xmp)?,
        None => xmp,
    };
    for case in adds {
        xmp = golden::xmp::append_entry(&xmp, case.xmp, case.operation)?;
    }
    Ok(xmp)
}

/// `apertrust-golden export`: see [`Command::Export`].
fn export(
    image: &Path,
    output: &Path,
    darktable_xmp: Option<&Path>,
    size: Option<Size>,
    adds: &[&Case],
) -> Result<()> {
    check_adds(adds)?;
    let raw = raw_stage::decode(image)?;
    let metadata = raw_stage::Metadata::of(&raw)?;
    let context = apertrust_engine::raw::image_context(&raw);
    let camera = context.camera.as_ref().context("no camera metadata")?;
    let [_, _, width, height] = camera.declared;
    let shown = shown([width, height], &all_defaults(&context)?, &context)?;
    let resample = size
        .map(|size| resample_to(size, shown))
        .transpose()?
        .flatten();
    let file_name = image
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into());
    let grain_at = GrainAt {
        offset: grain::darktable_offset(&file_name, grain_width(shown[0], resample)),
    };
    let mut instances: Vec<Instance> = all_defaults(&context)?;
    cropped_to_declared(&mut instances, camera);
    add_cases(&mut instances, adds, &grain_at)?;
    if let Some(path) = darktable_xmp {
        let xmp = darktable_history(image, &metadata, &context, &instances, size, adds)?;
        std::fs::write(path, xmp).with_context(|| format!("writing {}", path.display()))?;
    }

    let gpu = pollster::block_on(GpuContext::new(&GpuOptions::default()))?;
    let input = apertrust_engine::raw::upload(&gpu, &raw)
        .context("the raw stage takes only 16-bit mosaics so far")??;
    let tiling = Tiling::Auto { budget: BUDGET };
    let exported = pollster::block_on(export_at(
        &gpu, &instances, &context, input, tiling, resample,
    ))?;
    let (width, height) = (exported.desc.width, exported.desc.height);
    let picture = apertrust_export::Image::new(&exported.pixels, width, height)?;
    let start = Instant::now();
    let file = std::fs::File::create(output)
        .with_context(|| format!("cannot create {}", output.display()))?;
    let mut out = std::io::BufWriter::new(file);
    apertrust_export::Format::Png(apertrust_export::png::Options::default())
        .write(picture, &mut out)?;
    std::io::Write::flush(&mut out)?;
    tracing::debug!(
        target: "perf",
        "wrote {} in {:.0} ms",
        apertrust_base::log::file_name(output),
        start.elapsed().as_secs_f64() * 1e3
    );
    println!(
        "Wrote {} ({width}×{height}){}",
        output.display(),
        if exported.tiles > 1 {
            format!(" in {} tiles", exported.tiles)
        } else {
            String::new()
        }
    );
    Ok(())
}

/// The whole pipeline at full size or at `size`, darktable's side at `darktable_size`,
/// which may differ from it only in its high quality, with `adds`' modules on both sides.
fn whole_pipeline(
    names: &[PathBuf],
    size: Option<Size>,
    darktable_size: Option<Size>,
    adds: &[&Case],
) -> Result<ExitCode> {
    check_adds(adds)?;
    let darktable = DarktableCli::locate()?;
    let gpu = pollster::block_on(GpuContext::new(&GpuOptions::default()))?;
    // Each raw with its defaults, never a saved edit: the exit test checks the defaults
    // both sides share, so an `.apxmp` beside a sample changes nothing here.
    let held = match size {
        Some(_) => Held::Measured,
        None => Held::Passed,
    };
    let tally = each_raw_as(&corpus_images(names)?, held, |image| {
        let raw = raw_stage::decode(image)?;
        let metadata = raw_stage::Metadata::of(&raw)?;
        if !matches!(raw.pixels, Pixels::U16(_)) {
            return Err(skip(SkipKind::Not16Bit, "floating-point values"));
        }
        let context = apertrust_engine::raw::image_context(&raw);
        let camera = context.camera.as_ref().context("no camera metadata")?;
        let [x, y, width, height] = camera.declared;
        // The photo as shown, which an export at a size fits: turned as its orientation
        // turns it at auto, the defaults' (decision 16).
        let shown = shown([width, height], &all_defaults(&context)?, &context)?;
        let resample = size
            .map(|size| resample_to(size, shown))
            .transpose()?
            .flatten();
        // Grain's offset is darktable's for the width darktable renders it at, which its
        // own high quality decides.
        let their_resample = darktable_size
            .map(|size| resample_to(size, shown))
            .transpose()?
            .flatten();
        let file_name = image
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into());
        let grain_at = GrainAt {
            offset: grain::darktable_offset(&file_name, grain_width(shown[0], their_resample)),
        };
        let mut instances: Vec<Instance> = all_defaults(&context)?;
        if size.is_some() {
            cropped_to_declared(&mut instances, camera);
        }
        add_cases(&mut instances, adds, &grain_at)?;
        let (icc_type, to_lab) = output_profile(params_of(&instances, "colorout")?)?;

        // darktable first: its render is cached and needs only the parameters, so a raw
        // its loader rejects is skipped before the export is paid for.
        let theirs = {
            let xmp = darktable_history(image, &metadata, &context, &instances, size, adds)?;
            let png = raw_stage::render_as(
                &darktable,
                image,
                &xmp,
                &[PNG16],
                icc_type,
                "png",
                darktable_size,
            )?;
            let rendered = Image::read_png16(&png)?;
            // darktable renders rawprepare's crop; Apertrust the declared image inside it.
            let rawprepare = params_of(&instances, "rawprepare")?;
            let inside = |at: u32, margin: &str| {
                at.checked_sub(rawprepare.int(margin) as u32)
                    .map(|offset| offset as usize)
                    .with_context(|| format!("the declared image starts before the {margin} crop"))
            };
            match size {
                // Cropped to the declared image, and sized.
                Some(_) => rendered,
                None => {
                    // darktable turns its render as orientation says: the declared
                    // image's place in it turned the same way (decision 16).
                    let declared = [
                        inside(x, "left")?,
                        inside(y, "top")?,
                        width as usize,
                        height as usize,
                    ];
                    let [x, y, width, height] =
                        turned(declared, &rendered, orientation(&instances, &context)?);
                    raw_stage::crop(&rendered, x, y, width, height)?
                }
            }
        };

        // Apertrust: the export's output before `apertrust-cli export` rounds it to 8 bits,
        // clipped to 0 to 1 as a PNG holds it. Only the tile count outlives the block.
        let (ours, tiles) = {
            let input = apertrust_engine::raw::upload(&gpu, &raw)
                .ok_or_else(|| skip(SkipKind::Not16Bit, ""))??;
            let budget = Tiling::Auto { budget: BUDGET };
            let exported = pollster::block_on(export_at(
                &gpu, &instances, &context, input, budget, resample,
            ))?;
            let pixels = exported
                .pixels
                .iter()
                .map(|p| [0, 1, 2].map(|c| p[c].clamp(0.0, 1.0)))
                .collect();
            let (ours_width, ours_height) = (exported.desc.width, exported.desc.height);
            let ours = Image::new(ours_width as usize, ours_height as usize, pixels)?;
            (ours, exported.tiles)
        };

        if (theirs.width, theirs.height) != (ours.width, ours.height) {
            bail!(
                "darktable-cli exported {}×{}, Apertrust {}×{}",
                theirs.width,
                theirs.height,
                ours.width,
                ours.height
            );
        }
        let (_, stats) = delta_e::compare_with(&theirs, &ours, Tolerance::DARKTABLE, to_lab)?;
        let pixels = (ours.width * ours.height).max(1);
        let over = stats.over_max as f64 / pixels as f64;
        // At a size the comparison is a measurement, with no bound yet (decision 56).
        let passed = size.is_some() || (stats.mean <= PIPELINE_MEAN && over <= PIPELINE_OVER);
        let (width, height) = (ours.width, ours.height);
        Ok((
            passed,
            format!(
                "{}: {width}×{height}{}{}{}, ΔE2000 mean {:.3}, p95 {:.3}, p99 {:.3}, max {:.2}, {:.4}% over {}",
                raw.camera.model,
                match (size, darktable_size) {
                    (Some(size), Some(theirs)) if theirs.high_quality != size.high_quality => {
                        format!(
                            ", hq {} (darktable {}), upscale {}",
                            size.high_quality, theirs.high_quality, size.upscale
                        )
                    }
                    (Some(size), _) => {
                        format!(", hq {}, upscale {}", size.high_quality, size.upscale)
                    }
                    (None, _) => String::new(),
                },
                if adds.is_empty() {
                    String::new()
                } else {
                    let names: Vec<&str> = adds.iter().map(|case| case.name).collect();
                    format!(", with {}", names.join(" + "))
                },
                if tiles > 1 {
                    format!(" in {tiles} tiles")
                } else {
                    String::new()
                },
                stats.mean,
                stats.p95,
                stats.p99,
                stats.max,
                100.0 * over,
                Tolerance::DARKTABLE.max
            ),
        ))
    });
    Ok(exit_code(tally.passes()))
}

fn raw_run(names: &[PathBuf], as_shot: bool) -> Result<ExitCode> {
    let darktable = DarktableCli::locate()?;
    let gpu = pollster::block_on(GpuContext::new(&GpuOptions::default()))?;
    let tally = each_raw(&test_images(names)?, |image| {
        let raw = raw_stage::decode(image)?;
        let metadata = raw_stage::Metadata::of(&raw)?;
        let mut instances = raw_stage_defaults(&raw)?;
        if as_shot {
            let preset = apertrust_operations::temperature::PRESETS
                .iter()
                .position(|&p| p == "as_shot")
                .context("temperature has no \"as shot\" preset")?;
            for instance in &mut instances {
                if instance.operation.manifest().id == "temperature" {
                    instance.params.set("preset", Value::Choice(preset as u32));
                }
            }
        }
        let input = apertrust_engine::raw::upload(&gpu, &raw)
            .ok_or_else(|| skip(SkipKind::Not16Bit, ""))??;
        let context = apertrust_engine::raw::image_context(&raw);
        let limits = pipeline::Limits::of(&gpu);
        let plan = pipeline::plan(&instances, &context, input.desc(), Pipe::Export, &limits)?;
        let mut pipeline = pollster::block_on(Pipeline::new(&gpu, plan, input))?;
        pipeline.render(&gpu)?;
        let output = pipeline.output();
        let desc = output.desc();
        let rawprepare = params_of(&instances, "rawprepare")?;
        let stage = darktable_stage(image, &metadata, &instances)?;
        let xmp = raw_stage::passthrough_xmp(&stage)?;
        let rendered = Image::read(&raw_stage::render(&darktable, image, &xmp, &[])?)?;
        let [x0, y0] = desc.sensor_origin;
        let (expected, actual, tolerance) = if desc.format == Format::Rgba {
            // Demosaic writes the declared crop; darktable's render is rawprepare's.
            let rgb = output.download(&gpu)?;
            let expected = Image::new(
                desc.width as usize,
                desc.height as usize,
                rgb.iter().map(|p| [p[0], p[1], p[2]]).collect(),
            )?;
            let (left, top) = (rawprepare.int("left") as u32, rawprepare.int("top") as u32);
            let actual = raw_stage::crop(
                &rendered,
                (x0 - left) as usize,
                (y0 - top) as usize,
                desc.width as usize,
                desc.height as usize,
            )?;
            (expected, actual, DEMOSAIC_TOLERANCE)
        } else {
            let ours = output.download_mosaic(&gpu)?;
            let expected =
                raw_stage::mosaic_image(&ours, desc.width, desc.height, (x0, y0), &metadata)?;
            (expected, rendered, PASSTHROUGH_TOLERANCE)
        };
        let found = raw_stage::compare(&expected, &actual, 4, tolerance);
        let aligned = found.offset == (0, 0)
            && (actual.width, actual.height) == (expected.width, expected.height);
        let over = found.over as f64 / (found.compared.0 * found.compared.1).max(1) as f64;
        // A mosaic matches to rounding; a demosaiced image too, but for rare clusters
        // where rounding tips one of RCD's or Markesteijn's direction choices, most of
        // them in Markesteijn's homogeneity ties across flat, clipped highlights. A
        // porting error shows as a mean above 1e-6 and whole percents of pixels.
        let passed = aligned
            && if desc.format == Format::Rgba {
                over < DEMOSAIC_OVER && found.mean_abs < 1e-6
            } else {
                found.max_abs <= tolerance
            };
        let ids: Vec<&str> = instances
            .iter()
            .map(|i| i.operation.manifest().id)
            .collect();
        Ok((
            passed,
            format!(
                "{}: {} {}×{}, darktable {}×{}; offset {:?}, max |Δ| {:.1e}, mean {:.1e}, {:.4}% over {tolerance:.0e}",
                raw.camera.model,
                ids.join(" + "),
                desc.width,
                desc.height,
                actual.width,
                actual.height,
                found.offset,
                found.max_abs,
                found.mean_abs,
                100.0 * over,
            ),
        ))
    });
    Ok(exit_code(tally.passes()))
}

/// Difference from darktable's demosaic that counts a pixel as different: darktable's
/// CPU code fuses multiplications and additions where the GPU may not, and RCD's
/// direction choices can tip on a rounding difference.
const DEMOSAIC_TOLERANCE: f32 = 1e-4;
/// The share of pixels that may differ from darktable's demosaic by more than
/// [`DEMOSAIC_TOLERANCE`]: at most 0.02% on the corpus, on 40-megapixel X-Trans raws
/// with clipped highlights; 0.001% or less on Bayer raws.
const DEMOSAIC_OVER: f64 = 5e-4;

/// Largest relative difference between darktable's white balance coefficients and
/// Apertrust's that still counts as the same: both are the camera's multipliers in
/// `f32`.
const COEFFICIENT_TOLERANCE: f64 = 1e-4;

fn raw_defaults(names: &[PathBuf]) -> Result<ExitCode> {
    let darktable = DarktableCli::locate()?;
    let tally = each_raw(&test_images(names)?, |image| {
        let raw = raw_stage::decode(image)?;
        let metadata = raw_stage::Metadata::of(&raw)?;
        let instances = raw_stage_defaults(&raw)?;
        let ours = params_of(&instances, "rawprepare")?;
        let our_coefficients = coefficients(params_of(&instances, "temperature")?);

        // rawprepare's defaults: its entry removed, temperature at 1.
        let stage = RawStage::passthrough(None, Some([1.0; 3]), metadata.is_xtrans());
        let xmp = raw_stage::passthrough_xmp(&stage)?;
        let actual = Image::read(&raw_stage::render(&darktable, image, &xmp, &[])?)?;
        let theirs = raw_stage::recover_defaults(&raw, &actual)?;

        // temperature's defaults: our rawprepare, temperature's entry removed, in the
        // display-referred workflow, where colorin applies no late correction.
        let rawprepare = raw_stage::rawprepare_from(ours);
        let stage = RawStage::passthrough(Some(rawprepare.clone()), None, metadata.is_xtrans());
        let xmp = raw_stage::passthrough_xmp(&stage)?;
        let conf = [raw_stage::DISPLAY_REFERRED];
        let balanced = Image::read(&raw_stage::render(&darktable, image, &xmp, &conf)?)?;
        let their_coefficients =
            raw_stage::recover_coefficients(&raw, &balanced, &rawprepare, &metadata)?;

        let margin = |name| ours.int(name) as u32;
        let our_crop = [
            margin("left"),
            margin("top"),
            raw.width - margin("left") - margin("right"),
            raw.height - margin("top") - margin("bottom"),
        ];
        let our_black = BLACK.map(|name| ours.int(name) as f64);
        let our_white = ours.int("raw_white_point") as f64;

        // darktable's levels are whole numbers; a fit that does not land on one did
        // not find them.
        let whole = |v: &f64| (v - v.round()).abs() < 0.05;
        let exact = theirs.black.iter().chain(&theirs.white).all(whole);
        let same_balance = their_coefficients
            .iter()
            .zip(our_coefficients)
            .all(|(t, o)| (t / f64::from(o) - 1.0).abs() <= COEFFICIENT_TOLERANCE);
        let passed = exact
            && theirs.crop == our_crop
            && theirs.black.map(f64::round) == our_black
            && theirs.white.iter().all(|w| w.round() == our_white)
            && same_balance;
        let masked = raw.masked_black().map_or_else(String::new, |m| {
            format!(
                ", masked areas {}",
                levels(&m.values.iter().map(|&v| f64::from(v)).collect::<Vec<_>>())
            )
        });
        let balance = |c: [f64; 3]| format!("{:.4} {:.4} {:.4}", c[0], c[1], c[2]);
        Ok((
            passed,
            format!(
                "{}: crop {} / ours {}; black {} / ours {}{masked}; white {} / ours {our_white}; white balance {} / ours {}; fit rms {:.1e}",
                raw.camera.model,
                crop(theirs.crop),
                crop(our_crop),
                levels(&theirs.black),
                levels(&our_black),
                levels(&theirs.white),
                balance(their_coefficients),
                balance(our_coefficients.map(f64::from)),
                theirs.rms,
            ),
        ))
    });
    Ok(exit_code(tally.passes()))
}

/// "W×H at (x, y)".
fn crop([x, y, width, height]: [u32; 4]) -> String {
    format!("{width}×{height} at ({x}, {y})")
}

/// Levels as whole numbers where they are, one value when all are the same.
fn levels(values: &[f64]) -> String {
    let show = |v: &f64| {
        if (v - v.round()).abs() < 0.05 {
            format!("{}", v.round())
        } else {
            format!("{v:.2}")
        }
    };
    let shown: Vec<String> = values.iter().map(show).collect();
    if shown.windows(2).all(|pair| pair[0] == pair[1]) {
        shown[0].clone()
    } else {
        shown.join(" ")
    }
}

fn rgb_sums(pixels: &[[f32; 4]]) -> Vec<f64> {
    pixels
        .iter()
        .map(|[r, g, b, _]| f64::from(*r) + f64::from(*g) + f64::from(*b))
        .collect()
}

fn relative_error(actual: f64, expected: f64) -> f64 {
    (actual - expected).abs() / expected.abs().max(actual.abs()).max(f64::MIN_POSITIVE)
}

fn pass_fail(passed: bool) -> &'static str {
    if passed { "PASS" } else { "FAIL" }
}

/// Where reverse mode and finite differences disagree: how much of the total gap comes
/// from the worst pixels, and those pixels' inputs, with forward mode alongside. A gap
/// concentrated in few pixels points at a non-smooth spot in the kernel; an even
/// spread, at a missing term. Forward mode agreeing with finite differences where
/// reverse mode does not points at the reverse-mode code Slang generates.
fn explain_gap(
    input: &[[f32; 3]],
    describe: &dyn Fn(usize) -> String,
    reverse: &[f64],
    forward: &[f64],
    numeric: &[f64],
) {
    let mut gaps: Vec<(usize, f64)> = reverse
        .iter()
        .zip(numeric)
        .map(|(r, n)| r - n)
        .enumerate()
        .collect();
    let total: f64 = gaps.iter().map(|(_, gap)| gap).sum();
    gaps.sort_by(|a, b| b.1.abs().total_cmp(&a.1.abs()));
    let share = |per_mille: usize| -> f64 {
        let count = (gaps.len() * per_mille).div_ceil(1000);
        gaps[..count].iter().map(|(_, gap)| gap).sum()
    };
    let forward_gap: f64 = forward.iter().zip(numeric).map(|(f, n)| f - n).sum();
    println!(
        "    gap {total:.4e} (forward mode's {forward_gap:.4e}); the worst 0.1% of pixels carry {:.4e}, 1% {:.4e}, 10% {:.4e}",
        share(1),
        share(10),
        share(100)
    );
    // Per-pixel relative error where the finite difference is not tiny.
    let mut relative: Vec<f64> = reverse
        .iter()
        .zip(numeric)
        .filter(|(_, n)| n.abs() > 1e-4)
        .map(|(r, n)| (r - n) / n.abs())
        .collect();
    relative.sort_by(f64::total_cmp);
    if !relative.is_empty() {
        let at = |q: usize| relative[(relative.len() - 1) * q / 100];
        println!(
            "    per-pixel relative error, percentiles 5/25/50/75/95: {:.1e} {:.1e} {:.1e} {:.1e} {:.1e}",
            at(5),
            at(25),
            at(50),
            at(75),
            at(95)
        );
    }
    let show = |i: usize| {
        let [r, g, b] = input[i];
        println!(
            "      {} input ({r:.4}, {g:.4}, {b:.4}): reverse {:.4e}, forward {:.4e}, finite differences {:.4e}",
            describe(i),
            reverse[i],
            forward[i],
            numeric[i]
        );
    };
    println!("    largest gaps from finite differences:");
    for &(i, _) in gaps.iter().take(5) {
        show(i);
    }
    let mut modes_apart: Vec<usize> = (0..reverse.len()).collect();
    modes_apart.sort_by(|&a, &b| {
        let apart = |i: usize| (reverse[i] - forward[i]).abs();
        apart(b).total_cmp(&apart(a))
    });
    println!("    largest differences between forward and reverse mode:");
    for &i in modes_apart.iter().take(5) {
        show(i);
    }
}

fn exit_code(passed: bool) -> ExitCode {
    if passed {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn select(names: &[String]) -> Result<Vec<&'static Case>> {
    if names.is_empty() {
        Ok(CASES.iter().collect())
    } else {
        names.iter().map(|name| find(name)).collect()
    }
}

fn find(name: &str) -> Result<&'static Case> {
    cases::find(name).with_context(|| format!("no case named {name:?}"))
}

fn summary(stats: &Stats, tolerance: Tolerance) -> String {
    format!(
        "{}: ΔE2000 mean {:.4} (≤ {:.3}), p95 {:.4}, max {:.4} (≤ {:.3}), {} pixels over max",
        if stats.passes(tolerance) {
            "PASS"
        } else {
            "FAIL"
        },
        stats.mean,
        tolerance.mean,
        stats.p95,
        stats.max,
        tolerance.max,
        stats.over_max
    )
}

#[cfg(test)]
mod tests {
    use anyhow::anyhow;

    use super::*;

    fn raw(path: &str, expected: Expected) -> Raw {
        Raw {
            path: PathBuf::from(path),
            expected,
        }
    }

    /// A check that answers by the raw's file name.
    fn by_name(image: &Path) -> Result<(bool, String)> {
        match image.file_name().unwrap().to_str().unwrap() {
            "pass.raw" => Ok((true, "fine".into())),
            "fail.raw" => Ok((false, "off".into())),
            "mono.raw" => Err(skip(SkipKind::NotColourMosaic, "no colour filter array")),
            _ => Err(anyhow!("device lost")),
        }
    }

    #[test]
    fn each_raw_counts_and_passes_only_with_a_check_and_no_failure() {
        let any = |name| raw(name, Expected::AnySkip);
        let tally = each_raw(&[any("pass.raw"), any("mono.raw")], by_name);
        assert_eq!(
            tally,
            Tally {
                passed: 1,
                failed: 0,
                skipped: 1
            }
        );
        assert!(tally.passes());
        // A failed check and an error that is not a skip both fail the run.
        let tally = each_raw(
            &[any("pass.raw"), any("fail.raw"), any("lost.raw")],
            by_name,
        );
        assert_eq!(
            tally,
            Tally {
                passed: 1,
                failed: 2,
                skipped: 0
            }
        );
        assert!(!tally.passes());
        // Nothing checked is no pass.
        let tally = each_raw(&[any("mono.raw")], by_name);
        assert_eq!(tally.skipped, 1);
        assert!(!tally.passes());
        assert!(!each_raw(&[], by_name).passes());
    }

    #[test]
    fn only_expected_skips_skip_and_only_for_their_reason() {
        let mono = || Err(skip(SkipKind::NotColourMosaic, "no colour filter array"));
        let skipped = |expected| judge(mono(), expected).0;
        assert_eq!(skipped(Expected::AnySkip), Verdict::Skip);
        assert_eq!(
            skipped(Expected::Skip(SkipKind::NotColourMosaic)),
            Verdict::Skip
        );
        let (verdict, line, _) = judge(mono(), Expected::Checked);
        assert_eq!(verdict, Verdict::Fail);
        assert!(line.starts_with("skipped unexpectedly: "), "{line}");
        let (verdict, line, _) = judge(mono(), Expected::Skip(SkipKind::Undecodable));
        assert_eq!(verdict, Verdict::Fail);
        assert!(line.contains("rawler cannot decode it"), "{line}");
        // A listed sample that is now checked just runs, with a note.
        let (verdict, _, note) = judge(
            Ok((true, String::new())),
            Expected::Skip(SkipKind::Undecodable),
        );
        assert_eq!(verdict, Verdict::Pass);
        assert!(note.unwrap().contains("stale"));
        assert_eq!(judge(Ok((true, String::new())), Expected::Checked).2, None);
        // Errors that are not skips fail, whatever was expected.
        assert_eq!(
            judge(Err(anyhow!("lost")), Expected::Skip(SkipKind::Undecodable)).0,
            Verdict::Fail
        );
    }

    #[test]
    fn the_corpus_list_comes_from_the_manifest() {
        let digest = "a".repeat(64);
        let manifest = format!(
            "{digest}  Sony/ILCE-7RM5/7RM5-LosslessCompressedSmall.ARW\n\
             {digest} *Adobe DNG Converter/Canon EOS 5D Mark III/lossless.DNG\n\n"
        );
        assert_eq!(
            manifest_names(&manifest).unwrap(),
            [
                "Adobe DNG Converter/Canon EOS 5D Mark III/lossless.DNG",
                "Sony/ILCE-7RM5/7RM5-LosslessCompressedSmall.ARW"
            ]
        );
        assert!(manifest_names("not a checksum  Sony/x.ARW").is_err());
        assert!(manifest_names(&format!("{digest}  ")).is_err());

        // The real manifest: 40 samples, and every expected skip among them.
        let names = manifest_names(MANIFEST).unwrap();
        assert_eq!(names.len(), 40);
        for (name, _) in EXPECTED_SKIPS {
            assert!(names.contains(name), "{name} is not in the manifest");
        }
    }

    #[test]
    fn corpus_names_find_the_expected_skips() {
        let corpus = Path::new("/corpus");
        let name = corpus_name(
            Path::new("/corpus/Canon/EOS 5D Mark II/09.canon.sraw1.cr2"),
            corpus,
        );
        assert_eq!(
            name.as_deref(),
            Some("Canon/EOS 5D Mark II/09.canon.sraw1.cr2")
        );
        assert_eq!(
            expected_in_corpus(name.as_deref()),
            Expected::Skip(SkipKind::NotColourMosaic)
        );
        assert_eq!(corpus_name(Path::new("/elsewhere/mire1.cr2"), corpus), None);
        assert_eq!(expected_in_corpus(None), Expected::Checked);
        assert_eq!(
            expected_in_corpus(Some("Nikon/D5100/00.nikon.12b-lossy-raw.nef")),
            Expected::Checked
        );
    }

    #[test]
    fn a_name_resolves_to_what_exists_or_stays_as_given() {
        let dir =
            std::env::temp_dir().join(format!("apertrust-golden-names-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("Make/Model")).unwrap();
        std::fs::write(dir.join("Make/Model/a.raw"), b"").unwrap();
        assert_eq!(
            resolve(Path::new("Make/Model/a.raw"), Some(&dir)),
            dir.join("Make/Model/a.raw")
        );
        // A path that exists wins over the same name in the corpus folder. Tests run in
        // the crate's directory, where `src/main.rs` is.
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/main.rs"), b"").unwrap();
        assert_eq!(
            resolve(Path::new("src/main.rs"), Some(&dir)),
            Path::new("src/main.rs")
        );
        let missing = Path::new("test-images/raw.pixls.us/Make/Model/missing.raw");
        assert_eq!(resolve(missing, Some(&dir)), missing);
        assert_eq!(resolve(missing, None), missing);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn each_output_profile_has_its_icc_type_and_lab() {
        let mut values = ParamValues::new();
        let mut profile = |name: &str| {
            let index = colorout::PROFILES.iter().position(|&p| p == name).unwrap();
            values.set("type", Value::Choice(index as u32));
            output_profile(&values)
        };
        let pixel = [0.5, 0.25, 0.75];
        for (name, icc_type, to_lab) in [
            ("srgb", "SRGB", delta_e::srgb_to_lab as ToLab),
            ("lin_rec2020", "LIN_REC2020", delta_e::rec2020_to_lab),
            ("lin_rec709", "LIN_REC709", delta_e::rec709_to_lab),
        ] {
            let (got, lab) = profile(name).unwrap();
            assert_eq!(got, icc_type);
            assert_eq!(lab(pixel), to_lab(pixel), "{name}");
        }
        let other = colorout::PROFILES
            .iter()
            .find(|&&p| !["srgb", "lin_rec2020", "lin_rec709"].contains(&p))
            .unwrap();
        assert!(profile(other).is_err());
    }
}
