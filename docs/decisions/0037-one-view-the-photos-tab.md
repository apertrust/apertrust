# 0037. One view, the Photos tab

## Context

Raw editors usually split their window into views (a library, a darkroom, a map, print, a slideshow), each with its own panels and keys. That leaves two things that change with the view and are out of sight once it is chosen: the Tools panel's tabs, and whether a key acts on the active photo or the selection. Both are mode errors. How is the window organised instead, what is the Tools panel's tab for choosing photos called, and how does the engine show many photos while one is edited? In folder mode, where most people start, a tab named Library holding a tool named Library and offering Make This Folder a Library… reads as if a library already existed.

## Decision

**The app is one view.** There is no Library, Edit, Map, Print or Slideshow view.

**The window's parts are named by their role** ([0095](0095-the-windows-parts-named-by-their-role.md)): the **Toolbar** along the top edge, the window's title bar; **Tools**, the Tools panel, with the workspace's tabs and their tools; the **Viewer**, showing the layout, with the **viewer bar** along its top; and the **Browser**, the thumbnails of the open folder, album or project, with the **browser bar**. The names stay true wherever a part sits: the Tools panel at the right or the left, the Browser at the bottom, the left or the right, and Touch mode's portrait layout ([0094](0094-one-interface-standard-and-touch-modes.md)).

- **Photos is the Tools panel's first tab by default,** as in the built-in workspaces. It holds everything about photos as items: which photos the Browser shows, through its first tool, Collections, with Folders and later Albums and Smart albums ([0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md), [0073](0073-the-photos-tabs-tools-browse-first-filters-in-folder-mode.md)), and what they are. The other tabs (in Edit Adjust, Detail, Geometry, Effects, Raw and History) are about one photo's pixels ([0038](0038-the-side-panels-tabs-by-editing-stage.md)). The Browser under the photo shows the current folder or album and is always shown.
- **The Viewer shows a layout,** switched only when the person asks:
  - **Photo:** the active photo, or the selected photos side by side in a grid of 3 × 4 or 4 × 3 cells, following the Viewer's shape, up to 11 photos, the 12th cell saying "N more photos";
  - **Map** (later): switched by the viewer bar's Map toggle after Guides, or Shift+M; pressed again, back to Photo. Its tiles are offline first, a Natural Earth basemap shipped with the app and regional PMTiles files the person adds; online tiles, as darktable has them (OpenStreetMap and other sources, cached on disk), stay off until the person turns them on, over TLS through the platform's own (`native-tls`), which [0005](0005-rust-and-no-c-or-c-unless-no-rust-option-exists.md)'s rule for platform libraries covers. Planned with the Map layout.
- **The map shows only when asked:** opening a tab or using a section never switches the Viewer.
- **The Tools panel's tabs and what a key acts on stay the same** whatever the Viewer shows; the whole view is one keyboard scope.
- **With several photos selected,** the rating keys (rating, reject, colour labels), Apply edit and Export act on every selected photo, with the count shown and one Undo for all; sliders and Reset act on the active photo only. There is no "edit all selected" toggle.
- **Other tasks:** Import and Print are full-window sheets inside the main window ([0086](0086-import-as-a-full-window-sheet.md)); Export is a tool of the Photos tab, built for the active photo and acting on the selected photos once the selection is built ([0090](0090-export-as-a-photos-tab-tool.md)); Slideshow is an action.
- **Words:** Photos for the Tools panel's first tab; Library only for the Phase 5 mode, the library file a person creates and opens ([0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md)), never for a generic interface label, a tab, a tool, a group or a Settings page (the page is Photos); Collections for the tool that chooses what the Browser shows, Folder, Album and Smart album for what it holds, and "collection" for any set of photos it chooses, in code and docs only; Toolbar, Tools, Viewer and Browser for the parts ([0095](0095-the-windows-parts-named-by-their-role.md)); "layout" for what the Viewer shows; "workspace" for a named, saved set of the Tools panel's tabs with their tools, the tools it hides and the toolbar's items, built in or the person's own, each kept in a file of its own ([0096](0096-workspaces-as-files-hidden-tools-and-finding-a-tool.md)). The composition overlay is Guides.

**The engine's side, planned until spike W1 measures it:**

- one session per window, told the visible cells, each a photo id and a size;
- only the active photo has a live pipeline; every other photo on screen is a still, rendered once from its saved edit by the same viewport pipe at its cell's size, its frame kept and its pipeline freed, so a still equals the live frame at that size byte for byte and activating a photo moves nothing;
- commands name photos by id; an edit to a photo that is not live changes its edit and sidecar without the GPU, then renders its still again;
- the engine thread takes the commands, renders the live frame if it changed, then makes one bounded submission of background work, a tile of a still, a `SystemForeground` job ([0057](0057-jobs-as-darktables-control-jobs.md)), so a change waits for one such submission at most;
- decoding runs on a job worker, never the engine thread, the active photo opening in stages ([0077](0077-a-photo-opens-in-stages.md));
- compiled kernels live as long as the session, kept across photos from the staged open on ([0077](0077-a-photo-opens-in-stages.md)); one memory budget covers the session, stills dropped first; how many photos stay live is a setting, one for now.

## Options considered

- Views, as darktable has them: the mode errors above.
- Library as the tab's name: in folder mode it reads as if a library existed, and clashes with the Phase 5 mode.
- Browse as the tab's name: a verb among noun tabs, and Keywords, Metadata and Styles are not browsing.
- A layout switcher in the Toolbar: close to a view switch again.
- A Grid layout of thumbnails: the Browser is always shown, so a second grid adds nothing.
- The map showing itself when a map tab or section opens: it hides the photo a focused slider still edits and has no clear moment to switch back.
- An "edit all selected" toggle: a rule people find unclear.
- A session per visible photo: about 1.5 to 4.9 GB each untiled, and an edit waiting behind other photos' first renders of 150 to 580 ms.
- Several live pipelines now: 0.8 to 4.9 GB more each; kept as the setting.

## Consequences

- The Tools panel's tabs are [0038](0038-the-side-panels-tabs-by-editing-stage.md)'s; the Photos tab's tools are [0073](0073-the-photos-tabs-tools-browse-first-filters-in-folder-mode.md)'s.
- Known costs of the name: the Mac's Photos app shares the word, and Photos is close to the Viewer's Photo layout. A library's `Photos/` folder, which holds the photos copied in ([0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md)), keeps its name: it is a folder on disk, shown in Collections' Folders as any folder is, not an interface label.
- `apertrust-library` is built from Phase 2 for folder mode and reached through the session protocol, since a browser client's library lives on the server.
- Stills, several photos in one session and commands by photo id are planned; spike W1 measures the mechanism before it is built.
