# 0081. Hardware acceleration: the backends, the graphics card and the API as settings

## Context

Every Apertrust process creates one wgpu instance (`gpu::instance_descriptor`), and the app's window and the engine share its device (decision 17). The instance loaded every native backend, GL among them, though the engine never chooses GL and no test runs on it. A broken OpenGL driver then stops the app at start: AMD's 32.0.32015.7029 (Radeon 780M) crashes in `atio6axx.dll` as wgpu lists adapters whenever GL loads beside Direct3D 12 or Vulkan, in the app, the CLI and the tests alike.

The graphics card was a text setting, `engine.adapter`, matched against adapter names, so it could not tell one GPU on Direct3D 12 from the same GPU on Vulkan, and nobody could see what there was to choose. The API could only be chosen with wgpu's `WGPU_BACKEND`, which no user knows. On Windows wgpu chooses Vulkan wherever the GPU has a Vulkan driver.

## Decision

**The default backends are wgpu's primary set, never GL:** Direct3D 12 and Vulkan on Windows, Vulkan on Linux, Metal on macOS. `WGPU_BACKEND` still chooses others, GL among them, for experiments.

**Settings › Processing has one group, Hardware acceleration,** with every change applying after a restart, since the window's device is the engine's:

| Row | Setting | What it holds |
| --- | --- | --- |
| Graphics card | `engine.adapter` | A list: Automatic, then each adapter the chosen API finds by name, then a separator and **Software (no graphics card), slow**, the software adapter (WARP or lavapipe). Stored as the adapter's name, as before, Automatic as the empty text |
| Graphics API | `engine.backend` (new), Windows only (`platforms`) | Automatic, Direct3D 12, Vulkan. Hidden where the platform has one API |
| Memory budget | `engine.memory_budget` | As before |

- **Automatic** is the default of both lists and shows in brackets what it picks: wgpu's choice, the high-performance adapter on the first backend that has it.
- **The card list** holds the adapters the chosen API finds, listed as the dialog opens, never loading a backend the setting leaves out. Changing the API lists them again; a card the new API lacks goes back to Automatic, saying so on the row.
- **A card no longer there,** after a GPU is removed or the settings come from another computer, shows "Graphics card not found: Automatic used" in the warning colour, and Automatic runs (decision 7).
- **An "In use" line** under the rows always shows the adapter and API running now. Without a hardware GPU in use, a warning says that Apertrust renders on the processor and is slower.
- **Restart now.** While a setting that applies after a restart differs from what runs, the note "Your choice applies when Apertrust restarts" has a Restart now button: Apertrust quits as Quit does, saving edits and settings, and starts again with the same arguments, folder and photo (planned).
- **The CLI** takes `--backend NAME` (`auto`, `dx12`, `vulkan`, the setting's names; on macOS Automatic is Metal) beside `--adapter`, and `--conf engine.backend=…`. Precedence: `WGPU_BACKEND`, then the flag, then `--conf`, then the file, then Automatic; `-d settings` says which won. An unknown name typed for the run is an error.

## Options considered

- **Keeping GL:** a broken OpenGL driver stops Apertrust for a backend it never uses.
- **GL out on Windows alone:** Linux keeps a backend nothing tests; Vulkan covers every GPU Apertrust supports there, lavapipe included.
- **One combined list,** each card with its API ("RTX 5060 · Direct3D 12", "RTX 5060 · Vulkan"): one control, but every card twice and a technical word in front of everyone.
- **The API only through `WGPU_BACKEND`:** nobody finds it, and a person cannot work round a broken driver from Settings.
- **The software adapter only as a fallback:** a person whose driver misbehaves could not choose it.
- **The note without Restart now:** a person must quit and start again by hand for a setting that only applies then.

## Consequences

- `gpu::instance_descriptor` takes the backend setting; the app builds the window's instance from the same options, so the window and the engine never disagree on the device.
- The settings schema uses decision 43's `platforms` field for the first time, and a choice whose options are listed at run time, the adapters, is a new kind beside `Choice`'s fixed options.
- Restart now is the app's, an action of its own, for every setting that applies after a restart; relaunching a macOS app bundle needs a spike first.
- `apertrust-cli gpu` lists each backend's adapters and says which backends were loaded and why.
