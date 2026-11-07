# Plugins

How an operation plugs into the engine: the operation interface in `apertrust-plugin`, one crate per operation, manifests and parameter schemas, the passes an operation runs, and how third-party plugins are to load. The pipeline that runs them is [engine.md](engine.md); the modules people edit with, bound to operations' parameters, are its [module layer](engine.md#the-module-layer) and [modules/README.md](modules/README.md). Each crate's state is in [crates.md](crates.md).

Every operation, built-in or third-party, is a native plugin behind one interface (decision 20). An operation is what darktable's iop is without its GUI; the GUI's job goes to modules, which bind to the parameters of one or more operations (decision 4). darktable's modules are never loaded; darktable's code is the reference for reimplementing them.

**Status.** Built: the interface, and every built-in operation as a compile-time plugin in a crate of its own, listed in `apertrust-operations`, under the rules below. Planned: readouts, and third-party plugins as declarative packages.

## What an operation has

| Part | Purpose |
| --- | --- |
| Manifest | ID, version, domain (scene or display), slot (a stage and an order number, darktable's where Apertrust keeps darktable's place; decision 22), layer support, differentiable flag, neighbourhood radius at full resolution (a fixed reach, the default of `Operation::reach`, decision 13), and `default_enabled`: whether an unedited photo runs it, or only a module's binding adds it (decision 28) |
| Parameter schema | Operation parameters with ranges, units and meaning; versioned, with migrations |
| Kernels | Slang source, with one entry point per pass and, outside the raw stage, the derivative entry points (decision 21), and its generated WGSL |
| Setup | `prepare`: from the parameters and the image's context to uniforms and tables, as darktable's `commit_params` does |
| Tests | Golden cases, the gradient check and invariant tests |
| Provenance | The darktable module, its source files and the release it was ported from |

The engine builds the pipeline from manifests: the operations an unedited photo runs, and the others while a module's binding sets them. The UI builds module panels from module definitions, and MCP exposes the modules.

## One crate per operation

Decision 20, in `crates/op/`:

```text
crates/op/agx/      apertrust-op-agx
  slang/agx.slang           one entry point per pass, plus agx_backward and agx_forward_diff
  generated/agx.wgsl        committed, validated with naga
  src/setup.rs              prepare(): darktable's commit_params
  src/lib.rs                manifest, schema and its versions, provenance, impl Operation
```

The port spec stays in `docs/ports/<id>.md` and the golden cases in `apertrust-golden`.

## The interface

The interface lives in `apertrust-plugin`, as plain data plus `prepare` and without wgpu, so a declarative package can implement it later:

```rust
pub trait Operation: Send + Sync {
    fn manifest(&self) -> &'static Manifest;
    fn schema(&self) -> &'static Schema;
    fn kernel(&self) -> &'static Kernel;
    fn defaults(&self, image: &ImageContext) -> ParamValues; // the schema's unless overridden
    fn prepare(&self, params: &ParamValues, image: &ImageContext, input: &ImageDesc)
        -> Result<Prepared, NotPorted>;
}
```

## Parameters

Parameters cross the interface as data (decision 21): names mapped to numbers, whole numbers, choices or booleans, checked against the schema.

- A darktable port uses darktable's field names and ranges, an array's elements with an `_<index>` suffix, and darktable's scene-referred defaults where they differ from its introspection defaults.
- A value set from the CLI, a script, MCP or the session protocol is rejected when out of range; one loaded from a saved edit is clamped with a warning, so a photo always opens.
- Each built-in operation converts the values into its own typed settings.

## Passes

Three kinds of pass:

- **per-pixel,** which the engine can fuse unless it binds scratch (decision 13);
- **spatial,** with its radius, its operation's reach deciding regions (below);
- **whole-image** (`PassKind::WholeImage`, decision 15): passes that compute a result from the whole image at the pipe's scale, its stage at that scale as `ImageContext::whole_image` gives it (`ImageDesc::scaled_extent`), into the operation's scratch buffers, which its later passes read with no readback, whatever its size. Highlight reconstruction's statistic writes partial results per workgroup, then combines them; the tone equalizer's mask grid is another.

After demosaic, whole-image passes form runs: the passes that read the operation's input, then those that read only its scratch. Each run is one first round of an export's tiles. Their uniforms hold only what they read, so their result's key, apart from the operation's other passes, ignores parameters only the later passes use (the band curve). A pass may also declare `Strips`, how it runs in row strips where its image is larger than the GPU binds at once; a statistic's strips keep whole workgroup rows, so it stays the whole image's (decision 14).

## Readouts

Readouts (decision 19), apart from the passes, are what an operation measures of the photo for its module's panel, as darktable's GUI does from its preview pipe. Planned:

- The manifest lists their ids (`Manifest::readouts`).
- `prepare` returns, beside its passes, a `ReadoutPass` for each: one pass of its kernel binding what its passes bind and a counts buffer (`Resource::Readout`), writing nothing else, its uniforms only what it reads.
- `Operation::readout(id, words)` decodes the counts on the CPU into plain data (`ReadoutValue`, a `Histogram` first).
- The engine dispatches it in the viewport alone, only while a client asks and its key has changed, and reads it back as the scopes are read; no export, tile, fusion or derivative ever sees it. The tone equalizer's `mask_histogram` comes first ([engine.md](engine.md#readouts)).

## The reach

Decision 13. `Operation::reach(params, image)`, the pixels beyond its output region at the pipe's scale that an operation's output reads, all its passes together, as darktable's `tiling_callback` overlap. By default it is the manifest's fixed radius times the scale; non-local means' comes from its patch size, search radius, scattering and the scale. The planner calls it before `prepare` as it works regions backwards, and each `prepare` is told its output region and sees its input region through `ImageDesc`'s extent and origin (decision 16).

## Slots

Decision 22. A slot is the stage whose data the operation reads (raw, camera RGB, working space, display or output), then its order number (exposure is camera RGB 21; AgX working space 45.5; both darktable's).

- Demosaic and the tone mapper are operations within a stage, not anchors of their own.
- An operation keeps darktable's v5.0 number where it keeps darktable's place. Otherwise it takes a number between its new neighbours in that list, its spec saying why; so does one whose darktable number disagrees with its list position, as lut3d's 36, listed after filmic rgb (46.0), does. The first pass keeps darktable's place for every operation.
- No two operations share a slot, and the numbers rise with the stages. `crates/operations/tests/order.rs` holds the two rules, and every built-in operation to engine.md's [order table](engine.md#order).

## The image context

Decision 21. The image context carries:

- the developed image's size and the processing scale, so spatial operations scale their radii as darktable does with `roi->scale`;
- for a raw, the camera's metadata: sensor size, CFA, black levels, white point, as-shot white balance, D65 matrix, crop, exposure bias and a DNG's linear response limit (highlights' default threshold);
- the camera's identity as darktable's (decision 24): `make`, `model` (the main name, an alias resolved), `alias` (the name the file's camera has) and `iso`, each in darktable's spelling (decision 27), by which denoise finds its noise profile;
- planned: the camera's white balance presets from darktable's `wb_presets.json`, `white_balance_presets`, in the file's order, which temperature's defaults and the White balance module read (decision 27);
- the lens corrections the raw carries, `lens_correction`, as darktable's `exif_correction_type` and `exif_correction_data` hold them (decision 25);
- the pipe's output profile as matrices to and from XYZ D50 (`output_profile`, decision 26). colorout reports it through `Operation::output_profile(params)`, and the planner collects it before any `prepare`; `None`, without colorout, means the working profile. It is the edit's in the viewport and an export's chosen one in that export, read by filmic rgb's v6 and v7 and AgX's "export profile", never by `defaults` or a binding.

### The camera's names

The camera's identification comes from rawler's camera definitions (decision 23). The image context carries it as darktable's names mean it (decision 24):

- `make`;
- `model`, the main camera's name, an alias resolved from rawler's own embedded camera files;
- `alias`, the name the file's camera has;
- `iso`, darktable's `exif_iso`, read from rawler's EXIF by darktable's rule. Maker-note ISOs wait for a maker-note reader.

Where darktable spells a camera differently from rawler (Nikon's `Z 6_2` for rawler's `Z 6 2`, `OM System` for `OM Digital Solutions`, Fujifilm's `GFX100S`), `apertrust-raw` writes darktable's spelling into all three names from a short hand-written list, held by tests to rawler's cameras and darktable's data. So every consumer of darktable's data looks a camera up by `alias`, then `model` (decision 27).

`defaults(image)` uses the context for defaults that come from the file, as darktable's `reload_defaults` does.

## Images, bindings and scratch

Decision 21. `prepare` receives the image it gets and returns the one it produces, so rawprepare can crop and demosaic can turn a mosaic into RGBA. Each pass lists exactly what it binds, by convention input 0, output 1, uniforms 2, tables 3 to 7 and scratch buffers 10 up; an operation's passes share the scratch buffers it asks for, sized in bytes. A pass in row strips binds a window of its input and output, whose starts the 48-byte pass header carries as `input_base` and `output_base`; scratch buffers stay whole (decision 14).

## What an operation does not own

Shared colour maths is in `apertrust-color`, with the numerical helpers ports share, darktable's curve interpolation and box mean among them, each moved there once its second port is scheduled (decision 65). The engine owns the kernels that belong to no operation (downscaling, the resample, the display transform, export packing) and, from Phase 2, the fused kernels it builds from operations' per-pixel functions.

## Loading

1. **Built-in operations are compile-time plugins:** each is its own crate, and `apertrust-operations` lists them explicitly. The engine depends only on `apertrust-plugin`; the binaries hand it the list. Modular, type-safe, with no loading at run time.
2. **Third-party plugins come later as declarative packages:** manifest, schema and WGSL kernels as data, with any CPU-side logic (lookup tables, statistics for Auto) as a sandboxed WebAssembly component, and a module definition that binds the package's parameters. Portable across platforms and safe to install.
3. **No native shared-library plugins:** Rust has no stable ABI, packaging would be per platform, and a faulty plugin could crash the app.

## Rules the engine enforces for every plugin

- A plugin declares its slot; it cannot reorder the pipeline. A slot another operation has, or a number out of step with its stage, will be refused at load (decision 22).
- Kernels keep data on the GPU. Only a readout's counts, a few kilobytes, are read back, by the session, never between operations (decision 19).
- Kernels never write a local at a run-time index, which FXC fails or may fail to compile; the engine checks a plugin's kernels at load (decision 10).
- Schema versions and migrations are mandatory, so old edits keep their look. A schema version rises for a removal or a change, never for an addition, which an older build finds in an edit and opens for viewing only (decision 43; [Edits in another user data folder](settings.md#edits-in-another-user-data-folder)).
- Only differentiable plugins take part in reference matching and other optimisation features.
- One owning module per parameter holds across plugins: a module that owns a parameter another module owns, or one that does not exist, is reported as failed and left out (decision 28).

## darktable's role

Reference source and reference output only, with no loading of compiled darktable modules and no compatibility bridge. Each port follows the same workflow:

1. Extract parameters, maths and edge cases from darktable's source into a short spec, noting where Apertrust deliberately differs.
2. Write the operation's Slang source and setup from the spec.
3. Compare against `darktable-cli` renders in the golden-image harness.
4. Record provenance in the manifest, for traceability when darktable changes a module upstream and for GPL attribution.
