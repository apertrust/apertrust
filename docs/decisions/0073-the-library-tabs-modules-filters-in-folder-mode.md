# 0073. The Library tab's modules, filters in folder mode

## Context

The Library tab (decisions 37 and 38) chooses what the grid panel shows and describes the photos in it. Decision 46 put Folders, Albums and Smart albums into one pinned Library module. What else the tab holds, in what order, and whether folder mode can filter its photos before the library's cache exists (decision 55) were open. darktable's lighttable has a module for filtering (collections and filters), one for tags (tagging) and two for metadata (image information and the metadata editor).

## Decision

- **The tab's default modules,** in order: **Library**, pinned; **Filters**; **Keywords**; **Metadata**; then **Styles**. Customise tabs changes them as any tab's (decision 38).
- **Filters** narrows what the grid panel shows by collapsible groups with counts: a file name search, Rating, Colour label, History, Date taken, Camera, Lens, File type and the rest, the groups shown chosen by ＋ and kept app-wide. Its rules are the index's `Rules`, darktable's collect rules and filters (decision 54), the same a smart album saves.
- **In folder mode too, from Phase 3.** Filters runs over the open folder's `Index` in memory (decision 53), first the groups from what the index already reads (the sidecars and the capture times), then camera, lens and exposure, read with the capture times, then keywords and the metadata fields once those modules exist. Nothing is written beside the photos: the filter lives in the session. Save as Smart Album needs a library, so in folder mode it leads to Make This Folder a Library…. A filter that hides selected photos leaves only the shown ones selected.
- **Keywords is one module,** darktable's tagging: the Add field, the chips of the keywords on the selected photos (dimmed with a count on some of them), Recent, the last nine keywords used, as buttons, and the keyword list, a tree with counts, closed by default and resizable. A keyword is a path written with `|` as darktable's, to `dc:subject` and `lr:hierarchicalSubject` as darktable writes them; its properties are darktable's: synonyms, private (left out on export) and category (groups keywords, never set on a photo). Saved keyword sets are later.
- **The keyword list's file.** A library's is its `keywords.toml` (decision 46). Folder mode has none, so its keywords' properties and Recent are in a `keywords.toml` of the same form in the user data folder; inside a library the library's own file is used.
- **Changes to the keyword list** (rename, move, delete, edit) are `LibraryCommand`s that write every sidecar holding the keyword, each a cancellable job reporting its row (decision 72) and one undo step.
- **Metadata is one module,** darktable's image information and metadata editor together: the facts the file holds, read-only, and darktable's metadata fields (Title, Description, Creator, Publisher, Rights, Notes, Version name), edited in place, in collapsible groups (Photo, Shot, Description, Location, More). With several selected, a value they differ in says Different, and a field typed in is written to all of them.

## Options considered

- **Filters only with the library's cache, in Phase 5:** folder mode would have no filter for culling, though its index already holds what the first groups need.
- **Quick filters on the grid bar:** the grid bar keeps three buttons (Sort, Labels, Zoom); filters with counts need a module's room.
- **Keywords and a keyword list as two modules,** as commercial raw editors split them: two places would assign keywords, or the list could not assign at all; darktable's tagging is one module.
- **Image information and the metadata editor as two modules,** as darktable has them: the facts and the fields about one photo read best as one list.
- **Folder mode's keyword properties in each sidecar:** a property belongs to the keyword, not to a photo, and would differ between photos.

## Consequences

- `apertrust-library`'s index gains its rules and facet counts in Phase 3, which the library's cache and smart albums reuse in Phase 5.
- The read that gives the capture times (`LibraryEvent::Taken`) also gives camera, lens and exposure.
- The roadmap's steps 15 and 15b carry the Keywords, Metadata and Filters modules in folder mode; step 26 the library's keyword list.
- `docs/gui.md` lists the tab's modules; `docs/library.md` the filters in folder mode, the keyword file and the commands; `docs/settings.md` the user data folder's `keywords.toml`.
