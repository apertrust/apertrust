# 0073. The Photos tab's tools: Browse first, filters in folder mode

## Context

The Photos tab (decisions 37 and 38) chooses what the grid panel shows and describes the photos in it. Decision 46 put Folders, Albums and Smart albums into one pinned module. What that module is called, what else the tab holds, in what order, and whether folder mode can filter its photos before the library's cache exists (decision 55) were open. darktable's lighttable has a module for filtering (collections and filters), one for tags (tagging) and two for metadata (image information and the metadata editor).

## Decision

- **The tab's default modules,** in order: **Browse**, pinned; **Filters**; **Keywords**; **Metadata**; then **Styles**. A person reorders, pins, moves and floats them as any tab's sections ([0083](0083-the-side-panels-layout-dragged-and-floating.md)).
- **Browse** has one name in every mode and chooses what the grid panel shows: Folders alone in one photo and folder mode; Folders, Albums and Smart albums in a library or a project (Phase 5). Its top line shows what is open, in folder mode the open folder ("Folder: Wedding 2026-10"), its photo count on its row in Folders' tree, as the wireframe's frame draws it, in a library or a project its name, with the recents, ＋ and ⋯ where [gui.md](../gui.md#the-side-panel) puts them. Make This Folder a Library… is in its options (⋯).
- **Filters** narrows what the grid panel shows by collapsible groups with counts: a file name search, Rating, Colour label, History, Date taken, Camera, Lens, File type and the rest, the groups shown chosen by ＋ and kept app-wide. Its rules are the index's `Rules`, darktable's collect rules and filters (decision 54), the same a smart album saves.
- **In folder mode too, from Phase 3.** Filters runs over the open folder's `Index` in memory (decision 53), first the groups from what the index already reads (the sidecars and the capture times), then camera, lens and exposure, read with the capture times, then keywords and the metadata fields once those modules exist. Nothing is written beside the photos: the filter lives in the session. Save as Smart Album needs a library, so in folder mode it leads to Make This Folder a Library…. A filter that hides selected photos leaves only the shown ones selected.
- **Keywords is one module,** darktable's tagging: the Add field, the chips of the keywords on the selected photos (dimmed with a count on some of them), Recent, the last nine keywords used, as buttons, and the keyword list, a tree with counts, closed by default and resizable. A keyword is a path written with `|` as darktable's, to `dc:subject` and `lr:hierarchicalSubject` as darktable writes them; its properties are darktable's: synonyms, private (left out on export) and category (groups keywords, never set on a photo). Saved keyword sets are later.
- **The keyword list's file.** A library's is its `keywords.toml` (decision 46). Folder mode has none, so its keywords' properties and Recent are in a `keywords.toml` of the same form in the user data folder; inside a library the library's own file is used.
- **Changes to the keyword list** (rename, move, delete, edit) are `LibraryCommand`s that write every sidecar holding the keyword, each a cancellable job reporting its row (decision 72) and one undo step.
- **Metadata is one module,** darktable's image information and metadata editor together: the facts the file holds, read-only, and darktable's metadata fields (Title, Description, Creator, Publisher, Rights, Notes, Version name), edited in place, in collapsible groups (Photo, Shot, Description, Location, More). With several selected, a value they differ in says Different, and a field typed in is written to all of them.

## Options considered

- **Library** as the first module's name: a Library module in folder mode, offering Make This Folder a Library…, reads as if a library existed, and clashes with the Phase 5 mode (decision 37).
- **A name that changes with what is open** (Folder, Library, Project): one constant name is easier to find and to talk about; the top line says what is open.
- **Sources:** too technical.
- **Folders:** wrong once albums sit in it.
- **Folders and albums:** half empty in folder mode.
- **Places:** an album is not a place.
- **Photos:** it repeats the tab.
- **Collections:** the code's and the docs' word, kept out of the interface ([0037](0037-one-view-the-photos-tab.md)).
- **Filters only with the library's cache, in Phase 5:** folder mode would have no filter for culling, though its index already holds what the first groups need.
- **Quick filters on the grid bar:** the grid bar keeps three buttons (Sort, Labels, Zoom); filters with counts need a module's room.
- **Keywords and a keyword list as two modules,** as commercial raw editors split them: two places would assign keywords, or the list could not assign at all; darktable's tagging is one module.
- **Image information and the metadata editor as two modules,** as darktable has them: the facts and the fields about one photo read best as one list.
- **Folder mode's keyword properties in each sidecar:** a property belongs to the keyword, not to a photo, and would differ between photos.

## Consequences

- `apertrust-library`'s index gains its rules and facet counts in Phase 3, which the library's cache and smart albums reuse in Phase 5.
- The read that gives the capture times (`LibraryEvent::Taken`) also gives camera, lens and exposure.
- The roadmap's steps 15 and 15b carry the Keywords, Metadata and Filters modules in folder mode; step 26 the library's keyword list.
- Browse's section id is `browse`, its title message `section-browse`. View › Browse (`show-browse`, L, darktable's lighttable key) chooses the first tab holding it ([gui.md](../gui.md#view)).
- Browse is the one verb among the modules' titles, a known cost. So that the Photos tab holds one Browse, Browse's ＋ ends with Open…, as File › Open… (decision 46), and Styles' first content tab is All. The start window's Browse… button still shares the word.
- `docs/gui.md` lists the tab's modules; `docs/library.md` the filters in folder mode, the keyword file and the commands; `docs/settings.md` the user data folder's `keywords.toml`.
