# 0081. Hardware acceleration: the backends, the graphics card and the API as settings

## Context

Every Apertrust process creates one wgpu instance (`gpu::instance_descriptor`), and the app's window and the engine share its device (decision 17). The instance loaded every native backend, GL among them, though the engine never chooses GL and no test runs on it. A broken OpenGL driver then stops the app at start: AMD's 32.0.32015.7029 (Radeon 780M) crashes in `atio6axx.dll` as wgpu lists adapters whenever GL loads beside Direct3D 12 or Vulkan, in the app, the CLI and the tests alike.

The graphics card was a text setting, `engine.adapter`, matched against adapter names, so it could not tell one GPU on Direct3D 12 from the same GPU on Vulkan, and nobody could see what there was to choose. The API could only be chosen with wgpu's `WGPU_BACKEND`, which no user knows. On Windows wgpu chooses Vulkan wherever the GPU has a Vulkan driver.

## Decision

**The default backends are wgpu's primary set, never GL:** Direct3D 12 and Vulkan on Windows, Vulkan on Linux, Metal on macOS. `WGPU_BACKEND` still chooses others, GL among them, for experiments.

**Settings › Processing has one group, Hardware acceleration,** with every change applying after a restart, since the window's device is the engine's:

| Row | Setting | What it holds |
| --- | --- | --- |
| Graphics card | `engine.adapter` | A list: Automatic, then each hardware adapter the chosen API finds by name, in the order Automatic ranks them, then a separator and **CPU (no graphics card), slow**, the software adapter, whatever the platform calls it (WARP, lavapipe). Stored as the adapter's name, as before, Automatic as the empty text and CPU as the reserved `cpu` |
| Graphics API | `engine.backend` (new), Windows only (`platforms`) | Automatic, Direct3D 12, Vulkan. Hidden where the platform has one API |
| Memory budget | `engine.memory_budget` | As before |

- **Automatic** is the default of both lists: wgpu's choice, the high-performance adapter on the first backend that has it. It reads "Automatic" alone; the In use line shows what it runs.
- **The card list** holds the adapters the chosen API finds, listed as the dialog opens by a short-lived helper process, so a driver that crashes as it is loaded takes down the helper, not Apertrust, and the row says the cards could not be listed. It never loads a backend the setting leaves out. A name two adapters share, the same card under Direct3D 12 and Vulkan or two identical GPUs, is listed once. Changing the API lists them again; a card the new API lacks goes back to Automatic, saying so on the row.
- **Matching a stored name:** the adapter whose name is the stored text exactly, else, for a name typed by hand as before, the first whose name holds it; `cpu`, the software adapter of the chosen API.
- **CPU where the API has none,** Vulkan on Windows or Linux without lavapipe, is listed disabled, saying it is not found with that API; the Mac, whose Metal has none, does not list it.
- **The adapter's own name** for the software adapter is never shown: the list, the In use line and the warnings say CPU.
- **A card no longer there,** after a GPU is removed or the settings come from another computer, shows "Graphics card not found: Automatic used" in the warning colour, and Automatic runs (decision 7).
- **An "In use" line** under the rows always shows the adapter and API running now. Without a hardware GPU in use, a warning says that Apertrust renders on the processor and is slower.
- **Restart now.** While a setting that applies after a restart has a value other than the one this run started with, compared as values, the line "Your choice applies when Apertrust restarts" under the In use line has a Restart now button, the app's `restart` action: Apertrust quits as Quit does, through its own quit path (edits and settings saved, the dialog open closed as its Cancel, the quit alert asked where a job runs), and once it has ended starts again with the same flags and what is open, a photo alone by its path, a folder as what was open last.
- **A start that did not finish** opens the GPU with Automatic. Before the app opens its device it writes a marker in the user data folder naming the card and API it tries, holding it locked for the whole start, and removes it once the first frame is drawn. A marker another start holds locked is that start's, under way at the same moment, as Open With on several raws starts one process each: it is neither read nor written, and this start keeps none. A start that finds one unlocked, the last having crashed, hung or been closed with a black window, ignores the card and API settings for this run, keeps them in the file, and says so in the window, over its top on every platform: "{ app-name } did not start last time with { card } on { api }, so it started with Automatic", with Keep Automatic, which clears the two settings, and Try again, which restarts with them. Where the marker names Automatic itself, the start opens the CPU and says so, with Keep CPU and Try again; where the API has no CPU adapter (Metal, Linux without lavapipe), wgpu's choice opens again, and the notice says it started with Automatic, with Close alone. Where the user data folder cannot be written there is no marker.
- **`--safe-mode`** starts the app with Automatic for this run, the card and API settings kept in the file but ignored, as darktable's `--disable-opencl` does for OpenCL; it wins over a marker found, with no notice, and this start's marker names what it tries. Restart now keeps it only while the card and API are as the run started; Try again never does.
- **The CLI** takes `--backend NAME` (`auto`, `dx12`, `vulkan`, the setting's names; on macOS Automatic is Metal) beside `--adapter`, and `--conf engine.backend=…`. Precedence: `WGPU_BACKEND`, then the flag, then `--conf`, then the file, then Automatic; `-d settings` says which won. An unknown name typed for the run is an error.

## Options considered

- **Keeping GL:** a broken OpenGL driver stops Apertrust for a backend it never uses.
- **GL out on Windows alone:** Linux keeps a backend nothing tests; Vulkan covers every GPU Apertrust supports there, lavapipe included.
- **One combined list,** each card with its API ("RTX 5060 · Direct3D 12", "RTX 5060 · Vulkan"): one control, but every card twice and a technical word in front of everyone.
- **The API only through `WGPU_BACKEND`:** nobody finds it, and a person cannot work round a broken driver from Settings.
- **The software adapter only as a fallback:** a person whose driver misbehaves could not choose it.
- **The software adapter stored by its name:** lavapipe's name carries the LLVM version, so a Mesa update would turn the choice into "not found" and run the hardware GPU the person moved away from.
- **Listing the cards in Apertrust's own process:** a driver crashing as it loads would close Apertrust from Settings, as AMD's OpenGL driver did at start.
- **Two identical GPUs told apart by their PCI slot:** a second stored field for machines photo editing rarely runs on.
- **The note without Restart now:** a person must quit and start again by hand for a setting that only applies then.
- **No way back from a GPU setting that stops the app:** Settings, the only place to change it, is out of reach once the app cannot start; editing `settings.toml` by hand is no answer for most people.

## Consequences

- `gpu::instance_descriptor` takes the backend setting; the app builds the window's instance from the same options, so the window and the engine never disagree on the device.
- The settings schema uses decision 43's `platforms` field for the first time. It keeps `engine.adapter` a text; the Settings dialog draws its row as a list of what the helper lists.
- Restart now is the app's, an action of its own, `restart`, for every setting that applies after a restart. The app to start is found before the app quits, and where it cannot be, nothing quits and the waiting line says why. The new process starts as the app's last step, after its event loop has returned, so it never races the old one for a file: the same executable on Windows and Linux (on Linux `/proc/self/exe`, or the AppImage it runs from where the executable lies in its mount), on macOS the app bundle through `open -n -a` when no terminal started it, else the executable. It is given the same flags, the folders an environment variable chose among them, since `open` passes no environment, and what is open: a photo alone, a folder as what was open last, or, for nothing or one no longer there, the start window.
- The marker is `starting.toml` in the user data folder (`Folders::starting`), locked with the standard library's file lock, naming what the start tries: the settings before anything loads a driver, then what the engine's settings resolve them to, `WGPU_BACKEND` included. The engine takes the run's Automatic or CPU through `Options::instead`, which logs why under `-d settings`. The CLI writes no marker and has no `--safe-mode`.
- A crash after the first frame, as while a photo's kernels compile, is not caught: the marker has gone by then.
- `apertrust-cli gpu` says which backends were loaded and why (Automatic, the settings file, `--backend`, `--conf` or `WGPU_BACKEND`), and lists their adapters, marking the one Automatic opens and the software one as CPU under its own name, as a diagnostic tool.
