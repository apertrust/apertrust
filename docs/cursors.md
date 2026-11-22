# Cursors

A cursor decides what a press on the photo does. One is chosen at a time, and it stays chosen until another is chosen: Select, the default, selects and moves marks on the photo and pans a zoomed photo from empty photo; Pan always pans; a picker cursor sets White balance or Exposure from the area clicked or drawn; later, mask cursors paint a layer's mask, retouch cursors heal spots, annotation cursors mark the photo, and Copy and apply copies one photo's edit onto others. This document describes the well on the Toolbar where cursors are chosen, the groups and their settings, the keys, the pointer on the photo, the API every owner of cursors uses, and how the work is built. Pickers themselves are in [pickers.md](pickers.md), the Toolbar in [gui.md](gui.md#the-toolbar), layers and masks in [layers.md](layers.md).

**Status.** [Decision 104](decisions/0104-cursors-one-chosen-at-a-time-in-a-well-on-the-toolbar.md) records the design, with the options considered. Built, 104.1: the one-shot pick a picker cursor will send, `PickerCommand::Once`, and `Region::pressed` (79.17 of [pickers.md](pickers.md#a-one-shot-pick)). Planned: Phase 2 builds Select, Pan and Pick with White balance and Exposure ([The plan](#the-plan)); Copy and apply comes in Phase 3, the Mask and Retouch cursors with layers in Phase 4, Annotate later with its own decision. Today a drag on a zoomed photo pans it, and a tool's picker starts from its button alone.

## What it serves

| Owner | Its cursors | What a gesture becomes |
| --- | --- | --- |
| The app | Select, Pan, Copy and apply | A mark selected or moved; the view's pan; Copy edit and Apply edit on the photo clicked |
| A processing tool | A picker cursor for a picker of its own: White balance's `spot`, Exposure's `exposure` | `PickerCommand::Once`, one step of the tool where it changes the edit ([pickers.md](pickers.md#a-one-shot-pick)) |
| The Colour picker | Colour readout | Its commands designed with its section (79.16) |
| Layers and masks (decision 34) | Brush, Linear gradient, Radial gradient, AI object, Path | The chosen layer's mask parts, one step a stroke or shape |
| Retouch layers | Heal, Clone | Spots on a retouch layer |
| Annotations | Pen, Arrow, Text | A note on the photo, its own decision |
| A third-party plugin (later) | As a tool's | Its own commands, through the plugin API |

**Cursors are an additional speed-up, never the single way to a feature.** Every cursor's work is also reachable where the feature lives: a tool's picker button, the Layers section's mask and retouch buttons, Edit's Copy edit and Apply edit. The engine never hears of cursors: a cursor turns a finished gesture into commands its owner already sends.

## A cursor

- **One chosen at a time,** across every group. It decides what a press does over the photo in the Viewer and, for a cursor that acts on photos, over the Browser's thumbnails; elsewhere the pointer works as ever.
- **It stays chosen** across photos, zooms, tabs and edits until another is chosen, by its face, its key or its group's list. So a series is K once, then a click and → for each photo.
- **Select at every start.** The chosen cursor is never saved; each group's active cursor, the one its face shows, is kept for the run, the group's first at start.
- **While a tool's picker is on,** from its button, it takes the photo, as darktable's does ([pickers.md](pickers.md#on-the-photo)), a press with A held too; the chosen cursor waits underneath and acts again once the picker turns off.
- **While it cannot act,** it stays chosen: the photo not yet live or open for viewing only, a tool hidden by the History tab's eye, Exposure's picker toward a reference with none set. A press then does nothing, the pointer over the photo is the not-allowed one, and the cursor's tooltip says why.

## The groups

In the well's order. Each group is one button of the well, even a group of one cursor.

| Group | Cursor | Its tooltip, after its name and key | Gestures | Where | When |
| --- | --- | --- | --- | --- | --- |
| Select | Select | "Click a mark on the photo to select it, drag to move it; drag elsewhere to pan" | Click, drag | Viewer | Phase 2; marks to select come with stored samples (79.14) and masks |
| Pan | Pan | "Drag to pan a zoomed photo, over marks too" | Drag | Viewer | Phase 2 |
| Pick | White balance picker | "Click a neutral area, or drag one" | Click, area | Viewer | Phase 2 |
| Pick | Exposure picker | Its picker's own words, which follow Picker target ([Actions and words](#actions-and-words)) | Click, area | Viewer | Phase 2 |
| Pick | Colour readout | Designed with the Colour picker | | Viewer | With 79.16 |
| Mask | Brush | "Paint the layer's mask; Alt subtracts" | Stroke | Viewer | Phase 4 |
| Mask | Linear gradient | "Drag from full strength to none" | Shape | Viewer | Phase 4 |
| Mask | Radial gradient | "Drag from the centre" | Shape | Viewer | Phase 4 |
| Mask | AI object | "Click an object to select it" | Click | Viewer | Phase 4 |
| Mask | Path | "Click to add points, drag for curves" | Points | Viewer | Phase 4 |
| Retouch | Heal | "Paint over a spot" | Stroke | Viewer | Phase 4 |
| Retouch | Clone | "Alt-click the source, then paint" | Stroke | Viewer | Phase 4 |
| Annotate | Pen, Arrow, Text | A note drawn, an arrow dragged, a note typed | Stroke, shape, click | Viewer | Later, its own decision |
| Copy and apply | Copy and apply | "Click a photo to apply the copied edit; Alt-click copies" | Click | Viewer and Browser | Phase 3, with Copy Parts |

- **A group no owner fills is not drawn,** nor a cursor not yet built: Phase 2's well holds Select, Pan and Pick.
- **Select:** a click on a mark selects it and a drag that starts on one moves it; a drag that starts on empty photo pans a zoomed photo, the built pan by a drag and darktable's; a click on empty photo does nothing. Its pointer is the move pointer over a mark that can be moved, elsewhere the built pan's.
- **Pan** pans from any drag, one that starts on a mark too, so a mark is never moved by mistake: the built pan by a drag ([gui.md](gui.md#panning)), the open hand over a photo that can move, the closed hand while dragging, the arrow at Fit, where there is nothing to move.
- **Copy and apply** acts on the photo clicked, in the Viewer or on a thumbnail: its first click with nothing copied, or an Alt-click, runs Copy edit on it, every other click Apply edit, each one undo record as Apply edit is. While it is chosen a click on a thumbnail applies rather than opening the photo; Select gives the Browser its click back. The Toolbar keeps Copy edit and Apply edit.
- **Mask and Retouch** act on the chosen layer as the Layers section's buttons do (decision 34). A mask button in the Layers section and its cursor are one state: pressing the button chooses the cursor, which the well shows, and choosing the cursor shows the button on. Clone's source has a visible control too, as decision 94's rule 3 requires, settled with retouch's port in Phase 4; Alt-click is its shortcut.

## The well

**Where:** one toolbar item, `cursors`, a part of the Toolbar's (decision 92), centred between two flexible spaces after Apply edit, so it sits in the middle of the room between the actions and the status slots, with the window's drag strip on both sides:

`import, export, cull, reset, undo, redo, space, copy-edit, apply-edit, flexible-space, cursors, flexible-space, jobs, assistant, save-state`

Until decision 92's list is built (Phase 3), the well is a fixed part of the built Toolbar in the same place. It is client area: a press on it never moves the window, and a double-click never maximises it (`HTCLIENT` on Windows).

**Measures:**

| Part | Measure |
| --- | --- |
| Track | `sunken`, 30 points high, `radius_large` (5), 2 points of padding above and below and 4 at each end (`spacing.xxs`, `spacing.xs`) |
| Between groups | `spacing.s`, 6 points, the track showing through |
| A face | 22 by 26 points, the 16-point icon (`sizes.icon_large`) at its end, next to its chevron |
| A chevron | 11 by 26 points, its 10-point chevron at its start, right against its face |
| A group of one with no settings | One face 24 points wide, its icon centred, no chevron |
| The well | Phase 2's Select, Pan and Pick, 101 points; Phase 4's six groups, 218 |

**States** of a face and a chevron, with no plate in any state (decision 81):

| State | Dark | Light |
| --- | --- | --- |
| Track | `sunken` #232323 on the Toolbar's `surface` | `sunken` #FFFFFF |
| Rest | The outline icon in `text_secondary` #A0A0A0 | `text_secondary` |
| Under the pointer | The icon in `text_strong` #E6E6E6 | `text_strong` |
| Held down | The icon in `accent_text`, feedback only, as an icon button's | `accent_text` |
| Chosen | The face's icon in `accent_text` #E58A48, its stroke a quarter heavier than the outline (`icons::STROKE_HEAVY` against `STROKE`), and nothing else | `accent_text` #A15717 |
| Chosen, cannot act now (its picker not ready, the photo not live) | The chosen look kept, its icon in `accent_text`, its tooltip saying why; over the photo the not-allowed pointer | As dark |
| The list open | The chevron in `text_strong` | `text_strong` |
| Disabled: not chosen and cannot act now (no photo shown, the photo not live, its picker not ready) | `text_disabled`, the tooltip saying why, no focus | `text_disabled` |
| Focused after a key | As at rest, with the focus ring round the one target | As dark |
| High contrast | The track edged 1 point in `button_edge`, a face under the pointer outlined as an icon button's | |

- **The accent marks what a press on the photo acts with,** as amber marks what is chosen elsewhere (the tab underline, the active photo's outline). It is the one icon the Toolbar draws in the accent at rest; a toggle's On stays its filled form (decision 81).
- **The face shows its group's active cursor** and chooses it with one click; a click on the chosen face opens its amounts where it has some, and otherwise does nothing.
- **The chevron opens the group's list,** a popup menu through the popup service, native on the Mac and drawn elsewhere (decision 40), hanging under it:
  1. the group's cursors in their order, each its icon, name and key, a check on the active one; choosing one makes it the active one and chooses it at once; one not ready is drawn disabled with its reason;
  2. a separator, where the group has settings;
  3. the group's settings: a toggle as an item with a check, a choice as a heading with its options under it, the one in use marked.
- **A group of one with settings,** Copy and apply, has the chevron for its settings alone.
- **The well never folds.** The actions fold before it, so the built narrowing's steps start wider by the well and the 16 points before it, and the window keeps 24 points of drag strip, counted on both sides of the well together. Under decision 92's overflow it is a part, never folded; where the parts no longer fit, at a large interface size, the window's minimum width grows to them. The test that holds the Toolbar's fold widths (`the_toolbar_folds_where_gui_md_says`) gains the well, in every language.
- **Hidden** (decision 92's toolbar, Phase 3): every cursor with a key stays reachable by it, and every cursor's work by its tool's own button, so a cursor with no key, as Exposure's, waits for the well or a key the person gives it: Select on Esc, Pan on A held, a picker cursor's work by its tool's picker button, a mask's by the Layers section's buttons, Copy and apply's by Copy edit and Apply edit.

**Amounts,** a cursor's continuous values (a brush's size, hardness and flow), are no list items, since a list holds choices: a click on the chosen face opens them as slider rows under it, drawn as tool rows on `raised` with `menu_radius`, `menu_edge` and `menu_shadow`, 240 points wide, values kept as app settings. A face whose cursor has amounts shows, while chosen, a 4-point triangle at its bottom end corner in `text_secondary`. It closes on Esc, a second click on the face or a press elsewhere; a press on the photo closes it and still acts, so a stroke is never lost to a click. `[` and `]` change the size without it, by darktable's step. The first cursor with amounts is Phase 4's Brush.

## Group settings

A group's settings are toggles and choices its cursors share, shown under its cursors in its list. They are app settings in `settings.toml`, `[cursors.<group>]`, declared in the settings schema, never history and never sent with a command but where a setting changes what a command does. One group's settings are declared once, by the group's owner (`CursorGroupDef`).

| Group | Setting | Kind | Default | When |
| --- | --- | --- | --- | --- |
| Pick | Show the last pick on the photo (`cursors.pick.show_last`) | Toggle | On | Phase 2 |
| Copy and apply | A click applies: The whole edit, The parts chosen in Copy Parts… (`cursors.copy_apply.what`) | Choice | The whole edit | Phase 3 |

Each later group's settings are chosen with that group: Mask's and Retouch's with Phase 4, Annotate's with its decision.

**The last pick's mark:** with Show the last pick on, the area a picker cursor last read stays drawn on the photo as a stored area is, secondary and dashed ([pickers.md](pickers.md#on-the-photo)), until the next pick, another photo or another cursor chosen. It is no sample: Select does not take it.

## Keys

| Cursor | Default | darktable |
| --- | --- | --- |
| Select | Esc, where nothing focused keeps it | Esc leaves a mode |
| Pan | A, held | A, held: force pan, zoom and rotate with the mouse |
| White balance picker | K | Free in darktable's defaults and in Apertrust's map |
| Exposure picker | None | E is darktable's Exposure slider |
| Mask and retouch cursors | Chosen with Phase 4, Brush among them | B is darktable's collapsing controls |
| Copy and apply | None; Copy edit and Apply edit keep Ctrl+Shift+C and Ctrl+Shift+V | |

- **A tap chooses a cursor and it stays;** pressed again, it stays chosen. **A hold borrows it:** held past darktable's long-press time, it acts only while held, and the cursor chosen before returns on release. Pan's A is a hold alone, as H's peek is: Pan is chosen while A is held, never kept, so a tap of A does not keep it. Phase 2 has taps alone, and Pan's A; the tap and the hold on one key come with decision 98's dispatcher (step 5), the plain key then acting at release.
- **Esc chooses Select** when no focused widget keeps it: a focused picker on the photo turns off first, a value field or a list closes first, as gui.md's widget scope has it ([Who owns the keys](gui.md#who-owns-the-keys)).
- **Space stays Next Photo,** and ← and → change the photo with any cursor chosen; H still peeks.
- **While a cursor is chosen** the keys held ride with its gesture: Ctrl (Cmd on the Mac) and Shift with a pick, as `PickKeys` ([pickers.md](pickers.md#commands)); Alt subtracting with the Brush, setting Clone's source and copying with Copy and apply.

## On the photo

**Gestures,** in the photo's coordinates, 0 to 1 across the photo as export renders it, at every zoom ([pickers.md](pickers.md#regions)):

| Gesture | Is |
| --- | --- |
| Click | A press and release that moves less than egui's drag threshold |
| Area | A drag past egui's drag threshold, from the press to the release, its end moved where needed to lie at least one frame pixel from the press on each axis, so a drag along one axis is an area too |
| Shape | A drag whose direction counts, from its start to its end: a gradient |
| Stroke | The pointer's path while pressed, with the pen's pressure where it gives one |
| Points | Clicks and drags that build one shape, a path, until it is closed |

- **A gesture is the chosen cursor's only where it declares it:** a click with Pan does nothing, a drag with a cursor that takes clicks alone is a click where it began.
- **While a gesture is drawn,** its owner draws it live with the on-photo overlay (gui.md, On-photo overlays): an area's rectangle with its handles, a gradient's lines, a stroke's path. Esc gives the gesture up, sending nothing. In a zoomed view a drag past the view's edge scrolls it, as a picker's does.
- **Panning with any cursor:** a plain scroll moves a zoomed photo and Shift+scroll sideways, the trackpad's two fingers both ways, A held drags it while no tool's picker is on, Alt with the arrows moves it; Ctrl+scroll, the pinch and the middle click zoom ([gui.md](gui.md#panning)). A scroll over the photo with a mask or retouch cursor pans too, provisionally ([Open questions](#open-questions)).
- **Touch and pen** (Touch mode, planned): a tap is a click, a drag a drag or a stroke; two fingers pan and pinch with every cursor.

**The pointer** over the photo, from egui's system pointers:

| Cursor | Pointer |
| --- | --- |
| Select | The move pointer over a mark it can move; elsewhere as Pan's |
| Pan, or A held | The open hand over a photo that can move, the closed hand while dragging, the arrow at Fit |
| A picker cursor | The crosshair |
| A cursor that cannot act | The not-allowed pointer |
| Brush, Heal, Clone | No system pointer: the overlay draws the brush's two circles, its size and its hardness (Phase 4) |

**A picker cursor's press** is `PickerCommand::Once` ([pickers.md](pickers.md#a-one-shot-pick)), with the tool's picker settings as the app keeps them, the channel its tool shows and the keys held:

- **A click** sends `Region::Point` at the point: the session reads what a plain click there gives the picker in darktable, for White balance and Exposure, area pickers both, the area a press on the photo starts, 2 % of the photo's width each way (`Region::pressed`).
- **A drag** sends `Region::Area`, the area drawn.
- **Then** the tool's patch and Last pick follow the pick's reading as they follow any pick, and where the pick makes a step, the photo changes and one Undo takes it back. The live region says what the pick changed, as a picker's does ([pickers.md](pickers.md#accessibility)).
- **Each press that changes the edit is a step,** its tool's turning on included, none where it neither turns the tool on nor writes anything new: click, look, click again. A drag shows its area while drawn but reads only on its release; the picker button is the way to watch live readings as an area moves.

## Keyboard and screen readers

- **F6** reaches the Toolbar, and Tab passes the actions to the well, one Tab stop landing on the chosen cursor's face, or the first face; ← and → move between the faces and chevrons; Space or Enter presses one; ↓ on a face, or Space or Enter on a chevron, opens the group's list, but Enter or ↓ on the chosen face with amounts opens its amounts, its list then on its chevron alone.
- **AccessKit:** the well a radio group named "Cursors"; each face a radio button named by its active cursor with its key ("White balance picker, K"), checked while its cursor is chosen, its tooltip its description; each chevron a button named "Pick cursors and settings" that opens a menu and says whether it is open; a face disabled with its reason as description.
- **A change of cursor** is said politely, its name alone ("White balance picker"), by a key or from a list; a click on a face is its own announcement.
- **Every cursor is reachable without the pointer:** its key and the well by keyboard, and its work by its tool's own button. A cursor's gesture still needs a place on the photo: a picker's area is placed by keys through its picker button, whose region the arrows move ([pickers.md](pickers.md#on-the-photo)).

## The API

**In `apertrust-tools`**, data only, no egui, so a tool declares its cursor beside its pickers:

```rust
/// One cursor of the well.
pub struct CursorDef {
    /// snake_case, unique among its owner's cursors: `spot`, `select`, `brush`.
    pub id: &'static str,
    pub group: CursorGroup,
    /// One of `apertrust-ui`'s icons by name, as a toolbar item names its icon.
    pub icon: &'static str,
    /// Its default key, the trigger as text in Qt's portable form ("K"), or none.
    pub key: Option<&'static str>,
    /// Its place in its group, 100 apart.
    pub order: u16,
    pub gestures: &'static [Gesture],
    pub surfaces: &'static [Surface],
    /// Its amounts: setting rows of the settings schema, as a tool's `Row::Setting`.
    pub options: &'static [Row],
}

/// The groups, in the well's order.
pub enum CursorGroup { Select, Pan, Pick, Mask, Retouch, Annotate, CopyApply }

/// A group's own settings, toggles and choices, declared once by its owner.
pub struct CursorGroupDef {
    pub group: CursorGroup,
    /// `Row::Setting`s naming settings of `[cursors.<group>]`.
    pub settings: &'static [Row],
}

pub enum Gesture { Click, Area, Shape, Stroke, Points }
pub enum Surface { Viewer, Browser }

/// A tool's picker cursor: a click or a drag picks once with `picker`.
pub struct ToolCursor {
    pub cursor: CursorDef,
    pub picker: &'static str,
}
```

- **`ToolDefinition::cursors: &'static [ToolCursor]`,** in the parent every tool embeds, as `menus` and `toolbar` are (decisions 91 to 93). A tool's cursor runs no code of the tool's own.
- **`ProcessingDefinition::check` refuses** a cursor naming a picker the tool does not declare, or one that writes no control (a picker that only shows, a reference's); a group other than Pick; a gesture other than a click and an area, or an area for a point picker; a surface other than the Viewer; two cursors for one picker, two of one id, and options on a picker cursor. A utility's `cursors` stay empty until the Colour readout's design.
- **The first two:** White balance's `spot`, its key K, and Exposure's `exposure`, with no key, each with a click and an area, and each with an icon of its own, not its picker's, so the Pick face shows which is active: White balance's `pick-white-balance`, the pipette with a neutral square badge, and Exposure's `pick-exposure`, the pipette with a dot badge.

**In the app:**

```rust
/// A finished gesture, or one under way, in the photo's coordinates.
pub enum CursorInput {
    Click { at: [f32; 2], keys: Keys },
    Area { from: [f32; 2], to: [f32; 2], keys: Keys },
    Shape { from: [f32; 2], to: [f32; 2], keys: Keys },
    Stroke { points: Vec<StrokePoint>, keys: Keys },
    Points { points: Vec<[f32; 2]>, closed: bool },
    /// A click on a photo, in the Viewer or on a thumbnail.
    Photo { photo: PhotoRef, keys: Keys },
}

/// What every owner of cursors but a tool implements.
pub trait CursorOwner {
    fn cursors(&self) -> &'static [CursorDef];
    fn groups(&self) -> &'static [CursorGroupDef] { &[] }
    /// Whether a press can act now, or why not, in the interface's words.
    fn ready(&self, cursor: &str, cx: &CursorContext) -> Result<(), Message>;
    /// A gesture under way: what to draw on the photo; it sends nothing.
    fn preview(&mut self, cursor: &str, input: &CursorInput, cx: &mut CursorContext);
    /// A gesture finished: what the app sends, session commands or the app's intents.
    fn finish(&mut self, cursor: &str, input: CursorInput, cx: &CursorContext) -> Vec<Intent>;
    /// A gesture given up, by Esc, another photo or another cursor.
    fn cancel(&mut self, cursor: &str) {}
}
```

- **The owners:** the app's own (Select, Pan, Copy and apply, which owns Copy and apply's settings and Pick's), every tool's through one built-in owner that turns a click or an area into `Once`, layers and masks with their group settings (Phase 4), retouch (Phase 4), annotations (later).
- **The registry,** made once as the app starts: every owner's cursors and group settings, refusing one owner's id twice (a tool's cursor known by its tool and its id, as its action is), a key twice among cursors, a group's settings declared twice, and ordering them by group, then `order`.
- **The mode,** `Cursor { owner, id }`, the app's state, one at a time; each group's active cursor beside it, for the run.
- **The Viewer and the Browser** read a press as the chosen cursor's gestures allow, hand the input under way to `preview` and the finished one to `finish`, and send what comes back; a press goes to a tool's picker first while one is on.

## Actions and words

- **Each cursor is an action** generated as the app starts, as a tool's buttons are (decision 91), of decision 98's `hold` kind: `on` chooses it (its default effect, what its key, its face and its menu items run), `hold` borrows it while held, `off` chooses Select, `toggle` chooses it or, chosen already, Select.
  - A tool's: `tool-<tool>-cursor-<cursor>`, under its tool in the actions tree, `tool-temperature-cursor-spot` on K.
  - The others': `cursor-<id>`, the app's under Photos, `cursor-select` on Esc and `cursor-pan` on A held, layers' and retouch's under `<blending>`.
- **The words,** in Fluent: a tool's cursor's name `tool-<tool>-cursor-<cursor>` ("White balance picker") and how to use it `tool-<tool>-cursor-<cursor>-how`, or, where its picker's words follow a setting (`Picker::words_by`), the picker's own; the others' `cursor-<id>` and `cursor-<id>-how`; a group's name `cursor-group-<group>` for its chevron. The tooltip reads "Name · key — how", "Click again for amounts" added where a cursor has them.

## What a script sees

Nothing new: a cursor is interface state, and its gestures send the commands their owners send. A script picks with `PickerCommand::Once` ([session-protocol.md](session-protocol.md#commands)), applies an edit with Apply edit's command, and, with layers, adds a mask part with the layers' commands. An MCP tool does the same through the session protocol.

## Against darktable

| darktable at the reference release | Apertrust | Why |
| --- | --- | --- |
| No cursors: each module's picker button, the mask manager's and each module's drawn-mask buttons | A well of cursors on the Toolbar, beside the same buttons | One place to choose what a press does, for every owner, as commercial raw editors offer; the buttons kept as darktable's |
| A drag on the photo pans it unless a module takes it; A held forces the pan | Select, the default, pans from a drag on empty photo and moves the mark a drag starts on; Pan pans from any drag; A held pans from any cursor; scroll pans | darktable's drag-to-pan kept; A keeps darktable's meaning |
| A pick only through the picker's button, live while on | Also a picker cursor's click or drag, one reading as one step where it changes the edit | Quick repeated picks, a series picked with one key |

## Where the code lives

| Part | Where |
| --- | --- |
| `CursorDef`, `CursorGroup`, `CursorGroupDef`, `Gesture`, `Surface`, `ToolCursor`, `ToolDefinition::cursors` and their checks | `apertrust-tools`' `definition`, a `cursor` module |
| White balance's and Exposure's cursors | `apertrust-tools`, each tool's file |
| `Region::pressed` | `apertrust-tools`' `pick` |
| `PickerCommand::Once` | The engine's `picker` module and `session/pickers.rs` |
| The well, its faces, chevrons, lists and amounts | `apertrust-ui`'s components, in the gallery |
| `CursorInput`, `CursorOwner`, the registry, the mode, the app's own cursors and the tools' owner | The app's `cursors` module |
| The gestures and the pointer over the photo | The app's Viewer, beside `pickers.rs` |
| The cursor actions | The app's actions, generated as tools' actions are |
| `[cursors.<group>]` | `data/settings.schema.toml` |

No new crate.

## Tests

- **The tools** (`definition.rs`): each refusal of the check; White balance's and Exposure's cursors accepted; a test holding every cursor's icon name to one `apertrust-ui` has.
- **`Region::pressed`** against darktable's new box restated (`darkroom.c:4064-4073`), clamped at each edge, and the app's press on the photo with a picker on making the same area.
- **The session:** `Once`'s tests, listed with it ([pickers.md](pickers.md#tests)).
- **The design system** (`crates/ui/tests`): the well's states in each theme, a face's click and a chevron's, the list's items, checks and settings, the keys inside the well, the AccessKit tree, snapshots of Phase 2's well and of every group, dark and light.
- **The app:** a click and a drag with each picker cursor sending `Once` with the tool's settings and waiting for its `Saved`, one step each; a click with Select and with Pan sending nothing; Select's drag on empty photo and Pan's drag on a mark panning as the built drag does; A held panning from a picker cursor and the cursor back on release; Esc choosing Select only where nothing focused keeps it; K choosing White balance's cursor, and the key shown in its tooltip and list; the chosen cursor kept across another photo and while the photo goes live; a press while it cannot act sending nothing, with its pointer and its reason; a tool's picker on taking the photo first; Show the last pick drawing and clearing the mark; the Toolbar's fold widths with the well, in every language, at 500 points on both platforms.

## The plan

Phase 2, one pull request each, each reviewed before the next that needs it:

| # | Delivers | Touches | Out of scope |
| --- | --- | --- | --- |
| 104.1 | Built: `PickerCommand::Once` and `Region::pressed`, with their tests (79.17) | tools, engine | the app |
| 104.2 | `CursorDef`, `CursorGroup`, `CursorGroupDef`, `Gesture`, `Surface`, `ToolCursor`, `ToolDefinition::cursors` and the check; White balance's and Exposure's cursors and their words in every language | tools | the app |
| 104.3 | The well in the design system: faces, chevrons, states, the list with its settings, keys and AccessKit, in the gallery with snapshots | ui | amounts, the app |
| 104.4 | The app: the mode, the registry, `CursorOwner`, Select and Pan, the tools' owner; the gestures and pointers over the photo; the well on the Toolbar between two flexible spaces and the narrowing with it; the cursor actions with Esc, K and A held; Pick's Show the last pick in the settings schema | app, settings schema | the toolbar's list, holds on one key, Copy and apply |

**Later, with their features:** Phase 3, the `cursors` item in decision 92's toolbar, a key's tap and hold with decision 98's dispatcher, and Copy and apply with Copy Parts and the Browser's selection; the Colour readout with the Colour picker section (79.16); Phase 4, the Mask and Retouch groups with layers, their settings, amounts with `[` and `]`, their keys and the Layers section's buttons sharing their state; Annotate with its own decision.

## Open questions

- **Scroll over the photo with a mask or retouch cursor:** it pans, as with every cursor; whether it changes the brush's size instead, as `[` and `]` do, is settled with masks in Phase 4. Meanwhile there are no such cursors.
- **A mask cursor with no layer to draw on:** whether a press makes a new layer or the cursor cannot act, saying so, is settled with layers. Meanwhile there is none.
- **The Colour readout's commands:** whether a click moves the Colour picker's live sample or adds a stored sample, settled with the Colour picker section (79.16). Meanwhile it is not drawn.
- **What Select selects:** stored samples' marks (79.14) and a mask's shapes (Phase 4) are its first; whether the last pick's mark or a picker's area become selectable is settled with them. Meanwhile Select's drag pans and its click does nothing.
