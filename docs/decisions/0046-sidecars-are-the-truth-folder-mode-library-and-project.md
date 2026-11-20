# 0046. Sidecars are the truth: one photo, folders and optional Libraries

## Context

How are photos organised and found, and where is the truth about an edit kept? darktable and commercial raw editors keep their library in a database; a database can be lost, cannot be synced by a file service, and ties the edits to one machine. People also work in different ways: some open one photo, most work in their folders, some want albums and search across many folders and drives.

## Decision

**Sidecars are the truth; no database ever is.** A photo's edit and metadata live in its `.apxmp` ([0048](0048-the-apxmp-format.md)), written whenever they change; anything else, the saved index included ([0055](0055-the-librarys-cache.md)), is rebuilt from the files.

**One photo and folders, with optional Libraries** ([0101](0101-folders-first-libraries-optional.md)). The main window holds one place at a time, one photo, a folder or, from Phase 5, a Library, with one main window per user data folder ([0089](0089-one-instance-and-one-window-per-user-data-folder.md)).

**One photo: opening a photo, by any route, opens that photo alone.** Open With and a double-click on a raw or its `.apxmp` (the photo beside it; one whose photo is missing is refused with the alert naming the photo), File › Open…, raws dropped onto the window or the Mac's Dock icon, and `apertrust FILE` from a terminal all open it the same way, so the same photo never opens two ways. Opening a file means "show me this photo": a folder full of other photos, a changed recents list and a background listing are side effects nobody asked for.

- The photo opens with its edit beside it, as in folder mode, with no index beyond it, no folder tree and nothing written but its `.apxmp`, and its `.apdata` once the edit has one ([0049](0049-the-apdata-file-as-darktables-dtdata.md)).
- It adds nothing to the recents (Open Recent, the start window, `state.toml`) and never becomes what was open last, so the next start never reopens it (`library.reopen_last`).
- Collections' Folders holds its Open This Folder hint: **Open This Folder is the one way from the photo to its folder,** folder mode on the photo's folder with that photo active, and only then does the folder join the recents. The folder is one visible click away. Open Folder…, a recent and the start window still open a folder from one photo, leaving it, as from anything else open.
- **Several files opened together** (Open With on several raws, Open… with several, several raws dropped) open together in this mode, from one folder or several: the Browser shows just them, the first active, still with nothing in the recents and no folder tree; Open This Folder opens the active photo's folder.
- **Where an open goes:** every open replaces the window's place, one from outside (Open With, a double-click, a drop on the Dock icon, and on Windows and Linux a second start's paths, handed to the instance running on its user data folder, [0089](0089-one-instance-and-one-window-per-user-data-folder.md)) as one from inside (File › Open…, a drop on the window, Open This Folder).
- **Every path on the app's command line is taken as a drop's,** after the window opens and off the interface's thread: a folder opens folder mode, raws and edits open one photo, and what opens nothing is refused with the alert in the window, its reason also on stderr. The registrations on Windows and Linux pass the files alone. `--in-folder`, which told Open With from a terminal's `apertrust FILE` while the two differed, has no purpose: it is accepted and ignored, left out of `--help`, until the first release is tagged, so a registration an earlier nightly build wrote still opens its files, and removed in that release, whose own registrations never passed it.
- **Built:** every way in opening one photo, several photos opened together held together from any folders, the command line taken as a drop's, the registrations passing the files alone and `--in-folder` ignored; on Windows Open With takes one file, and several come through Send To, one start with every file, or through Open where Apertrust is the type's default, one start per file, each handing off to the instance, which joins them into one open ([0089](0089-one-instance-and-one-window-per-user-data-folder.md)).

**Folder mode, built first,** starts only from a folder: File › Open Folder…, a folder dropped onto the window or the Mac's Dock icon, a recent (Open Recent, the start window, Collections' recents), Collections' Folders and ＋'s Open Folder…, `apertrust FOLDER`, and Open This Folder from one photo.

- Opening, browsing and thumbnailing write nothing beside the photos; a photo gets its `.apxmp` only once something about it changes.
- The listing and the sidecars, read 16 at a time, make an index in memory, gone when the folder closes.
- Import… (planned) copies into a folder named by darktable's patterns ([0085](0085-import-in-folder-mode.md)).
- The CLI works on folders this way too: `library scan` built, `import` planned.
- From Phase 3 a folder shows its subfolders' photos with Include subfolders, keeps its own albums and smart albums in a hidden `.apertrust/` made with its first album or smart album, or a Library saved there, and moves and copies photos to any folder with a checked copy; how each feature's data is kept is [0101](0101-folders-first-libraries-optional.md)'s.

**A Library,** optional and planned for Phase 5: a named collection of folders from anywhere, adding search and albums across them, browsing while their drives are away, and folders found again after a mount changes. It is kept on this computer by default, with nothing written into its folders, or saved by choice in one of its own folders so that it travels with that folder ([0101](0101-folders-first-libraries-optional.md)). A folder can be in several Libraries or in none, and works the same either way. Several Libraries take the place of darktable's workspaces.

- **Albums** hold photos chosen by hand (by folder, relative path and virtual copy) and other albums and smart albums, to any depth, and show only their own photos; a move that would make a cycle is refused. A folder's albums are its own, in its `.apertrust/`; a Library's span its folders.
- **Smart albums** are darktable's collect rules, filters and sort, saved, with Apertrust's own rule "in album".
- **Collections,** the Photos tab's first tool ([0073](0073-the-photos-tabs-tools-browse-first-filters-in-folder-mode.md)): Folders, Albums and Smart albums; darktable's film roll is a folder. In one photo Collections has Folders alone; a folder has Folders, with its Albums and Smart albums from Phase 3 (the library's steps 19c and 19d); a Library has its folders' trees, its Albums and its Smart albums.

**Photos are imported by copy** into any folder, as darktable's "copy & import", with darktable's patterns and variables ([0085](0085-import-in-folder-mode.md)); a folder already on the disk is opened, or added to a Library, as it is. Importing writes no sidecar unless it sets something and recognises a photo by its size and camera name, confirmed by its fingerprint.

**With a server** ([0071](0071-a-self-hosted-server-with-sync.md), later, if asked), the truth is still the sidecar files, kept in the server's folders; the server's index is a cache, as a folder's is.

**Words:** the interface says Folder, Library, Album, Smart album, Virtual copy and Snapshot, with [0101](0101-folders-first-libraries-optional.md)'s commands; never project, film roll, catalogue, workspace for a Library, or session (the engine's word).

**darktable's library is kept feature by feature,** with its deviations listed in [library.md](../library.md#against-darktables-library).

**Planned:** the rest of one photo and folders through Phases 2 and 3; Libraries and the index across folders in Phase 5; the server later, if asked.

## Options considered

- A database as the truth, or a catalogue whose database holds the edits: lost or diverging with the files.
- An optional database the person chooses beside the sidecars: two truths.
- Session and Catalogue as two concepts; a Project for one job beside the Library, with folder roles; groups of albums beside nested albums: two containers to learn where folders and albums serve ([0101](0101-folders-first-libraries-optional.md)).
- Open With, a double-click, Open… and a drop opening the photo's folder with it active, since people often want the folder: every open of one photo lists a folder, changes the recents and what the next start reopens, none of which was asked for, while the folder is one click away.
- A photo from the folder already open shown in it, the folder staying open: an exception to one rule, so opening a file would do two things depending on what is open.
- One photo from a terminal alone, the system's ways opening the folder: the same photo opens two ways, and Windows and Linux need a flag to tell them apart.
- Several files from several folders cut to the first one's folder, the rest left with a notice: a set the person chose loses photos.
- Keeping `--in-folder` to send a launcher's refusals to the window and a terminal's to stderr: a flag for one difference, and the window's alert reaches a person at a terminal too. Telling a terminal apart by whether stderr is one: launchers differ, and Windows' windowed app has none.
- Removing `--in-folder` at once: a desktop entry or an installer's registration an earlier nightly build wrote would fail to open anything.
- A library file or folder a person makes, opens and finds again (`Name.aplibrary`), and a macOS package for one: [0101](0101-folders-first-libraries-optional.md)'s options.
- One file holding everything: two machines would conflict on it.

## Consequences

- The library's code is `apertrust-library`, in four layers ([0054](0054-four-layers-for-the-librarys-code.md)); its commands are the session's ([0058](0058-a-native-engine-every-client-through-the-session-protocol.md)).
- Where edits live is [0047](0047-where-edits-live-and-what-the-sidecar-holds.md)'s; two copies are compared by [0052](0052-comparing-two-copies-of-a-sidecar.md).
- The app's one photo state holds several photos; a restart passes them again. `packaging/linux/org.apertrust.Apertrust.desktop`'s `Exec` passes `%F` alone and `packaging/windows/installer.nsi`'s commands `"%1"` alone, as `packaging/nightly-identity.txt`'s line and `crates/app/tests/packaging.rs` then hold them.
- How each feature's data is kept, folder records, Move to… and Copy to…, the Library that travels, Keep Available Offline and Keep Edits on This Computer are [0101](0101-folders-first-libraries-optional.md)'s.
- Tethering and camera import (gphoto2) and Lua scripting would each need an exception to [0005](0005-rust-and-no-c-or-c-unless-no-rust-option-exists.md).
