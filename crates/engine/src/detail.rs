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

//! The zoomed view's whole-image results (decision 99; `docs/engine.md`, The zoomed view):
//! a detail renders a region, but an operation's whole-image passes cover the whole image
//! at the detail's scale, never the region (decision 15). Its plan leaves them out and
//! binds their result, which [`Results`] holds across the detail's renders under the key
//! the untiled plan at that scale gives it, with no region in it, so a pan, an edit after
//! the operation or a change of only its later passes runs nothing of them.
//!
//! Where the key changed, a first round over the whole image computes the result, as an
//! export's (`docs/engine.md`, Tiling): an operation that opts in
//! (`Operation::tiles_whole_image_passes`) in tiles from the detail's source, demosaic's
//! image, or in a tiled viewport the raw stage's output, each tile then demosaicing its
//! part as an export's first round does, the others (local contrast, filmic rgb's
//! reconstruction) whole with every operation before them. Haze removal's estimate, computed at full resolution by the
//! fitted pipeline, is borrowed from it by its key and read in place, never estimated
//! twice; where that pipeline holds none, with geometry after it, the detail estimates in
//! a round of its own. Every result of a fitted pipeline at scale 1, a photo no larger
//! than the bounds, is borrowed the same way.
//!
//! Built at scale 1, the 100% view; scales between Fit and 100% are planned.

use apertrust_plugin::{ImageContext, ImageDesc, PassKind};

use crate::compute::{ComputeError, GpuImage, Timings};
use crate::export::{self, ExportError, Round};
use crate::gpu::GpuContext;
use crate::pipeline::{
    self, Instance, Kernels, Limits, Pipe, Pipeline, Plan, PlanError, PlannedOperation,
    SMALLEST_TILE, scratch_buffer, scratch_size, takes_a_first_round,
};

/// The whole-image results of a detail's operations, held across its renders (decision
/// 99), and the kernels its pipelines compiled: built empty, filled by [`Self::plan`],
/// bound by [`Self::pipeline`] and computed by [`Self::run`].
#[derive(Default)]
pub struct Results {
    /// Per operation with whole-image passes, in the plan's order.
    held: Vec<Held>,
    /// The kernels the detail's pipeline and its rounds compiled, which each new one takes.
    kernels: Kernels,
}

/// One operation's whole-image result.
struct Held {
    id: &'static str,
    /// What its buffers hold: the key the untiled plan at the detail's scale gives its
    /// whole-image passes, or the key of the fitted pipeline's full-resolution part where
    /// borrowed; `None` until its round has run.
    key: Option<u64>,
    /// Per scratch buffer of the operation, in its order, the bytes of each one its passes
    /// after the whole-image ones bind; `None` for the rest, which take no memory.
    sizes: Vec<Option<u64>>,
    /// Those buffers, once made or borrowed.
    buffers: Vec<Option<wgpu::Buffer>>,
    /// Whether the buffers are the fitted pipeline's.
    borrowed: bool,
}

/// The rounds a detail's frame runs before its region's passes, from [`Results::plan`]:
/// those whose result's key changed, in the plan's order.
#[derive(Default)]
pub struct Rounds {
    due: Vec<Due>,
}

/// One round: the operation, the key its result will have, and its tiles' plans.
struct Due {
    id: &'static str,
    key: u64,
    plans: Vec<Plan>,
}

impl Rounds {
    /// Whether no round runs.
    pub fn is_empty(&self) -> bool {
        self.due.is_empty()
    }

    /// The operations whose rounds run, in order.
    pub fn operations(&self) -> Vec<&'static str> {
        self.due.iter().map(|due| due.id).collect()
    }

    /// How many tiles each round takes, in order.
    pub fn tiles(&self) -> Vec<usize> {
        self.due.iter().map(|due| due.plans.len()).collect()
    }
}

impl Results {
    /// Taking the kernels another pipeline over the photo compiled, the fitted one's.
    pub(crate) fn with_kernels(kernels: Kernels) -> Self {
        Self {
            held: Vec::new(),
            kernels,
        }
    }

    /// Each result held and its key, for the session's tests.
    #[cfg(test)]
    pub(crate) fn keys(&self) -> Vec<(&'static str, Option<u64>)> {
        self.held.iter().map(|held| (held.id, held.key)).collect()
    }

    /// The detail's plan and its rounds, from `region`, a plan of `instances` for
    /// `Pipe::Detail` over its source, demosaic's image or the raw stage's output
    /// ([`Pipeline::detail_source`]), whose content has the key `key`: its keys
    /// chained from that ([`Plan::keyed_on`]), and each operation's whole-image passes left
    /// out, its later passes binding the result held, of which only the scratch they bind
    /// takes memory. A result is borrowed from `fitted` where it holds it at full
    /// resolution (decision 15), or else, where its key changed, takes a round: tiles of
    /// its stage within `limits`' budget for an operation that opts in, the whole of it for
    /// the others. Refused, as an export is, where a held result binds more than the GPU
    /// allows ([`ExportError::HeldScratch`]) or no tiles fit ([`ExportError::NoTiles`]).
    pub fn plan(
        &mut self,
        region: Plan,
        key: u64,
        instances: &[Instance],
        image: &ImageContext,
        limits: &Limits,
        fitted: Option<&Pipeline>,
    ) -> Result<(Plan, Rounds), ExportError> {
        let mut plan = region.keyed_on(key);
        let runs = pipeline::runs(&plan.operations, 0);
        let Some(&last) = runs.last() else {
            self.held.clear();
            return Ok((plan, Rounds::default()));
        };
        let Pipe::Detail { scale, .. } = plan.pipe else {
            panic!("a detail's plan, not {:?}", plan.pipe);
        };
        // Every part planned with the detail's output profile, as an export's parts are
        // with the whole export's (decision 26).
        let image = &ImageContext {
            output_profile: plan.image.output_profile,
            ..image.clone()
        };
        let after: Vec<Instance> = (pipeline::from_source(instances, &plan.input).into_iter())
            .cloned()
            .collect();
        // The untiled plan at the detail's scale, through the last operation with
        // whole-image passes, whose keys hold no region: each result's key is its.
        let [width, height] = operation(&plan, last).extent;
        let pipe = Pipe::Detail {
            region: [0, 0, width, height],
            scale,
        };
        let through = export::instances_through(&after, last);
        let whole = pipeline::plan(&through, image, plan.input, pipe, limits)?.keyed_on(key);
        export::held_scratch_binds(&whole, &runs, limits)?;
        let mut held: Vec<Held> = Vec::new();
        let mut due = Vec::new();
        let mut largest = 0;
        for &run in &runs {
            let sizes = bound_sizes(&mut plan, &whole, run)?;
            let borrowed = fitted
                .and_then(|fitted| fitted.result_at_full_resolution(run))
                .filter(|(_, buffers)| fits(&sizes, buffers.iter().map(Some)));
            if let Some((part, buffers)) = borrowed {
                held.push(Held::borrowed(run, part, sizes, buffers));
                continue;
            }
            let mut entry = self.take(run, sizes);
            let op = operation(&whole, run);
            let result = op.whole_image.map_or(op.key, |whole| whole.key);
            if entry.key != Some(result) {
                entry.key = None;
                let earlier: Vec<&str> = held.iter().map(|held| held.id).collect();
                let tiles = Tiles {
                    after: &after,
                    image,
                    source: plan.input,
                    earlier: &earlier,
                    kept: entry.sizes.iter().flatten().sum(),
                    limits,
                };
                let (plans, own) = tiles.plan(op)?;
                largest = largest.max(own);
                let (id, key) = (run, result);
                due.push(Due { id, key, plans });
            }
            held.push(entry);
        }
        self.held = held;
        // The held results, counted in the region's plan, beside its other buffers and
        // the largest round tile's, its source counted once, logged where they pass the
        // budget (decision 15).
        let results = export::held_bytes(&plan, &runs);
        let what = "the region and each round's tile";
        let source = plan.input.buffer_size();
        let own = plan.bytes() - results + largest.saturating_sub(source);
        export::log_held("detail", results, own, &runs, what, limits);
        Ok((plan, Rounds { due }))
    }

    /// The result held for `run` whose buffers have `sizes`, taken out to be held again,
    /// with its key; or a new one, holding nothing yet, its buffers made by
    /// [`Self::pipeline`].
    fn take(&mut self, run: &'static str, sizes: Vec<Option<u64>>) -> Held {
        let kept = (self.held.iter())
            .position(|held| held.id == run && !held.borrowed && held.sizes == sizes)
            .map(|at| self.held.swap_remove(at));
        kept.unwrap_or_else(|| Held {
            id: run,
            key: None,
            buffers: vec![None; sizes.len()],
            sizes,
            borrowed: false,
        })
    }

    /// The detail's own pipeline for `plan`, from [`Self::plan`], over `source`, binding
    /// the results held, buffers made for those whose round has yet to run: `old` updated
    /// where it reads `source`, else one built afresh, taking the kernels compiled so far.
    pub async fn pipeline(
        &mut self,
        gpu: &GpuContext,
        plan: Plan,
        source: GpuImage,
        old: Option<Pipeline>,
    ) -> Result<Pipeline, ComputeError> {
        for held in &mut self.held {
            for (buffer, size) in held.buffers.iter_mut().zip(&held.sizes) {
                if let (None, Some(size)) = (&buffer, size) {
                    *buffer = Some(scratch_buffer(gpu, *size));
                }
            }
        }
        let held: Vec<(&'static str, Vec<Option<wgpu::Buffer>>)> = (self.held.iter())
            .map(|held| (held.id, held.buffers.clone()))
            .collect();
        let built = match old {
            Some(old) if old.reads(&source) => old.update_holding(gpu, plan, &held).await?,
            _ => {
                let kernels = self.kernels.clone();
                Pipeline::holding_with(gpu, plan, source, &held, kernels).await?
            }
        };
        // The region's kernels beside the rounds', which an update in place never saw.
        self.kernels.merge(&built.shared_kernels());
        Ok(built)
    }

    /// Runs `rounds` from `source`, the image the detail reads, each into the
    /// buffers held for its result, and their passes' timings. Each tile is a submission
    /// of its own, waited for, as an export's first rounds are (decision 15), after the
    /// caller's submissions, which may write that image; the round's working buffers are
    /// freed after it, and its result is kept under its key.
    pub async fn run(
        &mut self,
        gpu: &GpuContext,
        rounds: Rounds,
        source: &GpuImage,
    ) -> Result<Timings, ComputeError> {
        let mut timings = Timings::default();
        for Due { id, key, plans } in rounds.due {
            let at = (self.held.iter())
                .position(|held| held.id == id)
                .expect("a round's result is held");
            let mut held: Vec<(&'static str, Vec<Option<wgpu::Buffer>>)> = self.held[..=at]
                .iter()
                .map(|held| (held.id, held.buffers.clone()))
                .collect();
            let largest = export::largest(&plans).clone();
            let kernels = self.kernels.clone();
            let built = Pipeline::holding_with(gpu, largest, source.clone(), &held, kernels);
            let mut tiles = built.await?;
            // Every scratch buffer of its own held across its tiles, as an export's.
            held[at].1 = tiles.scratch_of(id).into_iter().map(Some).collect();
            for plan in plans {
                tiles = tiles.update_holding(gpu, plan, &held).await?;
                let rendered = tiles.render(gpu)?;
                timings.passes.extend(rendered.passes);
                timings.wall_ms += rendered.wall_ms;
            }
            self.kernels.merge(&tiles.into_kernels());
            self.held[at].key = Some(key);
        }
        Ok(timings)
    }
}

impl Held {
    /// The fitted pipeline's result for `run`, its full-resolution part's, keyed `part`:
    /// of its scratch `buffers`, those `sizes` asks.
    fn borrowed(
        run: &'static str,
        part: u64,
        sizes: Vec<Option<u64>>,
        buffers: Vec<wgpu::Buffer>,
    ) -> Self {
        let buffers = (sizes.iter().zip(buffers))
            .map(|(size, buffer)| size.map(|_| buffer))
            .collect();
        Self {
            id: run,
            key: Some(part),
            sizes,
            buffers,
            borrowed: true,
        }
    }
}

/// Leaves `run`'s whole-image passes out of `plan`, a detail's, and the scratch its later
/// passes do not bind; then the bytes of each scratch buffer of it they bind, `None` for
/// the rest. Refused where the region's plan sizes that scratch otherwise than `whole`,
/// the untiled plan, as an operation sized by its input rather than the whole image
/// would.
fn bound_sizes(plan: &mut Plan, whole: &Plan, run: &str) -> Result<Vec<Option<u64>>, PlanError> {
    let scratch = |plan: &Plan| -> Vec<u64> {
        let op = operation(plan, run);
        op.scratch
            .iter()
            .map(|&index| plan.scratch[index])
            .collect()
    };
    let all = scratch(plan);
    if all != scratch(whole) {
        return Err(PlanError::Detail {
            operation: operation(whole, run).id,
            what: "its whole-image passes are sized by the region",
        });
    }
    plan.retain_passes(run, |pass| pass.kind != PassKind::Reduction);
    let bound = plan.drop_unbound_scratch(run);
    let sizes = (all.iter().enumerate())
        .map(|(index, &bytes)| bound.contains(&index).then_some(bytes))
        .collect();
    Ok(sizes)
}

/// The operation `id` of `plan`.
fn operation<'a>(plan: &'a Plan, id: &str) -> &'a PlannedOperation {
    (plan.operations.iter())
        .find(|op| op.id == id)
        .expect("an operation of the plan")
}

/// Whether `buffers`, an operation's scratch buffers, are each the size `sizes` asks
/// where it asks one.
fn fits<'a>(
    sizes: &[Option<u64>],
    buffers: impl ExactSizeIterator<Item = Option<&'a wgpu::Buffer>>,
) -> bool {
    buffers.len() == sizes.len()
        && (sizes.iter().zip(buffers)).all(|(size, buffer)| match (size, buffer) {
            (Some(size), Some(buffer)) => buffer.size() == scratch_size(*size),
            (Some(_), None) => false,
            (None, _) => true,
        })
}

/// What a round's tiles are planned from: the detail's instances after the raw stage,
/// its image context, its source's image, the operations whose results the round binds,
/// the bytes of the round's own result the detail keeps, and the limits.
struct Tiles<'a> {
    after: &'a [Instance<'a>],
    image: &'a ImageContext,
    source: ImageDesc,
    earlier: &'a [&'a str],
    kept: u64,
    limits: &'a Limits,
}

impl Tiles<'_> {
    /// The plans of the round of `op`, from the untiled plan, and the bytes of the largest
    /// one's buffers beside the results held: tiles of its stage, the largest square ones,
    /// halving from 8192 to [`SMALLEST_TILE`], whose buffers, their source among them as
    /// an export's tiles count their input, fit the budget beside the results and bind,
    /// for an operation that opts in, or the whole stage for any other (decision 15). The
    /// round's scratch the detail does not keep, haze removal's copy of the image among
    /// it, is the round's own.
    fn plan(&self, op: &PlannedOperation) -> Result<(Vec<Plan>, u64), ExportError> {
        let own = |plan: &Plan| plan.bytes() - export::held_bytes(plan, self.earlier) - self.kept;
        let largest = |plans: &[Plan]| plans.iter().map(own).max().unwrap_or(0);
        let [width, height] = op.extent;
        if !takes_a_first_round(op) {
            let plans = self.at(op, &[[0, 0, width, height]])?;
            let largest = largest(&plans);
            return Ok((plans, largest));
        }
        let mut edge = 8192;
        while edge >= SMALLEST_TILE {
            let plans = self.at(op, &export::regions(op.extent, [edge, edge]))?;
            let fit =
                |plan: &Plan| own(plan) <= self.limits.budget && plan.binds_within(self.limits);
            if plans.iter().all(fit) {
                let largest = largest(&plans);
                return Ok((plans, largest));
            }
            edge /= 2;
        }
        Err(ExportError::NoTiles {
            binding: self.limits.binding,
            budget: self.limits.budget,
        })
    }

    /// The plans of `op`'s round over `regions` of its stage, the earlier results' unbound
    /// scratch taking no memory.
    fn at(&self, op: &PlannedOperation, regions: &[[u32; 4]]) -> Result<Vec<Plan>, PlanError> {
        let round = Round {
            run: op.id,
            extent: op.extent,
            regions,
            resample: None,
        };
        let through = export::instances_through(self.after, op.id);
        let mut plans =
            round.plans(&through, self.image, self.source, self.earlier, self.limits)?;
        for plan in &mut plans {
            for id in self.earlier {
                plan.drop_unbound_scratch(id);
            }
        }
        Ok(plans)
    }
}
