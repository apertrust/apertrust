# 0046. Sidecars are the truth: folder mode, Library and Project

## Context

How are photos organised and found, and where is the truth about an edit kept? darktable and commercial raw editors keep their library in a database; a database can be lost, cannot be synced by a file service, and ties the edits to one machine. People also work in different ways: some open a folder, some keep one large library, some work job by job.

## Decision

**Sidecars are the truth; no database ever is.** A photo's edit and metadata live in its `.apxmp` ([0048](0048-the-apxmp-format.md)), written whenever they change; anything else, a library's cache included ([0055](0055-the-librarys-cache.md)), is rebuilt from the files.

**Folder mode, built first:** File › Open Folder… or the Library tab's Folders.

- Opening, browsing and thumbnailing write nothing beside the photos; a photo gets its `.apxmp` only once something about it changes.
- The listing and the sidecars, read 16 at a time, make an index in memory, gone when the folder closes.
- Import… copies into a folder named by darktable's patterns; New Library… makes the open folder a library in place. The CLI works this way.

**A Library,** one of any size: a folder with `Name.aplibrary` at its top, a short TOML file to double-click (`format`, `id`, `name`, `written_by`, `edits`, `photos`), and a hidden `.apertrust/` holding one TOML file per root, album and smart album, named by id, `keywords.toml` and the machines' names. `Photos/` holds photos copied in; folders elsewhere are roots, found by id ([0053](0053-finding-folders-and-photos.md)). A window opens one folder, library or project at a time. Libraries replace darktable's workspaces.

- **Albums** hold photos chosen by hand (root, path, virtual copy) and other albums and smart albums, to any depth, and show only their own photos; a move that would make a cycle is refused.
- **Smart albums** are darktable's collect rules, filters and sort, saved, with Apertrust's own rule "in album".
- **The Library tab:** Folders, Albums, Smart albums; darktable's film roll is a folder.

**The Project:** `Name.approject`, a library in the same format and code for one job, inside its folder only, with the folder roles Capture, Selects, Output and Trash; made by New Project… (from a template or not) or Export as Project…; a watched Capture folder; the job's settings in its file (the next capture's folder, name pattern and job code, the start adjustments as `.apertrust/start.apxmp`, the Output folder and export preset); templates in the user data folder; Output kept out of smart albums unless a rule names it.

**Photos are copied or referenced** in every mode, as darktable's "add to library" and "copy & import", with darktable's patterns and variables; importing writes no sidecar unless it sets something and recognises a photo by its fingerprint.

**Import library…** brings a library or project in as one album named after it, by copy (the default, into `Photos/<its name>/`, the cost shown first) or by reference; keywords merge by path; a photo already here is asked about once: Keep both or Link.

**Words:** the interface says Folder, Library, Project, Album, Smart album, Capture, Selects, Output, Trash, Virtual copy and Snapshot; never film roll, catalogue or session (the engine's word).

**darktable's library is kept feature by feature,** with its deviations listed in [library.md](../library.md#against-darktables-library).

**Planned:** folder mode through Phases 2 and 3; the Library and the Project in Phase 5.

## Options considered

- A database as the truth, or a catalogue whose database holds the edits: lost or diverging with the files.
- An optional database the person chooses beside the sidecars: two truths.
- Session and Catalogue as two concepts; projects as containers of albums; groups of albums beside nested albums; folder roles on any album.
- A macOS package for a library: a plain folder on Windows and Linux anyway.
- One file holding everything: two machines would conflict on it.

## Consequences

- The library's code is `apertrust-library`, in four layers ([0054](0054-four-layers-for-the-librarys-code.md)); its commands are the session's ([0058](0058-a-native-engine-every-client-through-the-session-protocol.md)).
- Where edits live is [0047](0047-where-edits-live-and-what-the-sidecar-holds.md)'s; two copies are compared by [0052](0052-comparing-two-copies-of-a-sidecar.md).
- Tethering and camera import (gphoto2) and Lua scripting would each need an exception to [0005](0005-rust-and-no-c-or-c-unless-no-rust-option-exists.md).
