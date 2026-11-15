# 0046. Sidecars are the truth: folder mode, Library and Project

## Context

How are photos organised and found, and where is the truth about an edit kept? darktable and commercial raw editors keep their library in a database; a database can be lost, cannot be synced by a file service, and ties the edits to one machine. People also work in different ways: some open one photo, some a folder, some keep one large library, some work job by job.

## Decision

**Sidecars are the truth; no database ever is.** A photo's edit and metadata live in its `.apxmp` ([0048](0048-the-apxmp-format.md)), written whenever they change; anything else, a library's cache included ([0055](0055-the-librarys-cache.md)), is rebuilt from the files.

**Four modes:** one photo, folder mode, a library and a project. A window opens one of them at a time.

**One photo: opening a photo, by any route, opens that photo alone.** Open With and a double-click on a raw or its `.apxmp` (the photo beside it; one whose photo is missing is refused with the alert naming the photo), File › Open…, raws dropped onto the window or the Mac's Dock icon, and `apertrust FILE` from a terminal all open it the same way, so the same photo never opens two ways. Opening a file means "show me this photo": a folder full of other photos, a changed recents list and a background listing are side effects nobody asked for.

- The photo opens with its edit beside it, as in folder mode, with no index beyond it, no folder tree and nothing written but its `.apxmp`, and its `.apdata` once the edit has one ([0049](0049-big-edit-data-in-an-apdata-file.md)).
- It adds nothing to the recents (Open Recent, the start window, `state.toml`) and never becomes what was open last, so the next start never reopens it (`library.reopen_last`).
- Browse's Folders holds its Open This Folder hint: **Open This Folder is the one way from the photo to its folder,** folder mode on the photo's folder with that photo active, and only then does the folder join the recents. The folder is one visible click away. Open Folder…, a recent and the start window still open a folder from one photo, leaving it, as from anything else open.
- **Several files opened together** (Open With on several raws, Open… with several, several raws dropped) open together in this mode, from one folder or several: the grid panel shows just them, the first active, still with nothing in the recents and no folder tree; Open This Folder opens the active photo's folder.
- **Every path on the app's command line is taken as a drop's,** after the window opens and off the interface's thread: a folder opens folder mode, raws and edits open one photo, and what opens nothing is refused with the alert in the window, its reason also on stderr. The registrations on Windows and Linux pass the files alone. `--in-folder`, which told Open With from a terminal's `apertrust FILE` while the two differed, has no purpose: it is accepted and ignored, left out of `--help`, until the first release is tagged, so a registration an earlier nightly build wrote still opens its files, and removed in that release, whose own registrations never passed it.
- **Built:** one photo from a terminal; Open With, a double-click, Open… and a drop open the photo's folder with it active, the registrations pass `--in-folder`, and a terminal's path is checked before the window opens, a refusal on stderr with status 2. **Planned, Phase 2:** the other ways in opening one photo, several photos together, the command line taken as a drop's and `--in-folder` ignored.

**Folder mode, built first,** starts only from a folder: File › Open Folder…, a folder dropped onto the window or the Mac's Dock icon, a recent (Open Recent, the start window, Browse's recents), Browse's Folders and ＋'s Open Folder…, `apertrust FOLDER`, and Open This Folder from one photo.

- Opening, browsing and thumbnailing write nothing beside the photos; a photo gets its `.apxmp` only once something about it changes.
- The listing and the sidecars, read 16 at a time, make an index in memory, gone when the folder closes.
- Import… copies into a folder named by darktable's patterns; New Library… makes the open folder a library in place. The CLI works this way.

**A Library,** one of any size: a folder with `Name.aplibrary` at its top, a short TOML file to double-click (`format`, `id`, `name`, `written_by`, `edits`, `photos`), and a hidden `.apertrust/` holding one TOML file per root, album and smart album, named by id, `keywords.toml` and the machines' names. `Photos/` holds photos copied in; folders elsewhere are roots, found by id ([0053](0053-finding-folders-and-photos.md)). Libraries replace darktable's workspaces.

- **Albums** hold photos chosen by hand (root, path, virtual copy) and other albums and smart albums, to any depth, and show only their own photos; a move that would make a cycle is refused.
- **Smart albums** are darktable's collect rules, filters and sort, saved, with Apertrust's own rule "in album".
- **Browse,** the Photos tab's first tool ([0073](0073-the-photos-tabs-tools-browse-first-filters-in-folder-mode.md)): Folders, Albums, Smart albums; darktable's film roll is a folder. A library and a project have all three. In one photo and folder mode Browse has Folders alone: Albums and Smart albums are not shown, since folder mode writes nothing of Apertrust's but sidecars, while albums and smart albums are saved files. Make This Folder a Library… is in Browse's options.

**The Project:** `Name.approject`, a library in the same format and code for one job, inside its folder only, with the folder roles Capture, Selects, Output and Trash; made by New Project… (from a template or not) or Export as Project…; a watched Capture folder; the job's settings in its file (the next capture's folder, name pattern and job code, the start adjustments as `.apertrust/start.apxmp`, the Output folder and export preset); templates in the user data folder; Output kept out of smart albums unless a rule names it.

**Photos are copied or referenced** in every mode, as darktable's "add to library" and "copy & import", with darktable's patterns and variables; importing writes no sidecar unless it sets something and recognises a photo by its fingerprint.

**Import library…** brings a library or project in as one album named after it, by copy (the default, into `Photos/<its name>/`, the cost shown first) or by reference; keywords merge by path; a photo already here is asked about once: Keep both or Link.

**With a server** ([0071](0071-a-self-hosted-server-with-sync.md)), the truth is still the sidecar files, kept in the server's folders; the server's index is a cache, as a library's is.

**Words:** the interface says Folder, Library, Project, Album, Smart album, Capture, Selects, Output, Trash, Virtual copy and Snapshot; never film roll, catalogue or session (the engine's word).

**darktable's library is kept feature by feature,** with its deviations listed in [library.md](../library.md#against-darktables-library).

**Planned:** the rest of one photo and folder mode through Phases 2 and 3; the Library and the Project in Phase 5; the server in Phase 7.

## Options considered

- A database as the truth, or a catalogue whose database holds the edits: lost or diverging with the files.
- An optional database the person chooses beside the sidecars: two truths.
- Session and Catalogue as two concepts; projects as containers of albums; groups of albums beside nested albums; folder roles on any album.
- Open With, a double-click, Open… and a drop opening the photo's folder with it active, since people often want the folder: every open of one photo lists a folder, changes the recents and what the next start reopens, none of which was asked for, while the folder is one click away.
- A photo from the folder already open shown in it, the folder staying open: an exception to one rule, so opening a file would do two things depending on what is open.
- One photo from a terminal alone, the system's ways opening the folder: the same photo opens two ways, and Windows and Linux need a flag to tell them apart.
- Several files from several folders cut to the first one's folder, the rest left with a notice: a set the person chose loses photos.
- Keeping `--in-folder` to send a launcher's refusals to the window and a terminal's to stderr: a flag for one difference, and the window's alert reaches a person at a terminal too. Telling a terminal apart by whether stderr is one: launchers differ, and Windows' windowed app has none.
- Removing `--in-folder` at once: a desktop entry or an installer's registration an earlier nightly build wrote would fail to open anything.
- Hiding Albums and Smart albums in folder mode: the person never learns they exist or what they need.
- A macOS package for a library: a plain folder on Windows and Linux anyway.
- One file holding everything: two machines would conflict on it.

## Consequences

- The library's code is `apertrust-library`, in four layers ([0054](0054-four-layers-for-the-librarys-code.md)); its commands are the session's ([0058](0058-a-native-engine-every-client-through-the-session-protocol.md)).
- Where edits live is [0047](0047-where-edits-live-and-what-the-sidecar-holds.md)'s; two copies are compared by [0052](0052-comparing-two-copies-of-a-sidecar.md).
- The app's one photo state holds several photos; a restart passes them again. `packaging/linux/org.apertrust.Apertrust.desktop`'s `Exec` passes `%F` alone and `packaging/windows/installer.nsi`'s commands `"%1"` alone, as `packaging/nightly-identity.txt`'s line and `crates/app/tests/packaging.rs` then hold them.
- Tethering and camera import (gphoto2) and Lua scripting would each need an exception to [0005](0005-rust-and-no-c-or-c-unless-no-rust-option-exists.md).
