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

//! GPU adapter and device setup.

use apertrust_base::settings::Choice;
use thiserror::Error;

/// How the engine chooses its GPU adapter, and the device it opens on it.
#[derive(Debug, Clone, Default)]
pub struct GpuOptions {
    /// Which graphics API its instance loads: wgpu's primary backends by default.
    pub backend: Backend,
    /// Which adapter to open: wgpu's choice by default.
    pub adapter: AdapterChoice,
    /// Open the device binding at most this many bytes of a storage buffer at once,
    /// when the adapter allows more: to render, on any GPU, as on one that binds less,
    /// such as lavapipe with 128 MiB (decision 14).
    pub max_storage_binding: Option<u64>,
    /// Open the device with bound windows' offsets aligned to at least this many bytes,
    /// a power of two up to WebGPU's largest, 256, when the adapter asks for less.
    pub storage_offset_alignment: Option<u32>,
}

/// The graphics API the engine's instance loads, and a window's that shares its device
/// (decision 82), as `engine.backend`, `--backend` or `--conf engine.backend` asks.
/// `WGPU_BACKEND`, where set, comes before it ([`instance_descriptor`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Backend {
    /// wgpu's primary backends: Direct3D 12 and Vulkan on Windows, Vulkan on Linux, Metal
    /// on macOS, never GL.
    #[default]
    Auto,
    Dx12,
    Vulkan,
}

/// Each by its id, as the setting, `--conf` and `--backend` take it, in the order Settings
/// › Processing lists them on Windows, the only platform with a choice. macOS has Metal
/// alone, which Automatic is there.
impl Choice for Backend {
    const OPTIONS: &'static [(Backend, &'static str)] = &[
        (Backend::Auto, "auto"),
        (Backend::Dx12, "dx12"),
        (Backend::Vulkan, "vulkan"),
    ];
}

impl Backend {
    /// Its id, as `--backend` and `-d settings` write it.
    pub fn name(self) -> &'static str {
        let found = Backend::OPTIONS
            .iter()
            .find(|(backend, _)| *backend == self);
        found.map_or("auto", |(_, name)| name)
    }

    /// The backends an instance loads for it.
    pub fn backends(self) -> wgpu::Backends {
        match self {
            Backend::Auto => wgpu::Backends::PRIMARY,
            Backend::Dx12 => wgpu::Backends::DX12,
            Backend::Vulkan => wgpu::Backends::VULKAN,
        }
    }

    /// Whether this build of wgpu has it, so an instance can find an adapter on it:
    /// [`Backend::Auto`] always.
    pub fn built(self) -> bool {
        self.built_in(wgpu::Instance::enabled_backend_features())
    }

    /// Whether a build of wgpu with `backends` has it: [`Backend::Auto`] always.
    pub fn built_in(self, backends: wgpu::Backends) -> bool {
        self == Backend::Auto || backends.contains(self.backends())
    }
}

/// The reserved name of the chosen API's software adapter, whatever the platform calls it
/// (WARP, lavapipe), as `engine.adapter`, `--adapter` and `--conf engine.adapter` take it
/// (decision 82): stored so, a driver or Mesa update that renames it never breaks the
/// choice.
pub const CPU: &str = "cpu";

/// Which adapter the engine opens (decision 43): wgpu's choice, or one by its name, as a
/// flag or `--conf engine.adapter` asks for one run or a setting keeps it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum AdapterChoice {
    /// wgpu's: the high-performance adapter, or the one `WGPU_POWER_PREF` prefers.
    #[default]
    Any,
    /// The adapter this text names ([`matching`]): the one of exactly this name, else the
    /// first whose name contains it, ignoring case, or [`CPU`], the software adapter; as a
    /// binary's own flag (`--adapter`) or `--conf engine.adapter=NAME` asks for one run:
    /// one that names none is an error, [`GpuError::AdapterNotFound`], since what was
    /// typed for this run is never ignored.
    Named(String),
    /// The same, as the `engine.adapter` setting keeps it, which may outlive the GPU it
    /// names, after a GPU is removed or the folder moves to another machine: one that
    /// names none is warned of and wgpu's choice taken, as a software adapter is a
    /// fallback and not an error (decision 7), so a settings file never stops the engine.
    Preferred(String),
}

/// An open GPU device with its queue. A clone shares them.
#[derive(Debug, Clone)]
pub struct GpuContext {
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    /// Held round each of the engine's submissions ([`GpuContext::submit`]); a client
    /// that shares the device holds it while it may reconfigure a window's surface.
    pub gate: SubmitGate,
}

impl GpuContext {
    /// Submits `buffers` to the queue, holding the [`SubmitGate`]. Every submission of
    /// the engine's goes through here.
    pub fn submit<I: IntoIterator<Item = wgpu::CommandBuffer>>(
        &self,
        buffers: I,
    ) -> wgpu::SubmissionIndex {
        let _held = self.gate.hold();
        self.queue.submit(buffers)
    }
}

/// Keeps the engine's submissions from overlapping a reconfigure of a window's surface
/// on the same device. wgpu-core 30.0.1 configures a surface only once the device's
/// queue is empty, and fails, a failure wgpu's default error handler turns into a panic,
/// when another thread submitted while it waited. A window's
/// toolkit reconfigures its surface on the UI thread whenever the window resizes, so a
/// client that shares its device with the engine holds the gate while its toolkit runs,
/// and the engine holds it round each submission. The thread holding it may hold it
/// again, as a toolkit's callbacks may nest. Other threads take turns in the order they
/// asked, so a toolkit calling back with no pause between its callbacks keeps a waiting
/// submission out for the callback under way at most, never for all that follow it.
#[derive(Debug, Clone, Default)]
pub struct SubmitGate(std::sync::Arc<Gate>);

#[derive(Debug, Default)]
struct Gate {
    holder: std::sync::Mutex<Holder>,
    free: std::sync::Condvar,
}

/// Who holds the gate, and how many times over; and the turns: the next to hand out,
/// and the one the gate is for once free.
#[derive(Debug, Default)]
struct Holder {
    thread: Option<std::thread::ThreadId>,
    depth: usize,
    next: u64,
    serving: u64,
}

impl Gate {
    fn lock(&self) -> std::sync::MutexGuard<'_, Holder> {
        self.holder
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Takes a turn and waits for it, then holds the gate for this thread, `depth` times
    /// over.
    fn enter(&self, depth: usize) {
        let mut holder = self.lock();
        let turn = holder.next;
        holder.next += 1;
        while holder.serving != turn {
            holder = self
                .free
                .wait(holder)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
        holder.thread = Some(std::thread::current().id());
        holder.depth = depth;
    }

    /// Frees the gate for the next turn.
    fn leave(&self, holder: &mut Holder) {
        holder.thread = None;
        holder.depth = 0;
        holder.serving += 1;
        self.free.notify_all();
    }
}

impl SubmitGate {
    /// Holds the gate until what this returns is dropped: at once if this thread holds it
    /// already, else after the turns of every thread that asked before this one.
    pub fn hold(&self) -> Held<'_> {
        let gate = &self.0;
        let mut holder = gate.lock();
        if holder.thread == Some(std::thread::current().id()) {
            holder.depth += 1;
        } else {
            drop(holder);
            gate.enter(1);
        }
        Held(gate, std::marker::PhantomData)
    }

    /// Runs `f` with the gate free, if this thread holds it, then holds it again as
    /// often as before, also when `f` panics: for waiting on a thread that submits, such
    /// as the engine's when its session ends, from inside a toolkit's callback that holds
    /// the gate.
    pub fn released<R>(&self, f: impl FnOnce() -> R) -> R {
        /// Holds the gate again, `.1` times over, when dropped.
        struct Again<'a>(&'a Gate, usize);
        impl Drop for Again<'_> {
            fn drop(&mut self) {
                self.0.enter(self.1);
            }
        }

        let gate = &self.0;
        let mut holder = gate.lock();
        if holder.thread != Some(std::thread::current().id()) {
            drop(holder);
            return f();
        }
        let again = Again(gate, holder.depth);
        gate.leave(&mut holder);
        drop(holder);
        let result = f();
        drop(again);
        result
    }
}

/// The [`SubmitGate`], held until this is dropped; it never leaves the thread holding it.
#[must_use = "the gate is free again once this is dropped"]
pub struct Held<'a>(&'a Gate, std::marker::PhantomData<*const ()>);

impl Drop for Held<'_> {
    fn drop(&mut self) {
        let mut holder = self.0.lock();
        holder.depth -= 1;
        if holder.depth == 0 {
            self.0.leave(&mut holder);
        }
    }
}

#[derive(Debug, Error)]
pub enum GpuError {
    #[error("no usable GPU adapter")]
    NoAdapter(#[from] wgpu::RequestAdapterError),
    #[error("{}", not_found(.0))]
    AdapterNotFound(String),
    #[error("the graphics API {} is not built for {}", .0.name(), system())]
    BackendNotBuilt(Backend),
    #[error("GPU adapter {0:?} cannot draw to the window")]
    CannotDraw(String),
    #[error(
        "{}",
        if *window { "no GPU adapter can draw to the window" } else { "no usable GPU adapter" }
    )]
    NoneUsable { window: bool },
    #[error("could not open the GPU device")]
    RequestDevice(#[from] wgpu::RequestDeviceError),
}

/// The instance every Apertrust process creates, the app's included, from the same
/// `options` as its engine, so a window and the engine always load the same backends: the
/// backend `options` chooses, by default wgpu's primary backends, Direct3D 12 and Vulkan on
/// Windows, Vulkan on Linux, Metal on macOS, never GL, which the engine never chooses and a
/// broken OpenGL driver can crash in as adapters are listed (decision 82); with FXC as
/// Direct3D 12's shader compiler (decision 10). wgpu's default would use DXC wherever it
/// finds a `dxcompiler.dll`, a compiler CI never runs. wgpu's environment variables still
/// choose others, before the options, for experiments: `WGPU_BACKEND` (GL among them) and
/// `WGPU_DX12_COMPILER`.
pub fn instance_descriptor(options: &GpuOptions) -> wgpu::InstanceDescriptor {
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle_from_env();
    descriptor.backends = loaded(wgpu::Backends::from_env(), options.backend);
    descriptor.backend_options.dx12.shader_compiler = wgpu::Dx12Compiler::Fxc.with_env();
    descriptor
}

/// The backends an instance loads: `WGPU_BACKEND`'s, `asked`, where it is set, else
/// `backend`'s.
fn loaded(asked: Option<wgpu::Backends>, backend: Backend) -> wgpu::Backends {
    asked.unwrap_or(backend.backends())
}

/// Creates a wgpu instance from [`instance_descriptor`].
pub fn instance(options: &GpuOptions) -> wgpu::Instance {
    wgpu::Instance::new(instance_descriptor(options))
}

/// Whether an instance for `options` can find an adapter on the backend they choose: an
/// error where they choose one this build lacks, unless `WGPU_BACKEND` chooses in their
/// place. The app checks it before its window opens; [`GpuContext::new`] checks it too.
pub fn check_backend(options: &GpuOptions) -> Result<(), GpuError> {
    usable(wgpu::Backends::from_env(), options.backend)
}

/// Whether an instance that loads `backend` alone lists any adapter on this machine:
/// Vulkan finds none without a Vulkan driver, as in a virtual machine, over Remote
/// Desktop or on an old GPU, where Direct3D 12 still has WARP. Creates an instance for it,
/// so it is asked only of a backend the settings file names, and only where
/// `WGPU_BACKEND`, which the instance would load in its place, is not set.
pub fn finds_adapter(backend: Backend) -> bool {
    !listed(backend).is_empty()
}

/// [`check_backend`], with `WGPU_BACKEND`'s backends `asked`.
fn usable(asked: Option<wgpu::Backends>, backend: Backend) -> Result<(), GpuError> {
    if asked.is_none() && !backend.built() {
        return Err(GpuError::BackendNotBuilt(backend));
    }
    Ok(())
}

/// Lists every adapter the instance can see, software ones included.
pub async fn adapters(instance: &wgpu::Instance) -> Vec<wgpu::Adapter> {
    instance.enumerate_adapters(wgpu::Backends::all()).await
}

/// Every adapter an instance that loads `backend` lists, software ones included, in
/// wgpu's order, as Settings' list of graphics cards takes them from a helper process
/// (decision 82). `WGPU_BACKEND`, where set, chooses the backends in its place, as for the
/// engine's own instance.
pub fn listed(backend: Backend) -> Vec<wgpu::AdapterInfo> {
    let options = GpuOptions {
        backend,
        ..GpuOptions::default()
    };
    let all = pollster::block_on(adapters(&instance(&options)));
    all.iter().map(wgpu::Adapter::get_info).collect()
}

/// Which of `adapters`, each by its name and kind, the text `name` opens (decision 82):
/// [`CPU`], whatever its case, the first software one; else the one whose name is `name`,
/// ignoring case, else, for a name typed by hand, the first whose name contains it, so
/// that "RTX 4090" never opens an "RTX 4090 D" listed before it.
pub fn matching(adapters: &[(&str, wgpu::DeviceType)], name: &str) -> Option<usize> {
    if name.eq_ignore_ascii_case(CPU) {
        return adapters
            .iter()
            .position(|(_, kind)| *kind == wgpu::DeviceType::Cpu);
    }
    let wanted = name.to_lowercase();
    let exact = adapters
        .iter()
        .position(|(other, _)| other.to_lowercase() == wanted);
    exact.or_else(|| {
        adapters
            .iter()
            .position(|(other, _)| other.to_lowercase().contains(&wanted))
    })
}

/// The adapter of `adapters` that `name` opens ([`matching`]).
fn named<'a>(adapters: &'a [wgpu::Adapter], name: &str) -> Option<&'a wgpu::Adapter> {
    let infos: Vec<wgpu::AdapterInfo> = adapters.iter().map(wgpu::Adapter::get_info).collect();
    let listed: Vec<(&str, wgpu::DeviceType)> = infos
        .iter()
        .map(|info| (info.name.as_str(), info.device_type))
        .collect();
    matching(&listed, name).map(|at| &adapters[at])
}

/// What [`GpuError::AdapterNotFound`] says of `name`.
fn not_found(name: &str) -> String {
    if name.eq_ignore_ascii_case(CPU) {
        "no software GPU adapter (cpu) with this graphics API".to_owned()
    } else {
        format!("no GPU adapter name contains {name:?}")
    }
}

/// Warns that the `engine.adapter` setting names no adapter.
fn preferred_missing(name: &str) {
    tracing::warn!(target: "gpu", "engine.adapter {name:?} names no adapter: wgpu's choice used");
}

/// The adapter `choice` asks for among `adapters`, for a client that creates the engine's
/// device itself from the adapters its toolkit lists, as the app's window does: only those
/// that can draw to `surface`, where there is one. [`AdapterChoice::Any`], and a
/// [`AdapterChoice::Preferred`] that names none, take wgpu's choice, in the order wgpu's
/// `request_adapter` sorts them by their kind for the power preference `WGPU_POWER_PREF`
/// sets, high performance by default.
pub fn choose_adapter(
    adapters: &[wgpu::Adapter],
    surface: Option<&wgpu::Surface<'_>>,
    choice: &AdapterChoice,
) -> Result<wgpu::Adapter, GpuError> {
    choose(adapters, surface, choice, power_preference())
}

/// The power preference wgpu's choice is made for: `WGPU_POWER_PREF`'s, else high
/// performance.
pub fn power_preference() -> wgpu::PowerPreference {
    wgpu::PowerPreference::from_env().unwrap_or(wgpu::PowerPreference::HighPerformance)
}

/// Which of the adapters of `kinds`, in wgpu's order, wgpu's choice takes for
/// `preference`: the first of the best kind, as [`rank`] orders them.
pub fn automatic(kinds: &[wgpu::DeviceType], preference: wgpu::PowerPreference) -> Option<usize> {
    (0..kinds.len()).min_by_key(|at| rank(kinds[*at], preference))
}

/// [`choose_adapter`] for `preference`.
fn choose(
    adapters: &[wgpu::Adapter],
    surface: Option<&wgpu::Surface<'_>>,
    choice: &AdapterChoice,
    preference: wgpu::PowerPreference,
) -> Result<wgpu::Adapter, GpuError> {
    let draws = |adapter: &wgpu::Adapter| surface.is_none_or(|s| adapter.is_surface_supported(s));
    let usable: Vec<wgpu::Adapter> = adapters.iter().filter(|a| draws(a)).cloned().collect();
    match choice {
        AdapterChoice::Any => {}
        AdapterChoice::Named(name) => match (named(&usable, name), named(adapters, name)) {
            (Some(adapter), _) => return Ok(adapter.clone()),
            (None, Some(adapter)) => return Err(GpuError::CannotDraw(adapter.get_info().name)),
            (None, None) => return Err(GpuError::AdapterNotFound(name.clone())),
        },
        AdapterChoice::Preferred(name) => match (named(&usable, name), named(adapters, name)) {
            (Some(adapter), _) => return Ok(adapter.clone()),
            (None, Some(adapter)) => tracing::warn!(
                target: "gpu",
                "engine.adapter {name:?}: {} cannot draw to the window, wgpu's choice used",
                adapter.get_info().name
            ),
            (None, None) => preferred_missing(name),
        },
    }
    // The first of the best kind, as wgpu's stable sort leaves it.
    let kinds: Vec<wgpu::DeviceType> = usable.iter().map(|a| a.get_info().device_type).collect();
    let best = automatic(&kinds, preference).map(|at| usable[at].clone());
    best.ok_or(GpuError::NoneUsable {
        window: surface.is_some(),
    })
}

/// Where wgpu-core 30's `request_adapter` sorts an adapter of `device_type` for
/// `preference`, the lowest first: for high performance a discrete GPU, then an integrated
/// one, then any other, a virtual one and a software one; for low power an integrated GPU
/// before a discrete one; for none, every adapter as listed. Settings lists the graphics
/// cards in this order (decision 82).
pub fn rank(device_type: wgpu::DeviceType, preference: wgpu::PowerPreference) -> u8 {
    use wgpu::DeviceType::{Cpu, DiscreteGpu, IntegratedGpu, Other, VirtualGpu};
    let low_power = preference == wgpu::PowerPreference::LowPower;
    match device_type {
        _ if preference == wgpu::PowerPreference::None => 0,
        DiscreteGpu if low_power => 2,
        IntegratedGpu if low_power => 1,
        DiscreteGpu => 1,
        IntegratedGpu => 2,
        Other => 3,
        VirtualGpu => 4,
        Cpu => 5,
    }
}

/// The device the engine needs from an adapter, for whoever creates it: the engine
/// itself, or a UI that shares its device with the engine.
pub fn device_descriptor(adapter: &wgpu::Adapter) -> wgpu::DeviceDescriptor<'static> {
    device_descriptor_with(adapter, &GpuOptions::default())
}

/// [`device_descriptor`], with the limits `options` lower.
pub fn device_descriptor_with(
    adapter: &wgpu::Adapter,
    options: &GpuOptions,
) -> wgpu::DeviceDescriptor<'static> {
    // Full-resolution images need the adapter's own limits rather than the WebGPU
    // defaults: a 60 MP RGBA float32 buffer is about 960 MB.
    let mut limits = adapter.limits();
    if let Some(bytes) = options.max_storage_binding {
        limits.max_storage_buffer_binding_size = limits.max_storage_buffer_binding_size.min(bytes);
    }
    if let Some(alignment) = options.storage_offset_alignment {
        limits.min_storage_buffer_offset_alignment = limits
            .min_storage_buffer_offset_alignment
            .max(alignment.clamp(4, 256).next_power_of_two());
    }
    wgpu::DeviceDescriptor {
        label: Some("apertrust"),
        // GPU timestamps for timing passes, where the adapter has them.
        required_features: adapter.features() & wgpu::Features::TIMESTAMP_QUERY,
        required_limits: limits,
        ..Default::default()
    }
}

/// The backends this build of wgpu can use, and those it was built without, for the
/// startup banner: `Metal (Vulkan, DX12, GL not built for macOS)`.
pub fn backends() -> String {
    let built = wgpu::Instance::enabled_backend_features();
    let all = [
        (wgpu::Backends::VULKAN, "Vulkan"),
        (wgpu::Backends::METAL, "Metal"),
        (wgpu::Backends::DX12, "DX12"),
        (wgpu::Backends::GL, "GL"),
    ];
    let names = |with: bool| -> Vec<&str> {
        all.iter()
            .filter(|(backend, _)| built.contains(*backend) == with)
            .map(|(_, name)| *name)
            .collect()
    };
    let (with, without) = (names(true), names(false));
    let with = if with.is_empty() {
        "none".to_owned()
    } else {
        with.join(", ")
    };
    if without.is_empty() {
        return with;
    }
    format!("{with} ({} not built for {})", without.join(", "), system())
}

/// The platform this build is for, by its name.
fn system() -> &'static str {
    match std::env::consts::OS {
        "macos" => "macOS",
        "linux" => "Linux",
        "windows" => "Windows",
        other => other,
    }
}

/// An adapter as the gpu domain names it: `Apple M4 (Metal, integrated)`.
fn adapter_name(info: &wgpu::AdapterInfo) -> String {
    let backend = match info.backend {
        wgpu::Backend::Vulkan => "Vulkan",
        wgpu::Backend::Metal => "Metal",
        wgpu::Backend::Dx12 => "DX12",
        wgpu::Backend::Gl => "GL",
        wgpu::Backend::BrowserWebGpu => "WebGPU",
        wgpu::Backend::Noop => "no backend",
    };
    let kind = match info.device_type {
        wgpu::DeviceType::IntegratedGpu => "integrated",
        wgpu::DeviceType::DiscreteGpu => "discrete",
        wgpu::DeviceType::VirtualGpu => "virtual",
        wgpu::DeviceType::Cpu => "software",
        wgpu::DeviceType::Other => "other",
    };
    format!("{} ({backend}, {kind})", info.name)
}

/// Logs the adapters there are, `chosen` marked, with its driver and limits (the gpu
/// domain), as darktable's `-d opencl` lists its devices; `all` may be empty, as when
/// the domain is off. Warns when `chosen` is a software adapter: everything still works
/// there, only slower (decision 7).
pub fn log_adapters(all: &[wgpu::Adapter], chosen: &wgpu::Adapter) {
    let info = chosen.get_info();
    let mut listed = false;
    for adapter in all {
        let other = adapter.get_info();
        if other == info {
            listed = true;
            tracing::debug!(target: "gpu", "adapter: {}  <- chosen", adapter_name(&other));
        } else {
            tracing::debug!(target: "gpu", "adapter: {}", adapter_name(&other));
        }
    }
    if !listed {
        tracing::debug!(target: "gpu", "adapter: {}  <- chosen", adapter_name(&info));
    }
    let driver = format!("{} {}", info.driver, info.driver_info);
    if !driver.trim().is_empty() {
        tracing::debug!(target: "gpu", "driver: {}", driver.trim());
    }
    let limits = chosen.limits();
    let timestamps = chosen.features().contains(wgpu::Features::TIMESTAMP_QUERY);
    tracing::debug!(
        target: "gpu",
        "limits: buffer {} MiB, binding {} MiB, timestamps: {}",
        limits.max_buffer_size >> 20,
        limits.max_storage_buffer_binding_size >> 20,
        if timestamps { "yes" } else { "no" }
    );
    if info.device_type == wgpu::DeviceType::Cpu {
        tracing::warn!(target: "gpu", "software adapter: {}, rendering is slow", info.name);
    }
}

impl GpuContext {
    pub async fn new(options: &GpuOptions) -> Result<Self, GpuError> {
        check_backend(options)?;
        let instance = instance(options);
        // Every adapter, to look one up by name or to list them under `-d gpu`.
        let all = if options.adapter != AdapterChoice::Any
            || tracing::enabled!(target: "gpu", tracing::Level::DEBUG)
        {
            adapters(&instance).await
        } else {
            Vec::new()
        };
        let found = match &options.adapter {
            AdapterChoice::Any => None,
            AdapterChoice::Named(name) => {
                let adapter = named(&all, name).cloned();
                Some(adapter.ok_or_else(|| GpuError::AdapterNotFound(name.clone()))?)
            }
            AdapterChoice::Preferred(name) => {
                let adapter = named(&all, name).cloned();
                if adapter.is_none() {
                    preferred_missing(name);
                }
                adapter
            }
        };
        let adapter = match found {
            Some(adapter) => adapter,
            None => {
                instance
                    .request_adapter(&wgpu::RequestAdapterOptions {
                        power_preference: power_preference(),
                        ..Default::default()
                    })
                    .await?
            }
        };

        log_adapters(&all, &adapter);
        let descriptor = device_descriptor_with(&adapter, options);
        let (device, queue) = adapter.request_device(&descriptor).await?;
        let (limits, own) = (device.limits(), adapter.limits());
        if limits.max_storage_buffer_binding_size < own.max_storage_buffer_binding_size
            || limits.min_storage_buffer_offset_alignment > own.min_storage_buffer_offset_alignment
        {
            tracing::debug!(
                target: "gpu",
                "limits as asked: binding {:.1} MiB, offsets aligned to {} B",
                limits.max_storage_buffer_binding_size as f64 / (1 << 20) as f64,
                limits.min_storage_buffer_offset_alignment
            );
        }

        Ok(Self {
            adapter,
            device,
            queue,
            gate: SubmitGate::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// wgpu's default features build what each platform has (wgpu's `build.rs`).
    #[test]
    fn the_banner_names_the_backends_built_and_those_not() {
        let expected = if cfg!(target_os = "macos") {
            "Metal (Vulkan, DX12, GL not built for macOS)"
        } else if cfg!(target_os = "linux") {
            "Vulkan, GL (Metal, DX12 not built for Linux)"
        } else if cfg!(target_os = "windows") {
            "Vulkan, DX12, GL (Metal not built for Windows)"
        } else {
            return;
        };
        assert_eq!(backends(), expected);
    }

    #[test]
    fn the_submit_gate_keeps_other_threads_out_while_it_is_held() {
        use std::sync::mpsc;
        use std::time::Duration;

        let gate = SubmitGate::default();
        let held = gate.hold();
        // The holding thread may hold it again, as a toolkit's nested callbacks do.
        drop(gate.hold());
        let (entered, inside) = mpsc::channel();
        let other = {
            let gate = gate.clone();
            std::thread::spawn(move || {
                let _held = gate.hold();
                entered.send(()).expect("the test waits");
            })
        };
        // The other thread waits while the gate is held, and enters once it is free.
        assert!(inside.recv_timeout(Duration::from_millis(100)).is_err());
        drop(held);
        inside
            .recv_timeout(Duration::from_secs(5))
            .expect("the other thread enters once the gate is free");
        other.join().expect("the other thread ends");
    }

    #[test]
    fn a_thread_holding_the_gate_can_wait_for_one_that_submits() {
        let gate = SubmitGate::default();
        let held = gate.hold();
        let submitter = {
            let gate = gate.clone();
            std::thread::spawn(move || drop(gate.hold()))
        };
        // Joined with the gate held, this would never end.
        gate.released(|| submitter.join().expect("the submitter ends"));
        // Held again afterwards: another thread waits until it is dropped.
        let (entered, inside) = std::sync::mpsc::channel();
        let other = {
            let gate = gate.clone();
            std::thread::spawn(move || {
                let _held = gate.hold();
                entered.send(()).expect("the test waits");
            })
        };
        let wait = std::time::Duration::from_millis(100);
        assert!(inside.recv_timeout(wait).is_err());
        drop(held);
        inside
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("the other thread enters once the gate is free");
        other.join().expect("the other thread ends");
    }

    #[test]
    fn the_gate_is_held_again_after_a_panic_while_it_was_released() {
        let gate = SubmitGate::default();
        let held = gate.hold();
        let nested = gate.hold();
        let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            gate.released(|| panic!("while released"))
        }));
        assert!(panicked.is_err());
        // Held again, twice over: another thread waits until both are dropped.
        let (entered, inside) = std::sync::mpsc::channel();
        let other = {
            let gate = gate.clone();
            std::thread::spawn(move || {
                let _held = gate.hold();
                entered.send(()).expect("the test waits");
            })
        };
        let wait = std::time::Duration::from_millis(100);
        drop(nested);
        assert!(inside.recv_timeout(wait).is_err());
        drop(held);
        inside
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("the other thread enters once the gate is free");
        other.join().expect("the other thread ends");
    }

    impl SubmitGate {
        /// Returns once `turns` turns at the gate are asked for and not over, the holder's
        /// included: how a test knows that a thread it set going waits at the gate,
        /// without timing it. Fails after a minute rather than hang. For the gate's tests
        /// and the session's.
        pub(crate) fn until_asked(&self, turns: u64) {
            use std::time::{Duration, Instant};

            let deadline = Instant::now() + Duration::from_secs(60);
            loop {
                let asked = {
                    let holder = self.0.lock();
                    holder.next - holder.serving
                };
                if asked >= turns {
                    assert_eq!(asked, turns, "turns asked at the gate");
                    return;
                }
                assert!(
                    Instant::now() < deadline,
                    "turns asked at the gate after a minute: {asked}, not {turns}"
                );
                std::thread::sleep(Duration::from_micros(100));
            }
        }
    }

    /// A toolkit may call back with no pause between callbacks, as it does while a window
    /// resizes; a thread waiting to submit gets the next turn all the same. The test's
    /// thread is the toolkit, and each round a new thread asks while it holds the gate.
    #[test]
    fn a_waiting_thread_enters_after_the_hold_under_way_however_soon_it_is_held_again() {
        use std::sync::{Arc, Mutex};

        /// What the threads did, in the order they did it.
        #[derive(Debug, Clone, Copy, PartialEq)]
        enum Step {
            /// The toolkit holds the gate, for the nth time.
            Holds(usize),
            /// The toolkit is about to free the gate it holds for the nth time.
            Frees(usize),
            /// The thread asking in the nth round holds the gate.
            Enters(usize),
        }

        let gate = SubmitGate::default();
        let steps = Arc::new(Mutex::new(Vec::new()));
        let step = |step| steps.lock().expect("no step panics").push(step);
        let mut held = gate.hold();
        step(Step::Holds(0));
        let mut expected = vec![Step::Holds(0)];
        for round in 0..20 {
            let waiting = {
                let (gate, steps) = (gate.clone(), steps.clone());
                std::thread::spawn(move || {
                    let _held = gate.hold();
                    steps
                        .lock()
                        .expect("no step panics")
                        .push(Step::Enters(round));
                })
            };
            // The toolkit's turn and the waiting thread's.
            gate.until_asked(2);
            step(Step::Frees(round));
            drop(held);
            held = gate.hold();
            step(Step::Holds(round + 1));
            // The waiting thread entered before the toolkit held the gate again.
            expected.extend([
                Step::Frees(round),
                Step::Enters(round),
                Step::Holds(round + 1),
            ]);
            let seen = steps.lock().expect("no step panics").clone();
            assert_eq!(seen, expected);
            waiting.join().expect("the waiting thread ends");
        }
        drop(held);
    }

    /// The adapter a window takes where no setting names one that is there is the one
    /// wgpu's own `request_adapter` gives, for every power preference, on this machine's
    /// adapters.
    #[test]
    fn a_window_takes_the_adapter_wgpu_would() {
        use wgpu::PowerPreference;
        let instance = instance(&GpuOptions::default());
        let all = pollster::block_on(adapters(&instance));
        for preference in [
            PowerPreference::HighPerformance,
            PowerPreference::LowPower,
            PowerPreference::None,
        ] {
            let options = wgpu::RequestAdapterOptions {
                power_preference: preference,
                ..Default::default()
            };
            let wgpu = pollster::block_on(instance.request_adapter(&options));
            let wgpu = wgpu.expect("a GPU adapter, hardware or software");
            let ours = choose(&all, None, &AdapterChoice::Any, preference).expect("one");
            assert_eq!(ours.get_info(), wgpu.get_info(), "{preference:?}");
        }
        let none = choose(&[], None, &AdapterChoice::Any, PowerPreference::None);
        let refused = none.expect_err("no adapter").to_string();
        assert_eq!(refused, "no usable GPU adapter");
    }

    /// `rank` copies wgpu-core 30's `get_order` (`src/instance.rs`), which a machine with
    /// one kind of adapter cannot tell from another order: a newer wgpu-core must be read
    /// again.
    #[test]
    fn wgpu_core_is_still_the_one_rank_was_read_from() {
        let lock = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
        let lock = std::fs::read_to_string(lock).expect("the workspace's Cargo.lock");
        let version = lock
            .split("[[package]]")
            .find(|package| package.contains("\nname = \"wgpu-core\"\n"))
            .and_then(|package| package.lines().find_map(|l| l.strip_prefix("version = ")))
            .expect("wgpu-core in Cargo.lock");
        assert!(
            version.starts_with("\"30."),
            "wgpu-core is {version}, no longer 30.x: re-check rank against get_order in \
             wgpu-core's src/instance.rs, then this test"
        );
    }

    /// The order a window takes adapters in where no setting names one that is there, as
    /// wgpu-core 30's `request_adapter` sorts them, whatever the list's order.
    #[test]
    fn adapters_are_ranked_as_wgpu_ranks_them() {
        use wgpu::DeviceType::{Cpu, DiscreteGpu, IntegratedGpu, Other, VirtualGpu};
        use wgpu::PowerPreference;
        let sorted = |preference| {
            let mut kinds = [Cpu, VirtualGpu, Other, IntegratedGpu, DiscreteGpu];
            kinds.sort_by_key(|kind| rank(*kind, preference));
            kinds
        };
        let fast = [DiscreteGpu, IntegratedGpu, Other, VirtualGpu, Cpu];
        assert_eq!(sorted(PowerPreference::HighPerformance), fast);
        let low = [IntegratedGpu, DiscreteGpu, Other, VirtualGpu, Cpu];
        assert_eq!(sorted(PowerPreference::LowPower), low);
        let listed = [Cpu, VirtualGpu, Other, IntegratedGpu, DiscreteGpu];
        assert_eq!(sorted(PowerPreference::None), listed);
    }

    /// A flag that names no adapter is an error; a setting that names none falls back to
    /// wgpu's choice, with a warning (decision 43, as decision 7 has it), in the engine's
    /// own device and in a window's.
    #[test]
    fn an_adapter_is_found_by_name_and_a_setting_that_names_none_falls_back() {
        let all = pollster::block_on(adapters(&instance(&GpuOptions::default())));
        let first = all.first().expect("a GPU adapter, hardware or software");
        let info = first.get_info();
        // Its name in other case.
        let named = AdapterChoice::Named(info.name.to_uppercase());
        let found = choose_adapter(&all, None, &named).expect("found by its name");
        assert_eq!(found.get_info(), info);
        let none = "no adapter is called this";
        let refused = choose_adapter(&all, None, &AdapterChoice::Named(none.to_owned()));
        assert!(
            matches!(refused, Err(GpuError::AdapterNotFound(ref name)) if name == none),
            "{refused:?}"
        );
        let preferred = AdapterChoice::Preferred(none.to_owned());
        let fallback = choose_adapter(&all, None, &preferred).expect("wgpu's choice");
        let any = choose_adapter(&all, None, &AdapterChoice::Any).expect("wgpu's choice");
        assert_eq!(fallback.get_info(), any.get_info());

        let open = |adapter| {
            let options = GpuOptions {
                adapter,
                ..GpuOptions::default()
            };
            pollster::block_on(GpuContext::new(&options))
        };
        let refused = open(AdapterChoice::Named(none.to_owned()));
        assert!(
            matches!(refused, Err(GpuError::AdapterNotFound(_))),
            "{refused:?}"
        );
        let fallback = open(preferred).expect("wgpu's choice");
        let any = open(AdapterChoice::Any).expect("wgpu's choice");
        assert_eq!(fallback.adapter.get_info(), any.adapter.get_info());

        // `cpu` names this API's software adapter, where it has one, whatever its name.
        let software = all
            .iter()
            .find(|adapter| adapter.get_info().device_type == wgpu::DeviceType::Cpu);
        let cpu = AdapterChoice::Named(CPU.to_owned());
        match software {
            Some(software) => {
                let found = choose_adapter(&all, None, &cpu).expect("the software adapter");
                assert_eq!(found.get_info(), software.get_info());
            }
            None => {
                let refused = choose_adapter(&all, None, &cpu).expect_err("no software one");
                let said = "no software GPU adapter (cpu) with this graphics API";
                assert_eq!(refused.to_string(), said);
                let preferred = AdapterChoice::Preferred(CPU.to_owned());
                let fallback = choose_adapter(&all, None, &preferred).expect("wgpu's choice");
                assert_eq!(fallback.get_info(), any.adapter.get_info());
            }
        }
    }

    /// A stored name opens the adapter of exactly that name first, then, for one typed by
    /// hand, the first whose name holds it, ignoring case; `cpu` the first software
    /// adapter, whatever the platform calls it (decision 82).
    #[test]
    fn a_name_matches_exactly_first_then_by_what_it_holds_and_cpu_the_software_adapter() {
        use wgpu::DeviceType::{Cpu, DiscreteGpu, IntegratedGpu};
        let listed = [
            ("NVIDIA GeForce RTX 4090 D", DiscreteGpu),
            ("NVIDIA GeForce RTX 4090", DiscreteGpu),
            ("AMD Radeon 780M Graphics", IntegratedGpu),
            ("Microsoft Basic Render Driver", Cpu),
            ("llvmpipe (LLVM 17.0.6, 256 bits)", Cpu),
        ];
        assert_eq!(matching(&listed, "NVIDIA GeForce RTX 4090"), Some(1));
        assert_eq!(matching(&listed, "nvidia geforce rtx 4090 d"), Some(0));
        // Typed by hand, as the text field kept it: the first that holds it.
        assert_eq!(matching(&listed, "RTX 4090"), Some(0));
        assert_eq!(matching(&listed, "radeon"), Some(2));
        assert_eq!(matching(&listed, "cpu"), Some(3));
        assert_eq!(matching(&listed, "CPU"), Some(3));
        assert_eq!(matching(&listed, "llvmpipe"), Some(4));
        assert_eq!(matching(&listed, "Apple M4"), None);
        assert_eq!(matching(&listed[..3], CPU), None);
        assert_eq!(matching(&[], "radeon"), None);
    }

    /// wgpu's choice is the first adapter of the best kind for the preference: a software
    /// one only where there is nothing else.
    #[test]
    fn automatic_takes_the_first_of_the_best_kind() {
        use wgpu::DeviceType::{Cpu, DiscreteGpu, IntegratedGpu, VirtualGpu};
        use wgpu::PowerPreference::{HighPerformance, LowPower, None as Listed};
        let kinds = [Cpu, IntegratedGpu, DiscreteGpu, DiscreteGpu];
        assert_eq!(automatic(&kinds, HighPerformance), Some(2));
        assert_eq!(automatic(&kinds, LowPower), Some(1));
        assert_eq!(automatic(&kinds, Listed), Some(0));
        assert_eq!(automatic(&[Cpu, VirtualGpu], HighPerformance), Some(1));
        assert_eq!(automatic(&[Cpu], HighPerformance), Some(0));
        assert_eq!(automatic(&[], HighPerformance), None);
    }

    #[test]
    fn the_primary_backends_load_never_gl_unless_the_environment_says_otherwise() {
        let backends = instance_descriptor(&GpuOptions::default()).backends;
        match wgpu::Backends::from_env() {
            Some(asked) => assert_eq!(backends, asked),
            None => {
                assert_eq!(backends, wgpu::Backends::PRIMARY);
                assert!(!backends.contains(wgpu::Backends::GL), "{backends:?}");
            }
        }
    }

    /// Each backend loads itself alone, Automatic wgpu's primary set; `WGPU_BACKEND`, where
    /// set, comes before any of them (decision 82).
    #[test]
    fn the_backend_chosen_loads_alone_unless_wgpu_backend_is_set() {
        use wgpu::Backends;
        let each = [
            (Backend::Auto, Backends::PRIMARY),
            (Backend::Dx12, Backends::DX12),
            (Backend::Vulkan, Backends::VULKAN),
        ];
        for (backend, expected) in each {
            assert_eq!(loaded(None, backend), expected, "{backend:?}");
            assert_eq!(
                loaded(Some(Backends::GL), backend),
                Backends::GL,
                "{backend:?}"
            );
            if wgpu::Backends::from_env().is_none() {
                let options = GpuOptions {
                    backend,
                    ..GpuOptions::default()
                };
                assert_eq!(instance_descriptor(&options).backends, expected);
            }
        }
        // By id, as the setting, `--conf` and `--backend` take them: no Metal.
        for (backend, id) in Backend::OPTIONS {
            assert_eq!(Backend::from_id(id), Some(*backend));
            assert_eq!(backend.name(), *id);
        }
        assert_eq!(Backend::from_id("metal"), None);
        assert_eq!(Backend::from_id("gl"), None);
    }

    /// A backend this build lacks finds no adapter, so it is an error, unless
    /// `WGPU_BACKEND` chooses in its place; Automatic is always there. Windows builds
    /// every one.
    #[test]
    fn a_backend_this_build_lacks_is_refused_unless_wgpu_backend_chooses() {
        assert!(usable(None, Backend::Auto).is_ok());
        let built = cfg!(any(windows, target_os = "linux"));
        assert_eq!(Backend::Vulkan.built(), built);
        assert_eq!(Backend::Dx12.built(), cfg!(windows));
        let Some(lacking) = [Backend::Dx12, Backend::Vulkan]
            .into_iter()
            .find(|backend| !backend.built())
        else {
            return;
        };
        let refused = usable(None, lacking).expect_err("not built");
        let system = system();
        let named = format!(
            "the graphics API {} is not built for {system}",
            lacking.name()
        );
        assert_eq!(refused.to_string(), named);
        assert!(usable(Some(wgpu::Backends::PRIMARY), lacking).is_ok());
    }

    /// Automatic finds an adapter wherever the tests run, hardware or software; one this
    /// build lacks finds none.
    #[test]
    fn automatic_finds_an_adapter_and_a_backend_not_built_none() {
        if wgpu::Backends::from_env().is_some() {
            return;
        }
        assert!(finds_adapter(Backend::Auto));
        for backend in [Backend::Dx12, Backend::Vulkan] {
            if !backend.built() {
                assert!(!finds_adapter(backend), "{backend:?}");
            }
        }
    }

    #[test]
    fn direct3d_12_compiles_with_fxc_unless_the_environment_says_otherwise() {
        let compiler = instance_descriptor(&GpuOptions::default())
            .backend_options
            .dx12
            .shader_compiler;
        let expected = wgpu::Dx12Compiler::from_env().unwrap_or(wgpu::Dx12Compiler::Fxc);
        assert_eq!(format!("{compiler:?}"), format!("{expected:?}"));
        if std::env::var_os("WGPU_DX12_COMPILER").is_none() {
            assert!(matches!(compiler, wgpu::Dx12Compiler::Fxc), "{compiler:?}");
        }
    }
}
