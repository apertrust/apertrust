# 0089. One instance per user data folder, a window per place

## Context

Each start of the app is a process of its own, with one main window holding one place ([0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md)). So two processes can run on one user data folder, each writing `settings.toml`, `state.toml` and the same photos' sidecars. On Windows, Explorer starts the app once for each file opened with it, so several raws opened together open as one photo each, in as many apps. A second folder cannot be open beside the first.

The questions: how many processes run, and how a second start reaches the running one; how many windows there are and what each holds; where a file opened from outside goes; how a window closes and the app quits; what a different `--user-data-dir` means; and what this asks of the engine and of the window's host. The constraints: no C library ([0005](0005-rust-and-no-c-or-c-unless-no-rust-option-exists.md)); each platform's expectations (a Mac app keeps running with no window, a Windows or Linux one quits with its last); one device shared by the engine and every window ([0017](0017-threads-and-the-engines-session.md), [0018](0018-one-submit-gate-for-the-engine-and-the-window.md)); one memory budget ([0012](0012-planning-regions-scale-and-cache-points.md)); and eframe giving AccessKit to its root window alone ([0041](0041-dialogs-as-native-modal-windows.md)).

## Decision

**One process per user data folder,** its canonical path ([0043](0043-settings-and-apertrusts-folders.md)), holding several main windows, one per open place. The CLI stays a separate process, with no instance lock. All of it is planned, in the build order below.

### The hand-off

A start that finds an instance running on its folder hands it its paths and exits.

- **The hand-off first:** a start takes or checks the lock and hands off before anything else, before the log, the settings, the GPU or a window, so a second start lives only milliseconds.

- **The lock:** `instance.lock` in the user data folder, held for the whole run with the standard library's `File::try_lock`. The start that takes it is the instance; the system releases it when the process ends, a crash included.
- **The channel:** a Unix socket on macOS and Linux, `apertrust-<hash>.sock` in the instance's `$XDG_RUNTIME_DIR` (Linux) or `$TMPDIR` (macOS), or in a directory of the user's own in `/tmp` where that is unset or the socket's path would pass its limit, and a named pipe on Windows, `\\.\pipe\apertrust-<hash>`, created for the first instance only and open to the user's own account. `<hash>` is the first 16 hex digits of BLAKE3 over the canonical folder's path. Not in the user data folder itself: a socket's path is limited to 104 bytes on macOS and 108 on Linux, and sockets do not work on network file systems.
- **Only the user's own account reaches it, both ways.**
  - **The directory is checked, never assumed:** wherever the socket goes, `$XDG_RUNTIME_DIR`, macOS's `$TMPDIR` or the fallback, the instance uses the directory only if the user owns it and no other account may write to it or enter it: mode `0700`, and no access-control list granting another account any right on it, whether to write, add, delete or enter (POSIX ACLs on Linux, macOS's extended ACLs); otherwise it falls back to `/tmp/apertrust-<uid>/`, created with mode `0700` and refused under the same check, so no other account can create or replace the socket.
  - **The socket** is created with mode `0600`, whatever the umask, and the instance refuses a connection from another user where the system gives the peer's credentials (`SO_PEERCRED` on Linux, `getpeereid` on macOS).
  - **The second start checks the instance before it sends anything:** it connects, reads the server's credentials the same way, and sends its arguments, working folder and activation token only to a server of its own user; otherwise it says on stderr that the channel is not the user's own and exits with status 1. On Windows the pipe is created as the first instance of its name, open to the user's own account alone, and the second start sends only to a server whose process runs as the same user (`GetNamedPipeServerProcessId`, then the process's token).
- **Finding the channel:** on macOS and Linux the instance writes its socket's absolute path into `instance.lock`, which it holds, once the socket is bound; a start that finds the lock held reads the path there and connects to it, never deriving it from its own environment, so a start as the same user from another session, or with its environment changed, its `$XDG_RUNTIME_DIR` or `$TMPDIR` another, still reaches it; a start as another user, root under `sudo` among them, is refused by the credentials' check below. The lock is advisory there, so the file can be read while it is held. On Windows the pipe's name comes from the hash alone, the same for every start on the folder, and nothing is written: the lock is mandatory there, and the file cannot be read while it is held.
- **The instance** binds the channel right after taking the lock, before it opens its first window, so a hand-off arriving while it starts waits and is taken once that window shows. Hand-offs arriving before that window shows join its own command line into one open, however long the start takes. A socket file left by a crash, the one `instance.lock` still names, is removed by the start that next takes the lock, before it writes its own; so is any file already at its own socket's path, which a crash between binding and writing the path leaves unnamed, since holding the lock means no instance owns it. Either is removed only when it is a socket, owned by the user, named `apertrust-<hash>.sock` for this folder's hash, in a directory that passes the check above; any other path the file names is left alone, and the start says so on stderr.
- **The message:** a format version, the second start's arguments as given, its working folder (the instance resolves relative paths against it), its activation token, and the flags it was given for one run. The answer is Taken or Refused, with the reason. A second start whose format the instance does not speak (another build on the folder) is refused, saying another version is running.
- **A second start that cannot connect** while the lock is held, the file naming no socket yet or one that refuses, retries for 2 s, reading the file again each time, then says on stderr that the instance on that folder is not answering, and exits with status 1.
- **Never through the hand-off:** `--help`, `--version` and `--list-adapters` (Settings' helper, which the instance itself starts, [0082](0082-hardware-acceleration-the-backends-the-graphics-card-and-the-api-as-settings.md)) take no lock.
- **Restart now** ([0082](0082-hardware-acceleration-the-backends-the-graphics-card-and-the-api-as-settings.md)) releases the lock and the channel before it starts the next process, so that process becomes the instance.

| Platform | An open from outside | Bringing the window forward |
| --- | --- | --- |
| macOS | LaunchServices delivers Open With, a double-click and a Dock drop to the running app as an open-documents event ([0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md)); the channel serves a start of the binary from a terminal | The instance activates itself |
| Windows | Explorer starts a process per file, each handing off. The Open With verb has `MultiSelectModel=Player`, so it is offered for any number of files. The instance joins hand-offs, its own command line included, each arriving within 500 ms of the one before, into one open. Raws opened together open together while Explorer's starts come less than 500 ms apart; a start later than that is an open of its own. While the instance is starting, every hand-off joins its own, so the interval matters only while it runs. Step 1 opens 100 raws with Open With on Windows 11, the app closed and running, and records the largest gap; above 250 ms, Explorer's `DelegateExecute` verb replaces the join before the first release | The second start calls `AllowSetForegroundWindow` with the instance's process id, which the instance sends first |
| Linux | The desktop entry passes every file, `%F`, in one start, which hands off | `XDG_ACTIVATION_TOKEN` on Wayland (xdg-activation), `DESKTOP_STARTUP_ID` on X11 |

A second start with no paths, as a launcher's click on Windows or Linux, brings the instance's key window forward.

**A start with flags for one run** (`--safe-mode`, `--conf`, `-d`, a `--cache-dir` naming another folder than the instance's, and any such flag the app gains, an adapter or `--max-binding`) while an instance runs on its folder is **refused**: the instance shows an alert in its key window, brought forward (the start window, where no window is shown), saying Apertrust is already running with that folder and must be quit first; the second start prints the same on stderr and exits with status 1. Such a start is never handed off with its flags dropped, and never runs as a second writer.

### Windows and places

- **A place** is a folder, a library, a project, or photos opened alone, one or several together ([0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md)). A main window holds one place, or none, when it is the start window. Photos opened alone are one place however many they are, so a window holds every photo opened together, never one window for each; each window is one view ([0037](0037-one-view-the-photos-tab.md)).
- **A place is never open twice.** Opening a place already open in a window brings that window forward: a folder by its canonical path, a library or project by its id, photos alone by the same set of photos.
- **One edit per photo in the process.** A photo shown in two windows, as a folder's and as a photo opened alone, or in a folder and its subfolder, has one edit, held by the engine by the photo's identity and shared by both sessions, so two windows never write one sidecar apart.

**Where an open goes:**

| Open | Goes to |
| --- | --- |
| From outside: Open With, a double-click, a drop on the Mac's Dock icon, a second start's paths | A new window, by default; with the setting `library.open_in` at `current_window`, the key window, its place replaced |
| From inside a window: Open…, Open Folder…, Open Recent, the start window, Browse, Open This Folder, a drop on the window | That window, its place replaced, as now |
| A place already open, from outside or inside, whatever `library.open_in` says | That window, brought forward, never its place replaced into another |
| File › New Window (Ctrl+N) | A new start window |

`library.open_in`, owner `app`, a choice of `new_window` (the default) and `current_window`, on Settings › General's Start group, applied at once.

**With no window shown,** as on a Mac whose last window is closed, the app running on, whatever goes to the key window (an open from outside at `current_window`, a second start with no paths, a refused start's alert) shows the start window first, which becomes the key window: so such an open opens in it, as a new window would, and the alert rises from it. On Windows and Linux a window is always shown while the instance runs.

### Closing and quitting

| | macOS | Windows and Linux |
| --- | --- | --- |
| Closing a place: File › Close, Ctrl+W on Windows and Linux, Cmd+W (Close Window) on a Mac window holding a place | That window becomes the start window, after its edit's last write | The same |
| A window's close button, other windows open | That window closes, after its edit's last write | The same |
| A window's close button on the last window | The start window shows; the app keeps running | The app quits, as Quit does |
| Close Window on a start window | It closes; the last is hidden, the app running on | — |
| Quit | Every window closes, each after its edit's last write; the quit alert asked once, in the key window | The same |

### A different user data folder

A different `--user-data-dir` is a **separate instance, not a separate application**: its own process and lock, so two starts on one folder share an instance and different folders run side by side, but the same app id, `org.apertrust.Apertrust`, icon, Dock and taskbar grouping and file associations.

- **Opens from outside go to the default folder's instance.** On Windows and Linux they start with no `--user-data-dir`, so they reach it. On macOS, where LaunchServices may deliver to any running instance, an instance on another folder hands an open-documents event arriving after its start to the default folder's instance through its channel, or starts one (`open -n -a` with the bundle and the paths) where none runs. An instance on another folder takes files only from its own command line and its own launch.
- **Its windows' titles end with the folder's name,** "Apertrust — work"; the default folder's show none. The default is the platform's folder, compared by canonical path.
- **On macOS** an instance on another folder starts from a terminal (`open -n -a Apertrust --args --user-data-dir DIR`, or the binary), a power user's path.
- **A truly separate application** is Apertrust Nightly's model, its own identity and folder ([0043](0043-settings-and-apertrusts-folders.md), [0068](0068-packages-built-by-cargo-packager.md)).
- **Two instances opening one library or folder** rely on the library cache's lock and the sidecars' rules ([0052](0052-comparing-two-copies-of-a-sidecar.md), [0054](0054-four-layers-for-the-librarys-code.md), [0076](0076-the-cache-kinds-limits-and-control.md)), as the CLI does.

### The architecture

- **The app** owns the settings store, the recents (`state.toml`), the jobs' mirror and a registry of windows. **Each window** has its place, its session, its layout state, its egui context, its AccessKit adapter and its surface.
- **The engine:** one engine thread per device serves every window's session. Each turn it takes every session's commands, renders the key window's live frame first, then one bounded submission of background work across sessions. A window that is not the key window shows stills only: its active photo as its still, live again once its window is the key window. One live photo in the process, [0037](0037-one-view-the-photos-tab.md)'s setting. The memory budget ([0012](0012-planning-regions-scale-and-cache-points.md)), the compiled kernels and the job control are the process's.
- **Settings** are written once, by the app's store, and sent to every window and session. What a window remembers of its layout (the side panel's width and tab, the grid panel, its placement) is its own as it changes, the last change written as the next window's default, never pushed to the other windows open. A new window cascades from the key window.
- **Menus** ([0039](0039-one-top-bar-native-menus-on-the-mac-one-menu-button-elsewhere.md)): on macOS one menu bar acting on the key window, File › New Window, and the Window menu listing every window by its title, Cmd+` cycling them as AppKit does; on Windows and Linux a ☰ in each window, acting on that window.
- **Dialogs and alerts** ([0041](0041-dialogs-as-native-modal-windows.md)) are parented to the window they rise from, one dialog at a time per window. Settings and the jobs window stay one each, the jobs' rows labelled with their place; asked for from another window, each comes forward where it is.
- **The host:** main windows move from eframe's single root to the app's own winit, egui-winit and egui-wgpu host, as the dialog host's windows already are, with an AccessKit adapter per window and the submit gate held round each window's callbacks ([0018](0018-one-submit-gate-for-the-engine-and-the-window.md)). A spike measures it before it is committed to; a miss reopens this part.

### Build order

Each step a pull request, reviewed before the next ([roadmap.md](../roadmap.md#the-window-and-the-shell)).

1. **The single-instance hand-off,** still one window: the lock, the channel and its path in the lock file, the message, the hand-off before anything else, the joins while starting and the 500 ms join on Windows with the gaps between Explorer's starts measured, the activation token, the refusal of flags for one run; an open from outside replaces the window's place, as [0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md) does now.
2. **A spike:** the main window in the app's own host instead of eframe, AccessKit through its adapter, measured against eframe's on every platform.
3. **Several sessions on one device,** sharing the engine thread, the budget, the kernels and one edit per photo.
4. **The shell:** New Window, the Window menu, settings to every window, the closing and quitting rules, dialogs parented to their window, the start window on closing a place.
5. **Opens from outside to a new window,** with `library.open_in`, and the user data folder in the title.

## Options considered

- **A process per start, as now:** two writers on one folder's settings, recents and sidecars, and Windows' starts per file never open together.
- **Explorer's `DelegateExecute` verb** (`IExecuteCommand`), which hands a whole selection to one call, so no join is needed: a COM server the installer registers and the app serves through `apertrust-display`, the fallback: built before the first release if step 1 measures a gap above 250 ms, half the join's interval. Not first, since it is a COM server in `unsafe` code and an installer's registration, against gaps that a hand-off made first keeps to milliseconds.
- **D-Bus and GApplication** for the single instance: GLib is C ([0005](0005-rust-and-no-c-or-c-unless-no-rust-option-exists.md)), and Linux only.
- **A single-instance or IPC crate** (`interprocess` and the like): a dependency for what the standard library's Unix sockets and a few Win32 calls in `apertrust-display` do.
- **The lock alone:** says an instance runs, but cannot hand it the paths. **The socket alone:** a socket left by a crash looks alive, and two starts race to bind it; the lock decides which start owns it.
- **The socket in the user data folder:** past the socket path's limit in a deep folder, and not on a network file system.
- **Each start deriving the socket's path from its own environment:** a start as the same user from another session, its `$XDG_RUNTIME_DIR` or `$TMPDIR` another, looks in the wrong place and finds no instance while the lock says one runs. **A file of its own beside the lock** for the path: a second file to keep in step with the lock, where the lock file already holds it.
- **One window, every open from outside replacing its place:** a second folder can never be beside the first, and an Open With throws away what was open. It is step 1's state, and the setting's choice.
- **A window per photo:** one view already shows many photos ([0037](0037-one-view-the-photos-tab.md)), and each live pipeline costs 0.8 to 4.9 GB.
- **Tabs in one window,** drawn or the Mac's window tabs: one place hidden behind another, and height taken from the photo.
- **Opens from outside to the key window by default:** what is open is replaced unasked; kept as the setting.
- **Handing a start's flags for one run to the instance, or ignoring them:** a flag typed for the run silently does nothing, where a folder or adapter typed for the run is never ignored ([0043](0043-settings-and-apertrusts-folders.md)). **Running such a start as a second instance on the folder:** a second writer.
- **A different user data folder as a separate application,** with its own id: a second Dock icon and its own file associations for a second set of settings; that is the model for a separate build, Apertrust Nightly.
- **The same closing rule on every platform:** a Mac app that quits with its last window, or a Windows or Linux app left running with none, goes against what each platform's people expect.
- **eframe's viewports for the main windows:** no AccessKit and no window handle for any but the root ([0041](0041-dialogs-as-native-modal-windows.md)).
- **An engine thread per window:** several threads submitting on one device, the budget split between them, kernels compiled per window, and two writers of a photo shown twice.

## Consequences

- [0037](0037-one-view-the-photos-tab.md): one session per window stays, every window's on one engine thread per device, the key window's photo live. [0012](0012-planning-regions-scale-and-cache-points.md): the budget and the kernels are the process's. [0017](0017-threads-and-the-engines-session.md) and [0018](0018-one-submit-gate-for-the-engine-and-the-window.md): one engine thread per device, the gate round each window.
- [0039](0039-one-top-bar-native-menus-on-the-mac-one-menu-button-elsewhere.md): New Window, the Window menu, a ☰ per window. [0041](0041-dialogs-as-native-modal-windows.md): main windows in the host, dialogs per window. [0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md): opens from outside to a new window, Windows' raws together.
- [0043](0043-settings-and-apertrusts-folders.md): one instance per user data folder, `instance.lock` in it, `library.open_in`. [0076](0076-the-cache-kinds-limits-and-control.md): the cache shared only by instances on other folders given the same cache folder, and the CLI. [0082](0082-hardware-acceleration-the-backends-the-graphics-card-and-the-api-as-settings.md): the instance lock before the start's marker; Restart now releasing it first. [0068](0068-packages-built-by-cargo-packager.md): `MultiSelectModel=Player` in `packaging/windows/installer.nsi`, one app id for every user data folder.
- Code: the app's `instance` module (the lock, the message, the join, the refusal), the channel on Unix through the standard library and the named pipe, `AllowSetForegroundWindow` and, where winit lacks it, activating an existing window with another process's token in `apertrust-display`; `library.open_in` in the schema; the `new-window` action; the app's window registry and host; the engine's sessions on one thread.
- Tests: the hand-off between two processes of the app on a temporary user data folder (taken, refused, no answer, a second start with another `$XDG_RUNTIME_DIR` or `$TMPDIR` reaching the instance through the lock file's path, a stale socket at the start's own path that the lock file does not name, as a crash between binding and writing the path leaves it; a directory, the runtime or temporary one or the `/tmp` fallback, refused when another account owns it, its mode is open to others or an ACL grants another account a right, a connection from another user refused, and a second start refusing to send to a server of another user), the join's timing as a pure function, the hash and the canonical path, the refused flags; the window rules through the app's tests; Explorer's starts, the gaps between them timed for 100 raws on a cold start, LaunchServices and the activation tokens by hand on each platform.
- Docs: [gui.md](../gui.md#instances-and-windows), [settings.md](../settings.md#user-data-folders), [cache.md](../cache.md#two-processes-at-once), [library.md](../library.md#one-photo), [packaging.md](../packaging.md#opening-a-raw-from-the-system), [engine.md](../engine.md#several-photos), [session-protocol.md](../session-protocol.md), [architecture.md](../architecture.md) and [roadmap.md](../roadmap.md#the-window-and-the-shell).
