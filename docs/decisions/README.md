# Architecture decisions

Each architecture decision has a file of its own here, named `NNNN-short-title.md`. A record says what is decided now: the context, the decision, the options considered and the consequences.

- **To add a decision,** take the next number, write a new file with the same four sections, add a line to its area's table in the index, and update the area's row in [the current state by area](#the-current-state-by-area).
- **To change a decision,** edit its record so it states what is decided now, and update the docs and code comments that describe it.
- **To replace a decision wholly,** record the new one, cut the old record to a stub naming it ("Superseded by NNNN"), and mark the old line in the index "superseded by NNNN" (decision 66).
- [architecture.md](../architecture.md) is the overview of the design, and links each area's document, which holds its details and its open questions.

## The current state by area

All major technology choices are made; one waits on a spike to be confirmed, the AI models on the WebGPU execution provider. Each row gives the current state of one area, citing its decisions by number; the records below hold the options and reasons. Update the row with the record.

| Area | Decision | Rejected |
| --- | --- | --- |
| Product | A free, non-destructive, cross-platform raw photo editor; eight principles in three groups, Trust, Ease and Help, which product decisions follow ([principles.md](../principles.md); 1) | A darktable rewrite with a friendlier UI as the identity; agents as the headline |
| Migration | darktable's colour science and modules migrated first, as darktable has them, in Apertrust's interface; the migration ends once, for the whole project, at the end of Phase 3, after which Apertrust evolves on its own with its own renders as baselines (2) | A per-tool end; darktable as the reference for good |
| Reference release | darktable's latest stable release; its number lives in one constant of the golden harness and in each manifest's provenance (3) | darktable's master |
| Tools and operations | Tools are what people work with, everything in the Tools panel and in the Import sheet's column, each tool in one place, the Tools panel or Import (86, 90), of two kinds, a processing tool editing the photo through operations and a utility tool, "utility" for short, editing nothing of its own, darktable's processing and utility modules, "section" only the Tools panel's block (93); operations are the engine's algorithms and never appear in the UI; each parameter has at most one owning tool, one no tool owns yet set only through `Set` and stored as itself; "module" only for darktable's own things and the languages' modules, "MCP tool" always in full; the code, the docs' sweep, the Fluent ids and the sidecar's names renamed in steps, the agent kit's planned, settings keys and CLI flags unchanged (4) | darktable's "module" kept; "adjustments" or "panels"; "sections" for the tools that edit nothing; an Expert tab with direct operation access |
| Language | Rust (5) | C++ |
| C and C++ libraries | None compiled in or shipped unless no Rust option exists; ONNX Runtime the one exception, loaded at run time; libraries the platform provides, loaded at run time, outside the rule (5) | lcms2, exiv2, lensfun or RawSpeed compiled in |
| Programs and platforms | Windows, macOS and Linux, first-class; two executables, `apertrust` and `apertrust-cli` (6) | Separate server and MCP programs |
| Packaging | cargo-packager from one config, `packaging/Packager.toml`: an app bundle and disk image, an NSIS installer and a zip, an AppImage, a `.deb` and a tarball, and cargo-generate-rpm's `.rpm`, the Linux packages built on Ubuntu 22.04's glibc, each with the CLI and the licences, and owns `.apxmp`; one app id for every user data folder; on Windows Open With for one file, several through Send To, one start with every file, or through Open where Apertrust is the type's default, one start per file, each handing off (89); signing and notarisation planned (68) | A script per platform; WiX; cargo-dist |
| GPU | wgpu over Vulkan, Metal, DX12 and WebGPU; a GPU-resident fp32 pipeline on storage buffers (8); the primary backends, never GL; the graphics card, the API on Windows and the memory budget in Settings › Processing › Hardware acceleration, with Restart now, a way back from a start that did not finish that leaves out the API that did not start, and `--safe-mode` (82) | A hand-written Vulkan and Metal layer; host copies between operations; GL kept; the API only through `WGPU_BACKEND` |
| Without a GPU | The same kernels on a software GPU, lavapipe or WARP; what cannot bind there is refused, naming the pass and the sizes; image-shaped scratch bound in windows before Phase 3's exit (7, 15) | A CPU version of every operation; Slang's CPU target |
| Kernels | Slang compiled to WGSL, committed and validated with naga; every differentiable kernel exports derivative entry points (9); workgroup memory written before it is read, wgpu's zeroing of it left out (80) | Plain WGSL |
| Direct3D 12 | FXC pinned in the engine; no kernel writes a local at a run-time index (10) | DXC, a C++ library to ship |
| Differentiable pipeline | Differentiable-ready from the start; a command-line test matching a reference photo is Phase 3's first engine work (11) | Forward-only, with per-tool pickers |
| Planning and caching | Regions propagated backwards; the raw stage and demosaic at full resolution, downscaled for the viewport; a few cache points; a rebuild keeps compiled kernels and matching cache points (12, 28) | Every operation over the whole image; caching every output |
| Zoomed view | The fitted viewport pipeline kept while zoomed, read by the scopes, pickers, readouts and thumbnails; a detail pipeline (`Pipe::Detail`) for the visible region with darktable's 1.2 margin at the zoom's scale, never above 1, its source demosaic's full-resolution image borrowed by key, or in a tiled viewport the raw stage's output it keeps, its start at 1 a copy of the region and below 1 its box downscale, its whole-image results from a round over the whole image at its scale kept by key; above 100% the client enlarges the 1:1 frame, smooth to 200%, square pixels from 200%; `Command::View(View { scale, centre })`, `Frame::place`, `Frame::overview` and `Event::View`, the session's view told to every client (99; its timings, one budget for both pipelines and the remedy for drags upstream of a whole-image operation planned) | One pipeline over the whole image at the zoom's scale; every zoom rendered at 100% and downscaled; the pipeline upsampled above 100%; a tile cache |
| Tiles and strips | Exports tiled after the raw stage, bit-identical to untiled ones (13); a pass larger than one binding in row strips, the viewport's demosaic in tiles (14) | The raw stage tiled from CPU uploads |
| Whole-image passes | Results of any size written into scratch at the pipe's scale, never read back; a first round of export tiles after demosaic; haze removal's estimate at full resolution (15) | A pass kind per result size |
| Views below full scale | The export's processing at the view's scale, within a measured target, rather than darktable's preview shortcuts; iterated box means with radii r and r + 1 (63) | darktable's preview pipe |
| Departures from darktable | Only by an agreed difference, listed in its port's spec, darktable's behaviour kept behind a setting only the golden harness reaches (63) | Departing wherever Apertrust's maths seems better |
| Geometry | Each operation's `Geometry`, darktable's region and point maps, interpolated in its kernel (16) | Hardware filtering |
| Pipeline order | darktable's v5.0 order; every slot unique, its number rising along the pipeline (22) | Reordering by the user |
| Fusion | Chains of per-pixel operations fused into one kernel from Phase 2, the mechanism still open; one pass per operation until then (12, 22) | — |
| Threads | A UI thread; an engine thread owning GPU submission, its frames in two display buffers; one submit gate shared with the window (17, 18) | Rendering on the UI thread; a second device |
| Opening a photo | In three stages: the best picture in hand at once, a cached rendered thumbnail or the camera's embedded picture, unmarked; the values from the sidecar and the raw's metadata before the decode, read-only, commands on the edit refused until live; then the live frame in one swap; a newer open cancelling an older, one decode at a time (77) | One blocking step; a preview mark; edits queued until live; a low-resolution first render before measuring |
| History and undo | One log of steps per virtual copy, saved in the sidecar; a step one tool's change, as darktable's history item, with its source, note and batch and the tool's whole state after it, a change of several tools a step of each sharing a batch id, Reset one step holding every tool it reset, darktable's undo merging a control's quick repeats; a photo with no history given a default step of each of darktable's default modules for a new raw, steps 1 to n, as darktable's default and auto-applied items, written with the first change, none added to a history that has steps, Undo stopping above them, the end before one leaving a tool darktable has off until its workflow applies it not applied, the first change there keeping and applying every default step where darktable drops its auto-applied items; Undo and Redo taking back the session's undo records as darktable's, a batch whole as its undo group, else moving the log's end; `Undo`, `Redo`, `GoTo`, `Compress`, `Truncate`, `Reset`, `DiscardHistory` and `Step` through the session, one whole `History` fact; Compress and Truncate keeping every default step, so neither changes what the photo shows but at end 0; Reset a step, Discard History… darktable's discard, clearing the log, the photo given its default steps again, and the way out of a view-only state for a `sidecar::Problem` ([history.md](../history.md); 47, 78; built: the log in the session's memory, Undo and Redo, `Reset` a step and the `History` fact, step 9.1; `GoTo` and `Step`, step 9.2; merging, step 9.3; the app's Edit › Undo and Redo, step 9.4; the log saved in the sidecar's version 2, Undo after reopening and a newer file taken in, step 9.5; the History tab, a batch and the default steps each one closed group, step 9.6; the default steps; `Compress`, `Truncate` and `DiscardHistory`, step 9.8, in the History section's ⋯, Truncate History an item of its own (93), and Discard History… in Edit too, asking first; `apertrust-cli edit --note` and `history`, step 9.9) | Undo in memory only; one step holding several tools; steps as deltas; a history event of deltas; Discard History as Reset; the tools every photo runs as derived, unnumbered Default lines |
| Scopes and readouts | Scopes counted on the GPU after each frame, readouts at their operations when their key changes, read back without waiting, sent as events; while zoomed, both from the fitted pipeline, the scopes counting its frame (19, 99; the scopes while zoomed built, the readouts planned) | Readback and binning on the CPU |
| Statistics of the raw | The raw's histogram counted on the GPU by the engine once per photo, only for an operation that asks (exposure's automatic mode), read back once and handed to `prepare` in the image context; the number its setup computes shown as its tool's computed value (87) | Passes of the operation binding the mosaic; a CPU count |
| Colour pickers | One active picker per session, a tool's or the Colour picker's, with stored samples beside it; regions in the photo's coordinates, kept in the original image's; reduced on the GPU by the engine's own kernel at the operation's input and output in the fitted viewport pipe and on its frame, at every zoom (99); a reading applied by the session through the tool's pure `pick` as one change of source Auto: the first reading after `On`, then one after each move, and the stored latest again after a change of a setting its picker reads; one overlay language for every mark on the photo, darktable's line; the Colour picker a utility hidden by default, its place to be decided ([pickers.md](../pickers.md); 79). Built: 79.1, the Colour picker's own picker through the session, measured on the frame; 79.2, the tools' picker declarations and `ProcessingTool::pick`; 79.3, tool pickers in the engine, measured at their operation, the mosaic and denoise included, and applied through `pick`; 79.4 to 79.6, the overlay primitives, the picker button and marks, and the app's side; 79.7, Exposure's picker; 79.8, White balance's, turned on by Settings' from image area; the rest planned | The client applying samples; a readout per operation; a host copy of the box; the zoomed view's region |
| Operations | Native plugins, one crate each (Slang source, setup, manifest, schema), built in at compile time; third-party ones later as declarative packages with WebAssembly logic (20, 21) | Loading darktable modules; native shared-library plugins |
| Raw decoding | rawler, its camera data trusted and fixed upstream; the camera named as darktable names it (23, 24) | RawSpeed, LibRaw |
| Lens correction | The corrections raws carry, first; then lensfun's database, its lookup ported to Rust and its models in Slang (25, 5) | lensfun through FFI |
| Colour management | moxcms, validated against lcms2 in CI; operations see the edit's output profile, never the display's (26) | lcms2 shipped |
| White balance | Both of darktable's: White balance over `temperature`, and Colour calibration with its CAT tab, As shot snapped as darktable snaps it (27) | One White balance driving colour calibration |
| Tool layer | Static definitions and pure bindings in `apertrust-tools`, run by the engine's session; `.apxmp` version 1 holds tool values (28); one tool model as darktable's, by composition: `ToolDefinition`, the parent (id, page, reset, presets, whether it is open at first, every processing tool closed and Collections, Scopes and History open, and menu items built; toolbar items planned), embedded in `ProcessingDefinition` with the trait `ProcessingTool` and in `UtilityDefinition` (with `measures`), built; the app's trait `UtilityTool` for a utility, its widgets drawn through one `Context`, and one header and ⋯ for every tool; one page drawer for every tool's page, a utility's of the same rows, a check refusing those that edit, its variation in custom widgets; Export's stored settings setting rows keyed by `export.*` as strings with their own words, but the path and the size line, custom rows writing their keys, its page also holding custom rows with nothing stored (Target storage, the Summary and the button) and keyless placeholders (Store masks, Intent and Style), built; a utility's presets, of every setting its page stores, its custom rows' included, planned with one presets API every tool shares (93) | Each client translating tool values; two models, each utility drawing its own header and page; one flat definition for both kinds; `apertrust-tools` depending on `apertrust-export` |
| Tool pages and controls | darktable's page: its tabs and groups, no Advanced toggle (29); a group one parent of two kinds holding its rows, a plain group darktable's section label and a collapsible group its collapsible section, a plain one inside a collapsible one the only nesting, a group first in its container drawing no space above it, so every tool's first row is 4 points under its header, or 8 under its tab strip (100); darktable's sliders (hard and soft ranges, factor, digits, step) and the options it lists per photo, Exposure darktable's slider, `exposure.exposure` itself over darktable's −18 to +18 EV, its default the photo's (30); lossless views, preset rows and curves of the same stored values (31); Exposure's page the one exception, redesigned as the wireframe's board has it, its processing and stored values darktable's: a Manual or Automatic switch, Picker target with Middle grey, Lightness and a reference kept across photos in place of Area exposure mapping and measure mode, Last pick and two warnings, labels with the camera's value, its two camera rows hidden where the photo gives them nothing to do (`ProcessingTool::visible`, no other row), no Auto (88) | A label over rows it does not hold; every group collapsible; one message id for both kinds' titles; sliders narrower than darktable's; every option on every photo; darktable's Area exposure mapping kept; rows that can never act on a photo drawn disabled |
| Switched tools | A tool darktable leaves off on a new photo stays off until touched, then takes darktable's defaults; its reset turns it off (32) | A power button; darktable's defaults as a preset |
| Tone mapping | One tone mapper on the background per edit, a tool of its own: AgX now, sigmoid and filmic rgb to come; a layer's own tone mapper replacing it within the layer's mask, planned with layers (33) | A fixed AgX; stacked tone mappers |
| Layers | Masked tool instances at their pipeline positions, in a fixed order; each slider scaled in its own unit; a mask's parts combined top to bottom, an empty mask changing nothing; retouch as layers listed between the adjustment layers and Image, applied first; darktable's blend modes; masks and retouch ported from darktable's master (34, 74, Phase 4) | Compositing at the end of the pipe; darktable's linear mix for every slider |
| UI toolkit | egui, one interface for the desktop and a desktop browser (35), in two modes, Standard and Touch (94) | Iced, Slint, GPUI, Tauri with web panels |
| Design system | `apertrust-ui`: tokens and themes in TOML, IBM Plex Sans with Plex Mono for the log's lines, Fluent and ICU4X; British and American English, Ukrainian and Polish; AccessKit on (36); the slider's marker a setting, Triangle, Circle or Square, Square by default, whole at the track's ends (75); icon-only buttons with no plates, On their filled icon (81) | egui's default look; gettext; darktable's triangle alone, cut off at the ends; plates behind icon-only buttons |
| The window's parts | Named by their role: the Toolbar, Tools (the Tools panel), the Viewer with its viewer bar, the Browser with its browser bar, and the Photos tab's first tool Collections, in the interface, the docs and the code; the settings keys `tools_panel.*`, `viewer.*` and `browser.*`, `settings.toml` version 2 moving the old keys, action ids and Browse's id as it is read, the new key winning where both are; one rename pull request, built (95) | Positional names (top, side, central and grid panel); `tools.*` for the Tools panel's keys; Browse beside the Browser; keys renamed without a migration or in a second pull request |
| One view | No separate views: the Photos tab chooses photos, the Viewer shows the Photo layout and later Map, the Browser always shown (37) | darktable's views; a Grid layout |
| Instances and windows | One process and one main window per user data folder, held by `instance.lock`, an open from outside replacing the window's place; on Windows and Linux a second start or Open With handing its paths to it through a named pipe or an abstract Unix socket, each only the user's own, and exiting, Windows' starts joined into one open; on macOS LaunchServices' opens to the running app and a second start refused; a start with flags for one run refused; another `--user-data-dir` a separate instance of the same application (89) | A process per start; several main windows, one per place; the lock alone, every second start refused; a socket file; D-Bus and GApplication; another user data folder as a separate application |
| Tools panel tabs | The Default workspace's tabs by editing stage, Photos, Adjust, Detail, Geometry, Effects, Raw and History, each listing its tools in editing order, display order only, the defaults of a workspace as 96 has it; darktable's groups no built-in workspace, a person's own file holding them (38) | darktable's groups as the Default workspace's tabs; a built-in darktable workspace; light and colour in tabs apart; one list of every tool |
| Workspaces | A workspace its tabs (id, title, icon), each tab's pinned and other tools, the tools it hides and its toolbar, kept in `<user data>/workspaces/<id>.toml`, versioned alone; Default and Essentials, the tools most edits need with the rest hidden, data files in `data/workspaces/` embedded by the app, a file written whole once changed and deleted by Workspace ▸'s Restore Defaults (planned); a person's own by writing a file; a new build's tool placed in its default tab where the workspace has it, else hidden; `settings.toml` keeping the current one's id and Show Hidden Tools, `state.toml` the tab last shown; a tool in no tab hidden, but a floating one, ⋯'s Remove Tool hiding one, its edit kept, and Show Tool in Tab ▸ bringing it back, in the computed Hidden tab shown by Show Hidden Tools, a dot on the Tools panel's ⋮ while a hidden tool changes the photo; Find Tool… (Ctrl+Shift+F) in the tabs' place, matching darktable's names and aliases too; planned: the tab bar's menu, mirrored in ⋮ (step 5, ⋮'s Show Hidden Tools and Workspace ▸ built); switched from View › Workspace; Phase 2's steps 1 to 7, the visual editor, floating panels and the Photos workspace in Phase 3 (96; built: steps 1 to 4 and 6, the files read and written, Default and Essentials, switching, the Hidden tab with Show Hidden Tools and the dot, ⋯'s layout items with their keys, screen readers' actions in the accessibility tree until AccessKit's adapters pass them on, and Find Tool…) | Workspaces in `settings.toml`; darktable's presets in its database; every tool in a tab of the Default workspace; a new tool always hidden; search filtering the tab shown; Pin alone in ⋯; Remove Tool resetting the tool; the Customise tabs dialog on fixed lists |
| Tools panel layout | Each tab's pinned and scrolling sections and which float, the workspace's, in its file (96), which a person can always edit; changed by dragging a section's header, its ⋯ menu's items (Pin Tool or Unpin Tool, Move Tool Up and Move Tool Down, Move Tool to Tab ▸, Remove Tool; 96), keys and screen readers' actions, every move with a menu route, ⋯'s Move Tool Up and Move Tool Down the single-pointer alternative to the drag WCAG 2.2's 2.5.7 asks for; display order only, every section reordered on screen, processing tools included, never reaching the engine, the edit or its history, a departure from darktable's processing-module drag; pinned by a drop in the pinned area; floating panels inside the window, belonging to no tab, docked by a drop into any tab or back where they left by × or Dock, their place and width app-wide; a tab's reset and Processing Order the ways back (83; built: ⋯'s items, their keys, and the workspace's file written, 96's step 4, screen readers' actions in the accessibility tree, reaching screen readers once AccessKit's platform adapters pass custom actions on; planned: the drag in Phase 2, floating panels in Phase 3, 96) | Reordering utilities only, as darktable; no reordering; OS windows; a placeholder or a second copy in the tab; positions per workspace; pinning only in a dialog |
| Window frame and menus | The Toolbar along the window's top edge, its title bar; the Mac's native menu bar through muda; one menu button on Windows and Linux, drawn as the app's icon with a caret or as ☰ as a setting chooses, and no mark of its own, About in the menus (39) | The system's title bar above the Toolbar; muda's Win32 menu bar; ☰ with the mark beside it opening About |
| Toolbar | The Toolbar one a person customises, everything but the caption buttons and the traffic lights moved or hidden: one list of item ids per workspace, parts, the app's actions with an icon, tools' actions offered in `ToolDefinition::toolbar` (two at most; 93) and spaces; icons by default, Toolbar Labels one app-wide setting; guards for a hidden menu button (Show Menu Button in View › Panels, which F10 reaches), save state, jobs and assistant; fixed spaces removed, then actions folding into » as it narrows, » at the end with no flexible space, the window's minimum width raised to what never folds; a right-click anywhere on it opening the toolbar menu on every platform, the system's window menu on its own key; the workspace's file the way to customise until a Customise Toolbar window, built much later (92, 96, planned) | A fixed Toolbar; a palette in the main window; the system's window menu kept on empty space; a toolbar per layout; F6 landing on an empty toolbar; a limit on fixed spaces |
| Touch | One interface in two modes, Standard and Touch: the same tabs, tools, rows, actions, selection and commands, Touch changing sizes, spacing and gestures, with two layout adaptations presenting the same parts, the Tools panel as a bottom sheet in portrait and Photos' Select mode with its batch bar; Settings › General's Interface, Automatic, Standard or Touch, with View › Touch Mode, Standard on Windows, macOS and Linux and Touch on Android; a phone's layout a later design; Touch planned. From now on no literal size in a component, nothing only on hover, no action only on a right-click, a double-click or a modifier, sliders and pickers only through their components (94) | A separate touch app; Automatic as the desktop's default |
| Popups | Native menus on the Mac for plain lists and context menus, drawn everywhere else, through one popup service (40) | Win32 popups; egui's `Popup` |
| Context menus | Only in named places, the photo, a thumbnail and a tool's ⋯ open to tools' items, the only places `apertrust-tools`' `Place` has, the surround and the rest the app's own; every item a registry action, a tool's own generated as `tool-<tool>-<action>` as the app starts; one `MenuItem` for every item, the app's included, declared as data (`ToolDefinition::menus`, 93), an action or a submenu of up to eight, one level deep, a processing tool's of its own actions, a utility's a registry action it names, as History's `reset`; at most two a place on the photo and a thumbnail; the app building every menu, its items and the tools' sorted by `(section, order)`, `Section` constants 1000 apart, a tool's from `EDIT` up to `LAYOUT` and in `EDIT` from order 200, History's Reset alone at `(EDIT, 100)`, so Reset stays first and the layout's items last, and `order` 100 apart, a separator where the section changes, a repeat checked per menu, an item that cannot run hidden on the photo and a thumbnail and disabled in ⋯; History's Reset and Orientation ▸ the first items tools add, on the photo, Discard History… in no context menu; a thumbnail's menu opening its photo first unless it is the active one, keeping a selection that holds it, and acting on the selection, its Export… saying how many and revealing the Export tool, whose button exports the selection; no right-click on a tool, its presets from the header's presets button and its menu the header's ⋯; every item reachable without a right-click (91; its first slice built) | Callbacks building menus; the focused tool alone, or a Tools ▸ submenu; a menu on a tool's header; the app's groups first and tools in pipeline order; one weight number with bands; submenus the app's alone, read from the menu bar's |
| Help | The Apertrust Guide at `guide.apertrust.org`, its own repository, Astro with Starlight on GitHub Pages, written as how-to's after Diátaxis; the app knowing only topics, a tool's `tools/<id>` from its id unless `ToolDefinition::help` overrides it, and every other place naming its own, the Guide's pages claiming them and each topic a redirect, never renamed once released; the URL `{base}/{version}/{language}/topic/{topic}/`, the version `X.Y` or `development`, the language the interface's, English both Englishes; asked once before going online; Help last in every Tools panel tool's ⋯, Apertrust Guide in the Help menu, the Import sheet's tools none until it is designed again, Help in the browser bar's options, the Tools panel's ⋮, Settings and dialogs, the `help` action with no default key; every release's Guide kept, translations through Weblate; topics checked by both repositories (97, planned) | Page paths in the app, as darktable's; darktable's help mode; F1; only the latest release's manual online; Hugo; starlight-versions; mdBook; Docusaurus |
| Actions and shortcuts | darktable's action and shortcut framework ported whole: one registry shown as darktable's tree, by stable ids (`tool-<tool>-control-<control>` and the rest), every tool control an action of its widget's kind, generated from `ToolDefinition`, with darktable's elements and effects by their words, `enable` not ported; darktable's shortcut (key, modifiers, presses, button and click, move and direction, element, effect, speed, instance, device), acting at press where nothing longer can follow; fallbacks off by default, the value node's Ctrl a tenth and Shift ten, decision 42's fine step; a person's shortcut winning over a clashing default; `[keys]` differences only, with darktable's fields as inline tables, restore points, Load default shortcuts at startup, export, Import extras and a darktable `shortcutsrc` imported; Settings › Shortcuts as darktable's page, tooltips listing every shortcut, darktable's toasts, visual mapping (Map Shortcuts, outside the Toolbar's default, in Help and on the page) and the accels window with no default key, H staying the peek; the default keymap audited against darktable's before Phase 3's defaults; Phase 2's model, tools' actions, page and tooltips, Phase 3's dispatcher, fallbacks, mapping, files and defaults, instances in Phase 4, MIDI, gamepads and scripts' action call later, on demand; planned (98) | Flattened element and effect ids; label paths as ids; darktable's `shortcutsrc` files or a `shortcuts.toml`; an `enable` element; Alt for a tenth; Assign Shortcut… in ⋯ and a built-in darktable set; H for the accels window |
| Dialogs and alerts | Every dialog a native window of its own, modal, sheets on the Mac, Settings modeless; Import and Print full-window sheets instead (86), Export a tool of the Photos tab (90); the system's alerts, Linux's drawn by the host; Wayland's parent set, the app keeping modality; a border and drop shadow drawn by the host on Wayland (41); on Linux shown before X11's modal hints; framed by the host on Windows and Linux, no icon; Export on every platform, as a tool of the Photos tab (69, 90) | egui's viewports; egui modals |
| Numbers and choices | Shown, typed and stored as darktable does: a value read-only at rest that becomes a field on a click; choices Blended, darktable's combobox, in tool panels, the Export tool, the Import sheet's tools, Boxed elsewhere (42, 86, 90); the arrows' tenth of a step on Ctrl, Option on the Mac; the wheel over a row darktable's, with its setting on by default, a plain scroll scrolling the Tools panel (42) | A value field always shown; Alt for a tenth; the wheel only scrolling the panel |
| Settings and folders | One schema, `data/settings.schema.toml`, generating typed statics per owner; `apertrust-base`; the user data folder (43), one app instance per folder (89) | Settings declared in Rust; XDG's split by kind |
| Diagnostics | tracing with darktable-style `-d` domains, short lines and a startup banner; puffin for frames (44) | Tracy, a C++ client |
| Data files | A top-level `data/`, each file embedded at build time by the crate that parses it (45) | Files read from an installed folder |
| Library | One photo for a photo opened by any route, several opened together among them, never in the recents, Open This Folder its one way to folder mode, an open from outside replacing the window's place (89); folders the unit, opened as they are, Include subfolders off and remembered per folder; each feature's data where it belongs, made on first use: a folder's albums in its hidden `.apertrust/`, the keyword tree from the sidecars, folder records in the user data folder; Move to… and Copy to… with a checked copy; an optional Library, a named collection of folders kept on this computer, or saved in one of its folders to travel; Keep Available Offline; Keep Edits on This Computer; away, never deleted (46, 101) | A database as the truth; a photo opened from the system or a dialog opening its folder; library and project files with roots and a root map; a library made of a folder |
| Edits and sidecars | `.apxmp` and the optional `.apdata` beside the photos, or kept on this computer for a folder that asks it, routed to a Library (101); version 2 holds the history as a log of steps; hybrid logical clocks, another machine's clock read more than 5 minutes past the wall clock ignored (47, 48, 49) | A central folder of edits; history in memory only; a far-future clock followed or clamped |
| Metadata | Apertrust's own XMP layer on quick-xml, its target conformance to ISO 16684-1:2019, held by its fixtures; EXIF read by rawler (50) | exiv2, exiftool-rs |
| darktable edits | Imported once every module they use is ported; ratings, labels and keywords read either way (51) | A partial import |
| Several computers | Two copies of a sidecar compared by clock vectors; diverged edits kept as a second virtual copy, "From another computer"; metadata combined field by field; a folder's and a Library's files entry by entry; no locks (52) | Keeping one side by modified time; locks |
| Finding folders and photos | Folder records on each computer, found by the volume's id and the path below the mount as a hint, the photos' fingerprints confirming, else Locate…; photos by folder, relative path and fingerprint; listing in bulk; safe writes; a queue per folder (53, 101) | Roots with an id file and a root map; volume ids or absolute paths as the identity; a whole-file hash |
| Library API | Four layers, each calling only the one below, the model a folder tree or a Library; a storage interface when a second backend comes; clients see only `LibraryCommand` and `LibraryEvent` (54) | An SQL API; a second API beside the protocol |
| Photos tab | Collections pinned, one name in every mode, its top line saying what is open, with Import… planned (86); then Filters, Keywords, Metadata, Styles and Export last (90); Filters over the open folder's index from Phase 3, Save as Smart Album into the folder's `.apertrust/`; one Keywords tool, the tree from the sidecars, its properties in the user data folder's `keywords.toml`; Metadata the file's facts and darktable's fields in one (73) | A Library tab and tool; filters only with the library's cache; a keyword list tool of its own |
| Import | Import… in folder mode, darktable's copy & import on the Import sheet (86), with no Import tool, Collections' Import… one way in (90): the source's photos in a grid, checked to import, beside Import from, Import to, Naming, Metadata, Filters and After import; Shoot name for darktable's import job; Original names and By capture date as presets of the whole sheet; already imported told by the destination alone, the base folder's whole tree less the source, by size and camera name then the fingerprint, a probable match left unchecked and shown, nothing written on the card; copy in path order as darktable's, never moving or overwriting, skipping only a byte-for-byte duplicate at its destination, the camera name kept for every renamed copy, a JPEG left behind imported beside its raw, the sidecars and darktable's `.xmp` carried, no initial rating; a card marked on the Toolbar's Import…; planned in slices 20a to 20c (85) | A record of imports on each computer; a file on the card; darktable's 1 star; the grid's selection for what is imported; whole files compared for the mark |
| Saved index | One per folder tree per computer, in the cache, kept while a folder it covers has a record and limited otherwise; a Library's search over its folders' indexes; rebuilt from the files in the background, a scan never dropping a row; its engine chosen by spike L2 (55) | A database in a folder; one index per library |
| Cache | One cache folder by kind, holding only what can be rebuilt with nothing lost: each entry one file, the file system its index, last use its modified time; thumbnails and later previews within one limit, `cache.limit`, evicted oldest first in the background, or unlimited by the person's choice (`cache.limited`); nothing removed by age; a kept folder's index and its offline thumbnails kept until the person removes them, those long unopened offered for removal (`cache.unused_after`, Never a choice); sizes, the limit and Clear on Settings › Storage and in `apertrust-cli cache`; folder records, pending changes and conflict copies in the user data folder ([cache.md](../cache.md); 76) | A per-kind index of entries; unlimited by default, as darktable's |
| Server and sync | Later, if asked: a self-hosted server, a container on a NAS that the app optionally connects to, the originals and sidecars in its folders, the desktop editing local copies and sending back sidecars, merged by clock vectors; server folders; one server, `apertrust-cli serve` (71) | A NAS share alone; a database on the server as the truth; locks |
| Export | A job of the engine's session; darktable's sizes, resample, formats, variables and disk storage; `apertrust-export` with no GPU (56) | A job thread or a second device; writing in place |
| Sheets | Import as a full-window sheet inside the main window, on every platform: a sheet bar (the heading, Select buttons, Sort, Zoom; the presets row over the column), a grid of the Browser's cells checked to go, a column of tools on the Tools panel's side, 440 points by default, a footer with a two-line summary saying what may surprise, Keep open, Close and the primary button; checks, not a selection; each tool with the tool panel's header, ↺ and ⋯ holding Move Up and Move Down, reordering only, every row a setting row; presets of the whole sheet only; Return pressing the primary button; a tool in one place, import tools never in the Tools panel (86, planned) | Import as a dialog or as a tool with the card in the Browser; the tools at the window's start; presets per tool; Cancel |
| Export tool | One utility, last in the Photos tab, darktable's export module row for row: Storage options and Format options always open, as darktable's section labels, Global options collapsible and open at first, Metadata and Multi-preset export collapsible and closed; its stored settings keyed setting rows, darktable's choices of No and Yes included, but the path and the size line, custom rows writing their keys; ↺, presets and ⋯ in its header and no preset row, Default settings, For sharing and Full size JPEG; its end, a Summary group open at first, a labelled list of the preset or Custom, file name, size, profile, format and destination, then warnings, Export N photos and a failed export's line, scrolling with it; the selection, else the active photo, view-only photos left out; Export… and Ctrl+E revealing it with its button focused, Export Again on Ctrl+Shift+E; the wheel changing only the focused row; Export in every workspace; Select › Previous Selection on Ctrl+Shift+D; no per-photo marks; its page a utility's declaration (93): its stored settings setting rows keyed by `export.*`, but the path and the size line, custom rows writing their keys; custom rows for Target storage, the Summary and the button; and keyless placeholders for Store masks, Intent and Style; its presets, of every setting it stores, planned with the presets API every tool shares; built in part: the section with its groups, path and its folder button, header, summary and button states, the keys, the export of the selection with its count, and Select › Previous Selection on Ctrl+Shift+D (90) | A sheet of export tools beside checked photos (84, superseded); five export tools; every group collapsible, or none; a Preset row; the summary's list always open, or no summary; Import as a tool too; Ctrl+E exporting at once; the end pinned; an Import tool now |
| Jobs | darktable's control jobs, in `apertrust-base`'s `jobs` module (57); each job reporting its whole row in its own events, folded by a client with no code for any kind, Try again a command the job offers (72) | A crate of its own; GPU jobs as state machines; a client's code for each kind of job |
| Clients | The engine always native; every client through the session protocol; browsers and tablets remote; one server, `apertrust-cli serve`, later, for the engine and, if asked, sync (58, 71) | In-browser processing |
| Events | One event system: each event a fact or a notice, numbered against commands, sent through one outbox, taken in by one reducer per client (59) | A global bus of signals |
| AI inference | ONNX Runtime, the WebGPU execution provider by default, vendor providers as optional packs; copies between devices for now (60) | Burn on wgpu; bundled vendor providers |
| Agents | MCP through rmcp, a thin client of the session protocol; assistants a core capability, not the headline (1, 58) | MCP with its own logic |
| Correctness | During the migration, ΔE2000 golden tests against darktable-cli (61) and a whole-pipeline exit test (62); after it, Apertrust's own renders as baselines (2) | Visual review |
| Licence | GPL-3.0-or-later; darktable's copyright and provenance kept in ported files; bundled fonts under the OFL, icons derived from Lucide under ISC and MIT, with their notices (64) | A permissive clean-room licence |
| Names and crates | Apertrust, its parts prefixed `apertrust-`; few crates (65) | — |
| Decision records | One file per decision, with its options and reasons, stating what is decided now; a record wholly replaced kept as a stub naming its successor (66) | Records amended or superseded in a chain; a replaced record deleted |
| Repository | `github.com/apertrust/apertrust` and apertrust.org; every commit signed off (DCO); AI help disclosed once, in the README and the AI policy; the agent kit in `apertrust/agent-kit`, the hooks in the main repository (67) | An `Assisted-by:` trailer on every commit |
| CI | Every pull request: lint, cargo-deny and the tests on Linux (lavapipe), clippy on Windows and macOS; Windows' tests when a listed Windows path changes, Windows' and macOS's with the `full-ci` label; every platform's tests each night, a failure opening an issue; a guard keeping the path list current (70) | Every platform's tests on every pull request; building only; paid GPU runners |

## Foundations and principles

| Number | Decision |
| --- | --- |
| [0001](0001-what-apertrust-is-the-principles.md) | What Apertrust is: the principles |
| [0002](0002-rewrite-darktable-first-then-apertrusts-own-way.md) | Rewrite darktable first, then Apertrust's own way |
| [0003](0003-the-reference-is-darktables-latest-stable-release.md) | The reference is darktable's latest stable release |
| [0074](0074-masks-ported-from-darktables-master.md) | Masks ported from darktable's master |
| [0004](0004-tools-for-people-operations-for-algorithms.md) | Tools for people, operations for algorithms |

## Language and platforms

| Number | Decision |
| --- | --- |
| [0005](0005-rust-and-no-c-or-c-unless-no-rust-option-exists.md) | Rust, and no C or C++ unless no Rust option exists |
| [0006](0006-two-programs-on-three-desktop-platforms.md) | Two programs on three desktop platforms |
| [0007](0007-a-software-gpu-instead-of-a-cpu-path.md) | A software GPU instead of a CPU path |
| [0068](0068-packages-built-by-cargo-packager.md) | Packages built by cargo-packager |

## GPU and kernels

| Number | Decision |
| --- | --- |
| [0008](0008-a-gpu-resident-fp32-pipeline-on-wgpu.md) | A GPU-resident fp32 pipeline on wgpu |
| [0009](0009-slang-for-every-kernel-compiled-to-wgsl.md) | Slang for every kernel, compiled to WGSL |
| [0010](0010-fxc-on-direct3d-12-and-no-writes-at-run-time-indices.md) | FXC on Direct3D 12, and no writes at run-time indices |
| [0011](0011-a-differentiable-pipeline-and-matching-a-reference-photo.md) | A differentiable pipeline, and matching a reference photo |
| [0080](0080-workgroup-memory-written-before-it-is-read-and-not-zeroed.md) | Workgroup memory written before it is read, and not zeroed |
| [0082](0082-hardware-acceleration-the-backends-the-graphics-card-and-the-api-as-settings.md) | Hardware acceleration: the backends, the graphics card and the API as settings |

## Engine

| Number | Decision |
| --- | --- |
| [0012](0012-planning-regions-scale-and-cache-points.md) | Planning: regions, scale and cache points |
| [0013](0013-tiled-exports-equal-to-untiled-ones.md) | Tiled exports equal to untiled ones |
| [0014](0014-row-strips-for-images-over-one-binding.md) | Row strips for images over one binding |
| [0015](0015-whole-image-passes.md) | Whole-image passes |
| [0016](0016-geometry-for-operations-that-move-pixels.md) | Geometry for operations that move pixels |
| [0017](0017-threads-and-the-engines-session.md) | Threads and the engine's session |
| [0018](0018-one-submit-gate-for-the-engine-and-the-window.md) | One submit gate for the engine and the window |
| [0019](0019-scopes-and-readouts-counted-on-the-gpu.md) | Scopes and readouts counted on the GPU |
| [0077](0077-a-photo-opens-in-stages.md) | A photo opens in stages |
| [0078](0078-undo-redo-and-the-edits-history.md) | Undo, redo and the edit's history |
| [0079](0079-colour-pickers-one-api-for-tools-and-the-colour-picker.md) | Colour pickers, one API for tools and the Colour picker |
| [0087](0087-the-raws-histogram-counted-once-for-an-operation-that-reads-it.md) | The raw's histogram counted once for an operation that reads it |
| [0099](0099-the-zoomed-view-a-detail-pipeline-beside-the-fitted-one.md) | The zoomed view: a detail pipeline beside the fitted one |

## Operations and the plugin interface

| Number | Decision |
| --- | --- |
| [0020](0020-operations-as-native-plugins-one-crate-each.md) | Operations as native plugins, one crate each |
| [0021](0021-the-operation-interface.md) | The operation interface |
| [0022](0022-pipeline-order-and-the-default-pipeline.md) | Pipeline order and the default pipeline |

## Raw stage

| Number | Decision |
| --- | --- |
| [0023](0023-rawler-its-camera-data-fixed-upstream.md) | rawler, its camera data fixed upstream |
| [0024](0024-the-cameras-data-in-the-image-context.md) | The camera's data in the image context |
| [0025](0025-lens-correction.md) | Lens correction |

## Colour

| Number | Decision |
| --- | --- |
| [0026](0026-colour-profiles-and-colour-management.md) | Colour profiles and colour management |
| [0027](0027-both-of-darktables-white-balances.md) | Both of darktable's white balances |

## Tools and bindings

| Number | Decision |
| --- | --- |
| [0028](0028-tools-bound-to-operations.md) | Tools bound to operations |
| [0029](0029-a-tool-is-darktables-page.md) | A tool is darktable's page |
| [0030](0030-controls-as-darktables.md) | Controls as darktable's |
| [0031](0031-views-and-curves-of-the-same-stored-values.md) | Views and curves of the same stored values |
| [0032](0032-tools-off-until-first-touched.md) | Tools off until first touched |
| [0033](0033-one-tone-mapper-per-edit.md) | One tone mapper per edit |
| [0034](0034-layers-masked-tool-instances-in-a-fixed-order.md) | Layers: masked tool instances in a fixed order |
| [0088](0088-exposures-page-as-redesigned.md) | Exposure's page as redesigned |
| [0093](0093-one-tool-model-processing-tools-and-utility-tools.md) | One tool model: processing tools and utility tools |
| [0100](0100-groups-plain-and-collapsible-one-parent.md) | Groups: plain and collapsible, one parent |

## Desktop UI

| Number | Decision |
| --- | --- |
| [0035](0035-egui-for-the-desktop-and-the-browser.md) | egui, for the desktop and the browser |
| [0036](0036-the-design-system.md) | The design system |
| [0037](0037-one-view-the-photos-tab.md) | One view, the Photos tab |
| [0089](0089-one-instance-and-one-window-per-user-data-folder.md) | One instance and one window per user data folder |
| [0038](0038-the-side-panels-tabs-by-editing-stage.md) | The Tools panel's tabs, by editing stage |
| [0083](0083-the-side-panels-layout-dragged-and-floating.md) | The Tools panel's layout, dragged and floating |
| [0096](0096-workspaces-as-files-hidden-tools-and-finding-a-tool.md) | Workspaces as files, hidden tools and finding a tool |
| [0039](0039-one-top-bar-native-menus-on-the-mac-one-menu-button-elsewhere.md) | One Toolbar, native menus on the Mac, one menu button elsewhere |
| [0040](0040-popup-menus-native-on-the-mac-drawn-elsewhere.md) | Popup menus: native on the Mac, drawn elsewhere |
| [0091](0091-context-menus-places-and-tools-items-as-actions.md) | Context menus: places, and tools' items as actions |
| [0092](0092-the-toolbar.md) | The Toolbar |
| [0097](0097-help-topics-and-the-apertrust-guide.md) | Help topics and the Apertrust Guide |
| [0098](0098-actions-and-shortcuts-after-darktable.md) | Actions and shortcuts after darktable |
| [0094](0094-one-interface-standard-and-touch-modes.md) | One interface, Standard and Touch modes |
| [0095](0095-the-windows-parts-named-by-their-role.md) | The window's parts named by their role |
| [0041](0041-dialogs-as-native-modal-windows.md) | Dialogs as native modal windows |
| [0069](0069-dialogs-on-windows-and-linux-framed-by-the-host.md) | Dialogs on Windows and Linux, framed by the host |
| [0042](0042-numbers-shown-typed-and-stored.md) | Numbers shown, typed and stored |
| [0084](0084-export-as-a-sheet-of-export-tools.md) | Export as a sheet of export tools, superseded by [0090](0090-export-as-a-photos-tab-tool.md) |
| [0086](0086-import-as-a-full-window-sheet.md) | Import as a full-window sheet |
| [0090](0090-export-as-a-photos-tab-tool.md) | Export as a Photos-tab tool |
| [0075](0075-the-slider-marker-a-setting.md) | The slider's marker, a setting |
| [0081](0081-icon-only-buttons-draw-no-plates-on-is-the-filled-icon.md) | Icon-only buttons draw no plates; On is the filled icon |

## Settings, folders and diagnostics

| Number | Decision |
| --- | --- |
| [0043](0043-settings-and-apertrusts-folders.md) | Settings and Apertrust's folders |
| [0044](0044-logging-the-banner-and-versions.md) | Logging, the banner and versions |
| [0045](0045-a-top-level-data-folder.md) | A top-level data folder |
| [0076](0076-the-cache-kinds-limits-and-control.md) | The cache: kinds, limits and control |

## Library and sidecars

| Number | Decision |
| --- | --- |
| [0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md) | Sidecars are the truth: one photo, folders and optional Libraries |
| [0047](0047-where-edits-live-and-what-the-sidecar-holds.md) | Where edits live, and what the sidecar holds |
| [0048](0048-the-apxmp-format.md) | The .apxmp format |
| [0049](0049-the-apdata-file-as-darktables-dtdata.md) | The .apdata file as darktable's .dtdata |
| [0050](0050-our-own-xmp-layer.md) | Our own XMP layer |
| [0051](0051-importing-darktables-edits.md) | Importing darktable's edits |
| [0052](0052-comparing-two-copies-of-a-sidecar.md) | Comparing two copies of a sidecar |
| [0053](0053-finding-folders-and-photos.md) | Finding folders and photos |
| [0054](0054-four-layers-for-the-librarys-code.md) | Four layers for the library's code |
| [0055](0055-the-librarys-cache.md) | The saved index, one per folder tree |
| [0071](0071-a-self-hosted-server-with-sync.md) | A self-hosted server with sync |
| [0073](0073-the-photos-tabs-tools-browse-first-filters-in-folder-mode.md) | The Photos tab's tools: Collections first, filters in folder mode |
| [0085](0085-import-in-folder-mode.md) | Import in folder mode |
| [0101](0101-folders-first-libraries-optional.md) | Folders first, libraries optional |

## Export and jobs

| Number | Decision |
| --- | --- |
| [0056](0056-export-as-an-engine-job.md) | Export as an engine job |
| [0057](0057-jobs-as-darktables-control-jobs.md) | Jobs as darktable's control jobs |
| [0072](0072-each-job-reports-its-own-row.md) | Each job reports its own row |

## Session protocol, events and AI

| Number | Decision |
| --- | --- |
| [0058](0058-a-native-engine-every-client-through-the-session-protocol.md) | A native engine, every client through the session protocol |
| [0059](0059-one-event-system.md) | One event system |
| [0060](0060-ai-inference-with-onnx-runtime.md) | AI inference with ONNX Runtime |

## Correctness and the golden harness

| Number | Decision |
| --- | --- |
| [0061](0061-golden-image-tests-against-darktable-cli.md) | Golden-image tests against darktable-cli |
| [0062](0062-the-whole-pipeline-comparison.md) | The whole-pipeline comparison |
| [0063](0063-departures-from-darktable-and-views-below-full-scale.md) | Departures from darktable, and views below full scale |

## Licences and the project

| Number | Decision |
| --- | --- |
| [0064](0064-licences-gplv3-darktables-provenance-and-bundled-fonts.md) | Licences: GPLv3, darktable's provenance and bundled fonts |
| [0065](0065-names-and-crates.md) | Names and crates |
| [0066](0066-one-record-for-each-decision.md) | One record for each decision |
| [0067](0067-going-public.md) | Going public |
| [0070](0070-ci-in-three-layers.md) | CI in three layers |
