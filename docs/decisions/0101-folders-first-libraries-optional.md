# 0101. Folders first, libraries optional

## Context

Sidecars are the truth ([0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md)), yet a catalogue's shape still costs people four things wherever one is used: photos imported before they are seen, a library file to find again when a drive moves, a database that breaks in a sync folder or on a share, and two truths once sidecars are written beside it. What people ask for most is to work in their folders as they are, across several computers and drives, with albums and search across folders for those who want them. How are photos organised, and where does each feature keep what it needs, so that nothing has to be made, opened or found before a folder can be worked in?

## Decision

**Folders are the unit; a Library is optional.** Any folder opens by every way in (Open Folder…, a drop, a recent, the start window, `apertrust FOLDER`, Open This Folder) and is worked in as it is: edited, rated, labelled, tagged, culled and exported. One photo stays as [0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md) has it. A first start shows folders, never a library. Everything about a photo is in its sidecar beside it, so photos move and rename freely, in Apertrust or in a file manager. Each feature's data is made where it belongs, when it is first used.

### Where each feature's data lives

| What | Where | Made |
| --- | --- | --- |
| Edits, ratings, labels, keywords, groups, virtual copies | The photo's `.apxmp` beside it ([0047](0047-where-edits-live-and-what-the-sidecar-holds.md), [0048](0048-the-apxmp-format.md)) | When something about the photo changes: the edit and its history built; ratings, labels, keywords, groups and further virtual copies planned (Phase 3) |
| A folder's own albums and smart albums, of photos anywhere under it | `.apertrust/` at that folder's top, hidden; it travels with the folder | When the first album or smart album is made there (Phase 3), or a Library is saved there (Phase 5) |
| The keyword tree | Built from the sidecars' hierarchical keywords; the keywords' properties (synonyms, private, category) and Recent in the user data folder's `keywords.toml` ([0073](0073-the-photos-tabs-tools-browse-first-filters-in-folder-mode.md)) | Never a file beside the photos |
| Search across a folder and its subfolders | The index: in memory over the open tree (Phase 3), then saved in this computer's cache ([0055](0055-the-librarys-cache.md), Phase 5) | As the folder opens, in the background |
| Browsing a folder whose drive is away | One thumbnail size per photo, kept in the cache, and the folder's index | When the folder is marked Keep Available Offline or is in a Library (Phase 5) |
| A Library: its folders and its albums across them | `<user data>/libraries/<id>/`, on this computer only | New Library… (Phase 5) |
| A Library that travels, by choice | `.apertrust/libraries/<id>/` in its home folder, one of its own folders | Save Library in a Folder…, or the choice in New Library… (Phase 5) |

```text
NAS/Photos/                          a folder, opened as it is
  .apertrust/                        hidden; only once an album is made or a Library saved here
    albums/7c0e9a14.toml             one file per album, named by id, naming its parent
    smart-albums/3f2a91d0.toml
    libraries/9e4d2b61/              only if a Library is saved here: library.toml,
                                     albums/, smart-albums/
  2026/2026-11-20 Lisbon/
    DSCF0001.RAF
    DSCF0001.RAF.apxmp

<user data>/                         this computer, what clearing the cache must not lose
  folders/5b0c7d0e.toml              a folder record: last path, volume id, path below the
                                     mount, Keep Available Offline, edits kept here
  folders/5b0c7d0e/pending/          ratings and tags waiting while its drive is away
  folders/5b0c7d0e/edits/            only with its edits kept on this computer, by path
  libraries/a1b2c3d4/library.toml    a Library: its name and its folders' ids
  libraries/a1b2c3d4/albums/  smart-albums/
  libraries/9e4d2b61/library.toml    a Library saved in a folder: its home folder's record
                                     and the last copy read, for while the home is away
  keywords.toml                      keyword properties and Recent

<cache>/                             rebuilt from the files
  indexes/                           a folder tree's index: kept while a folder it
                                     covers has a record, limited otherwise
  thumbnails/                        limited
  offline/                           kept: one thumbnail per photo of a folder kept offline
```

### Folders

- **Include subfolders** shows a folder's photos with its subfolders' (Phase 3, step 19b). It is off by default and remembered per folder, as darktable's film roll shows one folder and Import's `include_subfolders` is off. Search and filters follow what the Browser shows; with it off they say how many photos the subfolders hold, from the tree's walk, "37 more photos in subfolders", with the switch beside it, matches counted only where a saved index already holds them.
- **The first open of a large tree** lists and reads in the background, the open folder first. At the measured SMB rates on the M4 against a NAS (342 ms for 2,825 files listed in bulk, 2.2 ms a sidecar read 16 at a time), 100,000 photos list in about 12 s and their sidecars read in about 4 minutes; with the saved index (Phase 5) later opens start from it.
- **A folder's `.apertrust/`** holds its albums and smart albums, one TOML file each named by its id and naming its parent, and `libraries/<id>/` only where a Library is saved there. Album entries name photos by their path relative to that folder, with their fingerprint and virtual copy. `.apertrust/` holds no id of its own. It is made only by the first album or smart album made there, or by a Library saved there, whichever comes first; it holds those two and nothing else, and is removed once neither is left.
- **Nested `.apertrust/` folders** are found by the tree's walk: opening the outer folder shows its own albums and, grouped under the inner folder's name, the inner's. Nothing is merged.
- **Hidden:** the dot hides it on macOS and Linux, and Samba hides dot files from Windows clients by default; Explorer shows it otherwise, as it shows `.git`. Setting Windows' hidden attribute on a folder takes `SetFileAttributesW`, an `unsafe` call in the `windows` crate, which the standard library does not wrap (it reads the attribute, as the listing does, and sets attributes only on files it creates). So it is set only if `apertrust-fs` comes for another reason ([0065](0065-names-and-crates.md)); until then `.apertrust/` stays visible in Explorer, as `.git` does. The listing skips it already, by its dot.

### Folder records

**A folder record** is the one new internal piece, with no word in the interface: `<user data>/folders/<id>.toml`, made only when this computer must find a folder again, because it is in a Library, marked Keep Available Offline, keeps its edits on this computer, or is a saved Library's home folder.

- **It holds** the folder's last path, its volume's id and its path below the mount, its Keep Available Offline mark, whether its edits are kept here, and the offers declined in it; beside it, `pending/`, the changes waiting while its drive is away, and `edits/`, its edits when they are kept here.
- **The volume id is a hint, never the identity:** macOS's volume UUID, Linux's filesystem UUID, Windows' volume GUID, a share's server and share name, read from the platform's volume list, which step 20c's `LibraryEvent::Volumes` reads too. The photos' fingerprints confirm the folder. A clone with the same id at two places asks which is the folder. With no match, Locate… asks once per folder on each computer, accepted once enough of its photos match by fingerprint, and the answer is kept in the record.
- **Libraries name folders by the record's id,** so a folder in two Libraries has one record, one index, one set of kept thumbnails and one set of pending changes. A record is never shared between computers: a Library that travels names its folders by ids of its own, and each computer's record notes which entry it answers.
- **A plain recent keeps no record:** a moved mount shows it as Not found, with Locate….

### Move to… and Copy to…

Between any two folders (Phase 3, step 19, with Rename and Delete), each a `UserForeground` job, cancellable ([0057](0057-jobs-as-darktables-control-jobs.md)). This record decides what a move guarantees; how each guarantee is met is step 19's spec.

- **A photo moves whole or stays:** its main file (its raw, else its only image file) with its `.apxmp`, `.apdata` and attached files, and its edits kept on this computer, which go where the destination's edits are kept. A cancel, a crash or a write from elsewhere during the move never leaves a photo in both places, or half in each, and never loses a file: what cannot be settled is named in the activity for the person to settle.
- **On one volume, a rename; across volumes, a checked copy:** each file is copied, read back past the system's cache and compared with its source's hash before any source file is removed. The cache is passed with `F_NOCACHE` on the writing and the reading handle on macOS, `POSIX_FADV_DONTNEED` after `fsync` on Linux, best effort, and reads with `FILE_FLAG_NO_BUFFERING` on Windows; the read-back adds about 1.4 s a GB on the M4's SSD and about 1 s a raw over the NAS ([library.md](../library.md#the-checked-copy), where the card, Linux and Windows are still to measure).
- **Never replaces a file** at the destination, renaming each into place by a rename that refuses to, but on a share that has no such rename, as macOS's SMB client: there the copy looks for a file just before a plain rename, so another program's file made between the two is replaced, the window narrowed to that, never closed. A cancel takes effect between photos.
- **Albums:** Move to… out of a folder's tree counts the album entries in its `.apertrust/` that the move takes out, and says so before it runs.
- **Import's copy is the same checked copy** ([0085](0085-import-in-folder-mode.md)), from its first slice; its Verify the copies (20b) is the read-back, and its Backup (20d) a second destination.
- **Delete** sends photos with their sidecars to the system's trash.

### Libraries, optional

Phase 5. **New Library…** makes a named collection of folders, such as "Archive" or "Work"; **Add Folder…** adds folders from any drive, and a folder's ⋯ in Collections has **Add to Library…**.

- **What it adds:** search and filters across its folders; albums and smart albums spanning them; browsing, rating and tagging its folders while their drives are away; its folders found again by their volume id after a mount or drive letter changes.
- **It is light:** its name, its folders and its spanning albums and smart albums, kept by default in `<user data>/libraries/<id>/` on this computer, nothing written into its folders; its folders' indexes are in the cache.
- **A folder can be in several Libraries or in none,** and works the same either way: its sidecars and its own albums are the folder's. A Library never holds one folder twice, or a folder and one of its subfolders.
- **Import knows no Libraries:** Import… copies into a folder, never into a Library. A destination inside a folder a Library holds is in that Library as any subfolder is; one anywhere else is in none until Add Folder… adds it.
- **Removing a Library** deletes only its list and its albums, never photos, sidecars or a folder's own albums. Removing the last Library of a folder whose edits are kept here asks first: write the edits beside the photos, or add the folder to another Library.
- **Several Libraries per user data folder** take the place of darktable's workspaces; "workspace" names the Tools panel's layouts ([0096](0096-workspaces-as-files-hidden-tools-and-finding-a-tool.md)), so the word is Library. Settings, presets, styles, workspaces and keywords' properties are shared by every Library of the user data folder. Separate user data folders stay for wholly separate setups ([0089](0089-one-instance-and-one-window-per-user-data-folder.md)).
- **Keywords:** each scope's tree, a folder's or a Library's, comes from its own photos' sidecars, so a Library never shows another's keywords; properties and Recent are the user data folder's one `keywords.toml`, so a synonym set once works everywhere. A rename rewrites the sidecars of the open scope only.
- **In the window,** Collections lists the open folder's tree, with its Albums and Smart albums from steps 19c and 19d, and, from Phase 5, each Library with its folders, Albums and Smart albums. The recents hold a Library by its id.

### A Library that travels

Phase 5, step 27b, the last Library step, which moves to Later alone if Phase 5 runs long. As a choice, a Library is saved in one of its own folders, its **home folder**, so any computer that opens that folder can open the Library with its albums. It is the same Library kept elsewhere: the folder does not become a library, and no Library nests in another.

- **Saving it there:** New Library… asks where to keep it, This computer (the default) or In one of its folders; a Library kept here has Save Library in a Folder… in its ⋯ in Collections, and Keep Library on This Computer moves it back. Either move is Move to…'s checked copy: written, verified, and only then the other copy removed. Other computers then say once that it has left its folder.
- **The home folder** is one of the Library's own folders and writable, told by trying the write as `Cache::create` does, never by permission bits; a folder whose edits are kept on this computer cannot be home. While it is home it cannot be removed from the Library: Remove from Library asks first to move the Library to another of its folders or back to this computer.
- **Where:** `<home>/.apertrust/libraries/<id>/`, named by id as albums are, so renaming the Library moves nothing and two Libraries named Archive never collide.
- **What it holds, and nothing else:** `library.toml` (its id, name and format; its folders, each by an id of the Library's own with its volume id and path below the mount as a hint, the home folder `.`), and its spanning albums and smart albums, one file each. Never one computer's paths, Keep Available Offline marks, pending changes, edits kept on a computer, indexes, thumbnails, settings or keywords: those stay in each computer's folder records and cache.
- **On another computer,** opening the home folder asks once: "This folder holds the Library 'Archive'. Open it?" **Open** lists it in Collections with that computer's own Libraries, marked with its home folder; **Not Now** is remembered in the home folder's record, its ⋯ still offering Open Library "Archive". Its folders show as found (the hint matched, the photos confirming), away (no such volume present) or with Locate… (the volume present without the path, or an id this system reads differently, as an exFAT drive's can be between macOS and Linux). Its albums show as on the first computer; entries in a folder not found yet are away, never missing.
- **Two computers changing it:** its files are small, one per album, and [0052](0052-comparing-two-copies-of-a-sidecar.md)'s comparison applies to each, an album entry by entry and `library.toml`'s folders entry by entry, the later clock winning each. A folder removed is kept as a removal, so another computer's copy does not bring it back.
- **Its home folder away:** the Library shows as away, browsed from the last copy this computer read, kept in its `<user data>/libraries/<id>/`; its other folders work as usual; album changes wait in the home folder's record's `pending/` and are written through [0052](0052-comparing-two-copies-of-a-sidecar.md) when it is back. Renamed or moved, the Library goes with it, found as any folder with a record. Copied, as a backup, two copies of one Library id are found: a computer that finds both asks once which is the home and ignores the other.
- **Removed:** Remove Library… asks first, saying every computer loses it, then deletes `.apertrust/libraries/<id>/`, and `.apertrust/` if nothing else is in it; other computers say once that it was removed. Remove from This Computer forgets it on one computer only.

### Keep Available Offline

A folder's own mark, in Collections' ⋯, with no Library needed (Phase 5, step 27); a Library marks each of its folders the same way. The name stays Keep Available Offline, beside the cloud folders' Make Available Offline (step 29), which downloads a placeholder's file.

- **It keeps** the folder's record, its index and one thumbnail per photo in the cache's kept kind `offline/`, 256 px: 25.3 kB each on the corpus's embedded pictures on the M4, so 100,000 photos take about 2.5 GB ([0076](0076-the-cache-kinds-limits-and-control.md)).
- **Away,** its photos are browsed, rated and tagged; editing says the drive is not connected. The changes wait in the record's `pending/` and are written through [0052](0052-comparing-two-copies-of-a-sidecar.md)'s comparison when the drive is back, combining with any change another computer made meanwhile.

### Away, never deleted

A volume or share not present means away, never deleted. A file gone from a present volume means missing, re-paired by fingerprint when it comes back under the folder. No index row, album entry or pending change is dropped by a scan, only by the person.

### Nothing written beside the raws

For a person trying the app, an archive that must never be touched, or a share that cannot be written (Phase 5, step 28):

- **Albums go in a Library** kept on this computer, never in a folder's `.apertrust/`.
- **Keep Edits on This Computer,** a folder's setting in Collections' ⋯, and a setting for every folder not yet edited, `library.keep_edits_here`, off by default, with `--conf` for a test install, as darktable's manual suggests `write_sidecar_files=never` for one. The edits of every photo in its tree go to the folder record's `edits/`, mirrored by the path relative to that folder, the same `.apxmp` byte for byte; a photo's are looked for under the nearest folder above it, the folder itself included, whose record keeps edits here. A record a subfolder gains for another reason, Keep Available Offline or a Library, changes nothing of where its photos' edits live.
- **Turned on or off for a folder that has edits,** it asks first, naming how many, and moves them to their new place by Move to…'s checked copy, so a photo's edits are only ever in one place and none is lost. How a sidecar already at the new place, a folder that cannot be written and a subfolder's own setting are met is step 28's spec.
- **Nested:** a subfolder of a folder that keeps its edits here shows the setting on, from that folder, and changes it only there.
- **Routed to a Library,** so nothing is left unsaved: the first change in such a folder adds it to a Library, asking once which, or making one. Every Library holding the folder uses the same edits, and the folder is then only read.
- **A known limit, said in the docs and in the setting's help:** another computer, or another user data folder on this one (Apertrust Nightly, a development build), sees none of these edits and would write sidecars beside the photos, a second truth.

### Several computers and outside changes

- **Photos moved or renamed outside Apertrust** are matched again by fingerprint (size plus BLAKE3 of the first and last 64 KiB, [0053](0053-finding-folders-and-photos.md)) wherever the orphan sidecar and the photo are in one indexed scope, the open folder with its subfolders or a Library; across scopes, Find Photos… on the orphans. An orphan stays as Edits without a photo until the person removes it.
- **A folder with `.apertrust/` moved or renamed:** its albums move with it unchanged, since their entries are relative. **Copied,** as a backup, it is two folders, diverging from there; a Library holding both shows the photos twice.
- **Two computers editing one sidecar:** [0052](0052-comparing-two-copies-of-a-sidecar.md), a diverged edit kept as a second virtual copy named "From another computer" and the time. **Changing a folder's albums:** one file per album, combined entry by entry. An album file's conflict copy from a sync tool is combined into the album its id names, then moved as a sidecar's is.
- **A newer format** of an album, smart album or Library file opens view-only ([0052](0052-comparing-two-copies-of-a-sidecar.md)).

### What is left out, and what waits

- **Not part of the model:** a Project, with roles, a watched capture folder, job settings, templates and Export as Project…; library or project files; a library's own `Photos/` and import into a library; Import library…; nested or merged libraries; roots marked by an id file and a root map; machine names; the start window's Browse…. A job's names and destinations are Import's presets, its output an export preset, its selects ratings and albums, and its trash the system's.
- **Later, if asked:** editable previews and local copies of raws kept elsewhere; cloud storage folders, behind a spike; the server ([0071](0071-a-self-hosted-server-with-sync.md)); a presence note, written only into an existing `.apertrust/`; carrying album entries to Move to…'s destination; videos listed; the watched folder, with tethering.

### The interface

- **The start window's ways in:** New Library…, a disabled placeholder until Phase 5, then making a Library; Open Folder…; Import… (planned, step 20a2); with the recents.
- **Collections' ＋:** New Library…, then Open Folder… and Open…. **Its ⋯** gains Keep Available Offline, Keep Edits on This Computer and Add to Library…, each with its step.
- **No file is a library:** a `.aplibrary` dropped, opened or passed on the command line is refused as any file that is no raw.
- **The words:** Folder, Library, Album, Smart album, Include subfolders, Move to…, Copy to…, Keep Available Offline, Keep Edits on This Computer, Add Folder…, Add to Library…, Save Library in a Folder… and Keep Library on This Computer. The code and docs add **folder record**, **home folder** and **folder tree**; the tree's root stays the folder opened in Collections. Never project, catalogue or film roll, nor workspace for a Library.

## Options considered

- **Four modes with library files** (`Name.aplibrary` and a hidden `.apertrust/`, roots marked by `.apertrust/root.toml` and found through a per-machine root map, and the Project with roles, a watched folder, templates, Import library… and Export as Project…): a container a person must make, open, find again and merge, a second concept beside it, and an id file written into every folder added.
- **A library as a folder made one,** with `.aplibrary/` inside it or kept apart: two kinds of library, nested libraries needing a merge.
- **A Library always saved in its folders:** writes into the folders of people who want nothing there. **Always on this computer:** albums across folders never reach a second computer. **Export Library… and its import:** two copies that drift apart, a second truth. **Import into a Library, or a Library chosen in Import's After import:** brings back a place photos go rather than a list of folders, for what a destination inside a Library's folder or Add Folder… already gives.
- **Keywords per Library:** a synonym set in one is missing in the next, and the tree, built from the sidecars, already keeps each Library to its own keywords.
- **`.apalbums/` or `.aplibrary/` for a folder's own data:** the first too narrow for what a folder's data grows to; the second names what is no longer there.
- **Keep Available Offline only through a Library:** the same record and kept thumbnails either way; a folder needs no Library for it.
- **Include subfolders on by default:** every photo and sidecar of a whole NAS archive read on its first open. **Off, with search always spanning subfolders:** two rules where one does.
- **Edits kept on this computer, told to other user data folders** by a registry outside every one of them: breaks [0089](0089-one-instance-and-one-window-per-user-data-folder.md)'s separate setups for one case; **by a marker file in the folder:** writes beside the raws, which the setting exists to avoid.
- **Volume ids as a folder's identity:** a clone or a reformatted drive would be taken for the folder; they are a hint, the fingerprints confirming.
- **The saved index in Phase 3:** a large tree reads in the background meanwhile, so it waits for Phase 5.

## Consequences

- [0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md) is one photo and folders with optional Libraries; [0047](0047-where-edits-live-and-what-the-sidecar-holds.md) edits beside the photos or kept on this computer per folder; [0052](0052-comparing-two-copies-of-a-sidecar.md) covers `.apertrust/`'s and a Library's files; [0053](0053-finding-folders-and-photos.md) folder records in place of roots, volume ids as hints; [0054](0054-four-layers-for-the-librarys-code.md) a folder tree or a Library as the model, `MoveTo` and `CopyTo` for `ImportLibrary` and `ExportAsProject`, the storage interface waiting for a second backend; [0055](0055-the-librarys-cache.md) an index per folder tree; [0071](0071-a-self-hosted-server-with-sync.md) later, if asked, and [0058](0058-a-native-engine-every-client-through-the-session-protocol.md)'s one server serving the engine to remote clients without waiting for it; [0076](0076-the-cache-kinds-limits-and-control.md) the kinds `indexes/` and `offline/`, the folder records and Libraries in the user data folder; [0005](0005-rust-and-no-c-or-c-unless-no-rust-option-exists.md), [0037](0037-one-view-the-photos-tab.md), [0043](0043-settings-and-apertrusts-folders.md), [0049](0049-the-apdata-file-as-darktables-dtdata.md), [0073](0073-the-photos-tabs-tools-browse-first-filters-in-folder-mode.md), [0085](0085-import-in-folder-mode.md), [0089](0089-one-instance-and-one-window-per-user-data-folder.md) and [0095](0095-the-windows-parts-named-by-their-role.md) lose the Project and the library file.
- The roadmap: Phase 3 gains steps 19 (widened), 19b, 19c and 19d; Phase 5 becomes "Libraries and the index across folders", steps 21 to 33 with 27b.
- The code. Built: the start window's ways in are New Library…, a disabled placeholder, and Open Folder… (`start.rs`); Collections' ＋ has no New Project… (`library.rs`); `dropped.rs` refuses a `.aplibrary` as `Refusal::Unknown`, as any file that is no raw; `Icon::Project` and the strings only it used are gone from every language. Planned: `recents.rs`'s `library` kind a Library by id in Phase 5; base's `cache::KINDS` takes `indexes` and `offline` in place of `libraries` and `copies` with their steps, and Settings › Storage's Library caches line and `cache.unused_after`'s row name the indexes then.
- The names that stay: `apertrust-library`, `LibraryCommand`, `LibraryEvent` and `Source::Folder`, joined by `Source::Library` in Phase 5.
- Docs: [library.md](../library.md), [roadmap.md](../roadmap.md), [gui.md](../gui.md), [settings.md](../settings.md), [cache.md](../cache.md), [crates.md](../crates.md) and [architecture.md](../architecture.md).
