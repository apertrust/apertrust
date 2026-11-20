# 0073. The Photos tab's tools: Collections first, filters in folder mode

## Context

The Photos tab (decisions 37 and 38) chooses what the Browser shows and describes the photos in it. Decision 46 put Folders, Albums and Smart albums into one pinned tool. What that tool is called, what else the tab holds, in what order, and whether folder mode can filter its photos before the library's cache exists (decision 55) were open. darktable's lighttable has a module for filtering (collections and filters), one for tags (tagging) and two for metadata (image information and the metadata editor).

## Decision

- **The tab's default tools,** in order: **Collections**, pinned; **Filters**; **Keywords**; **Metadata**; **Styles**; then **Export**, a utility, last (built for the selection, else the active photo, its groups and Summary included; its presets planned; [0090](0090-export-as-a-photos-tab-tool.md)). A person reorders, pins, moves and floats them as any tab's sections ([0083](0083-the-side-panels-layout-dragged-and-floating.md)).
- **Collections** has one name in every mode and chooses what the Browser shows, as darktable's collections module chooses what its filmstrip and lighttable show (`src/libs/collect.c`), each set it chooses a collection in the code and the docs ([0095](0095-the-windows-parts-named-by-their-role.md)): Folders alone in one photo; Folders with the folder's own Albums and Smart albums in a folder (Phase 3, [0101](0101-folders-first-libraries-optional.md)); each Library with its folders, Albums and Smart albums (Phase 5). Its top line shows what is open, in one photo the photo (the active one where several were opened together), in folder mode the open folder ("Folder: Wedding 2026-10"), its photo count on its row in Folders' tree, as the wireframe's frame draws it, in a Library its name, with the recents, ＋, Import… (planned; [0090](0090-export-as-a-photos-tab-tool.md)) and ⋯ where [gui.md](../gui.md#the-tools-panel) puts them. Its options (⋯) gain Keep Available Offline, Keep Edits on This Computer and Add to Library… in Phase 5. In one photo, Folders holds Open This Folder alone, the active photo's folder: the one way from a photo opened alone to folder mode ([0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md)).
- **Filters** narrows what the Browser shows by collapsible groups with counts: a file name search, Rating, Colour label, History, Date taken, Camera, Lens, File type and the rest, the groups shown chosen by ＋ and kept app-wide. Its rules are the index's `Rules`, darktable's collect rules and filters (decision 54), the same a smart album saves.
- **In folder mode too, from Phase 3.** Filters runs over the open folder's `Index` in memory (decision 53), first the groups from what the index already reads (the sidecars and the capture times), then camera, lens and exposure, read with the capture times, then keywords and the metadata fields once those tools exist. The filter lives in the session, writing nothing; Save as Smart Album saves it into the folder's `.apertrust/` (the library's step 19d), the folder's first album or smart album making it. A filter that hides selected photos leaves only the shown ones selected.
- **Keywords is one tool,** darktable's tagging: the Add field, the chips of the keywords on the selected photos (dimmed with a count on some of them), Recent, the last nine keywords used, as buttons, and the keyword list, a tree with counts, closed by default and resizable. A keyword is a path written with `|` as darktable's, to `dc:subject` and `lr:hierarchicalSubject` as darktable writes them; its properties are darktable's: synonyms, private (left out on export) and category (groups keywords, never set on a photo). Saved keyword sets are later.
- **The keyword list's file.** The tree each scope shows, a folder's or a Library's, comes from its own photos' sidecars; the keywords' properties (synonyms, private, category) and Recent are one `keywords.toml` in the user data folder, shared by every folder and Library ([0101](0101-folders-first-libraries-optional.md)). A rename rewrites the sidecars of the open scope only.
- **Changes to the keyword list** (rename, move, delete, edit) are `LibraryCommand`s that write every sidecar holding the keyword, each a cancellable job reporting its row (decision 72) and one undo step.
- **Metadata is one tool,** darktable's image information and metadata editor together: the facts the file holds, read-only, and darktable's metadata fields (Title, Description, Creator, Publisher, Rights, Notes, Version name), edited in place, in collapsible groups (Photo, Shot, Description, Location, More). With several selected, a value they differ in says Different, and a field typed in is written to all of them.

## Options considered

- **Library** as the first tool's name: a Library tool in a folder reads as if a library existed, and clashes with the optional Library (decisions 37 and 101).
- **A name that changes with what is open** (Folder, Library): one constant name is easier to find and to talk about; the top line says what is open.
- **Sources:** too technical.
- **Folders:** wrong once albums sit in it.
- **Folders and albums:** half empty in folder mode.
- **Places:** an album is not a place.
- **Photos:** it repeats the tab.
- **Browse:** a verb among the tools' nouns, and beside the Browser it shows for, two names a letter apart ([0095](0095-the-windows-parts-named-by-their-role.md)).
- **Filters only with the saved index, in Phase 5:** folder mode would have no filter for culling, though its index already holds what the first groups need.
- **Quick filters on the browser bar:** the browser bar keeps two buttons (Sort, Zoom); filters with counts need a tool's room.
- **Keywords and a keyword list as two tools,** as commercial raw editors split them: two places would assign keywords, or the list could not assign at all; darktable's tagging is one module.
- **Image information and the metadata editor as two tools,** as darktable has them: the facts and the fields about one photo read best as one list.
- **Folder mode's keyword properties in each sidecar:** a property belongs to the keyword, not to a photo, and would differ between photos.

## Consequences

- `apertrust-library`'s index gains its rules and facet counts in Phase 3, which smart albums reuse from step 19d and the saved index in Phase 5.
- The read that gives the capture times (`LibraryEvent::Taken`) also gives camera, lens and exposure.
- The roadmap's steps 15 and 15b carry the Keywords, Metadata and Filters tools in folder mode, step 19d Save as Smart Album.
- Collections' section id is `collections`, its title message `section-collections`. View › Collections (`show-collections`, L, darktable's lighttable key) chooses the first tab holding it ([gui.md](../gui.md#view)).
- Collections' ＋ ends with Open…, as File › Open… (decision 46), and Styles' first content tab is All.
- `docs/gui.md` lists the tab's tools; `docs/library.md` the filters in folder mode, the keyword file and the commands; `docs/settings.md` the user data folder's `keywords.toml`.
