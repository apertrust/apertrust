# 0098. Actions and shortcuts after darktable

## Context

Apertrust has one actions registry for buttons, menus, tooltips and keys ([0036](0036-the-design-system.md), [0039](0039-one-top-bar-native-menus-on-the-mac-one-menu-button-elsewhere.md)), a default keyboard map, keys overridden by id in `settings.toml`'s `[keys]` ([0043](0043-settings-and-apertrusts-folders.md)), tools' buttons generated as `tool-<tool>-<action>` ([0091](0091-context-menus-places-and-tools-items-as-actions.md)) and toolbar items that are registry actions ([0092](0092-the-toolbar.md)). It has no way to give a key to a tool's control, no page to set keys on, and a key is only ever a press.

darktable at the reference release has a whole framework for this (`src/gui/accelerators.c`, `src/common/action.h`, `src/bauhaus/bauhaus.c`, `src/develop/imageop.c`, `src/libs/lib.c`, `src/libs/tools/global_toolbox.c`, `data/shortcutsrc`): every widget an action with elements and effects, shortcuts of keys, presses, clicks and moves at a speed, fallbacks, a shortcuts page, visual mapping, MIDI and gamepads, and Lua's calls. The first pass mirrors darktable ([0002](0002-rewrite-darktable-first-then-apertrusts-own-way.md)), so the question is how much of it is ported, and where Apertrust's decisions make it differ.

## Decision

**darktable's action and shortcut framework is ported whole,** with its model, its names and its behaviour: one registry shown as darktable's tree; every tool control generated as an action of its widget's kind, as bauhaus defines it; darktable's elements and effects per kind, by their words; darktable's shortcut, with its key, modifiers, presses, mouse button and click, move and direction, element, effect, speed, instance and device; fallbacks, off by default as darktable has them; the Shortcuts page with darktable's two lists; visual mapping; the accels window. Three things Apertrust has stay: stable ids rather than label paths; `[keys]` holding only what differs from the defaults (43); and one view (37). Every place it differs from darktable is in [the deviations](#deviations-from-darktable) below. All of it is planned, in the steps under [Consequences](#consequences); the registry, the default keyboard map and `[keys]` as plain keys are built.

### Names

darktable's terms are kept, but where a term already means something else here.

| darktable | Apertrust prose | Apertrust code |
| --- | --- | --- |
| action, `dt_action_t` | action | `Action`, `ActionId` (built) |
| the action tree | the actions registry, shown as a tree | `actions::Registry`, `Action::owner` |
| action definition, `dt_action_def_t` | kind | `Kind` ("definition" names `ToolDefinition` here, [0093](0093-one-tool-model-processing-tools-and-utility-tools.md)) |
| element | element | `ElementDef`, its index a `u8` |
| effect | effect | `Effect` |
| shortcut, `dt_shortcut_t` | shortcut: a trigger and what it does | `Shortcut`; the built `Shortcut { modifiers, key }` becomes `Trigger` |
| a registered default, `is_default` | default shortcut | the app's `DefaultShortcut`, replacing `Binding`, which names a tool's binding to parameters here; a tool declares its own as `apertrust_tools::ShortcutDef`, plain data the app converts |
| fallbacks | fallbacks | `Fallback` |
| speed, instance | speed, instance | `Shortcut::speed`, `Shortcut::instance` |
| input driver | input device, its driver | `Device`, `Driver` |
| visual shortcut mapping | visual mapping | `Mapping` |
| view | scope | `Scope` |
| processing module, utility module | processing tool, utility | the kinds keep darktable's words in data, `processing module` and `utility module`, and the interface shows them translated |

### The registry, shown as a tree

The registry stays flat by id. Each action gains an **owner**, which places it in a tree for display, as darktable's owner chain does; nothing is stored by path. The roots, translated:

| darktable's root | Apertrust's | Holds |
| --- | --- | --- |
| `global` | Global | `Scope::App` actions, in darktable's sections |
| `views` | Photos | `Scope::View` actions: the photos, the Viewer, the Browser, rating and labels |
| `lib` | Utility tools | each utility's node, then its rows' actions |
| `iop` | Processing tools | `<focused>` first, then each processing tool: its node, its tabs, its rows' actions |
| `iop/blend` | Processing tools › `<blending>` | layers' and masks' actions, Phase 4 ([0034](0034-layers-masked-tool-instances-in-a-fixed-order.md)) |
| `lua` | Scripts | with scripting |
| `fallbacks` | Fallbacks | one node per kind |

**Ids are stable and ASCII; the tree shows the translated path.** Tool, control and action ids are snake_case with no hyphen, so `-` always splits a family's parts:

| Family | Id | darktable's |
| --- | --- | --- |
| The app's actions | as built: `undo`, `full-screen`, `panel-tab-3` | `global/…`, `views/…` |
| A workspace | `workspace-<id>` (built, [0096](0096-workspaces-as-files-hidden-tools-and-finding-a-tool.md)) | `lib/modulegroups/preset/<name>` |
| A tool | `tool-<tool>` | `iop/<module>`, `lib/<lib>` |
| A tool's tabs | `tool-<tool>-tabs` | `iop/<module>/page` |
| A control or a view coordinate | `tool-<tool>-control-<control>` | `iop/<module>/<section>/<label>` |
| A button or a `Row::Action` | `tool-<tool>-<action>` (built, 91) | the button's |
| A picker | `tool-<tool>-picker-<picker>` | the picker's toggle |
| A one-shot pick a tool's menus name (`MenuEntry::Pick`, [0079](0079-colour-pickers-one-api-for-tools-and-the-colour-picker.md)) | `tool-<tool>-pick-<picker>` | none |
| A tool's setting row | `tool-<tool>-setting-<setting>` | a widget on a conf key |
| A preset row | `tool-<tool>-preset_row-<id>` | a combobox |
| A preset | `tool-<tool>-preset-<name>`, with the presets API | `iop/<module>/preset/<name>` |
| A fallback node | `fallbacks-<kind>` | `fallbacks/<kind>` |

A preset's id is `tool-<tool>-preset-` and its name, encoded to stay ASCII and reversible: `a`–`z`, `0`–`9` and `_` as they are, and every other byte of the name's UTF-8, upper-case letters among them, as `~` and two lower-case hex digits (`été` is `~c3~a9t~c3~a9`, `C3A9` is `~433~419`), `~` taken by nothing else; every escape is three characters, so no two names meet. So the preset row's family is `preset_row`, with an underscore: no other family starts with `preset-`, and no preset's name can make another family's id. `tabs` is reserved: no button or `Row::Action` of a tool may take it as its id, since `tool-<tool>-tabs` is the tool's tabs, and the tools' check refuses it with the other id rules. The other families cannot collide with a button's id, which has no hyphen, as each adds a further part after its word. Renaming a preset moves its shortcuts, as darktable's `dt_action_rename_preset` does; it is the one rename the registry handles. A test fails on two actions with one id, on a tool action named `tabs`, and on a family's words missing in any language.

### What becomes an action

Every action is generated from `ToolDefinition` as the app starts, as decision 91's are; a tool's author writes nothing for shortcuts. One action serves the tool wherever it is drawn, as darktable's `dt_action_define_iop` defines one on the module's class. **One action per control,** `tool-<tool>-control-<control>`, however many rows draw it: a control with a row in each of a tool's modes (`Row::When`) acts on the row the current mode shows.

| In the definition | Kind |
| --- | --- |
| A `ProcessingDefinition` | `processing module` |
| A `UtilityDefinition` | `utility module` |
| More than one `Tab` | `tabs`, its elements the tabs |
| `Row::Slider`, a control's or a view coordinate's | `slider` |
| `Row::Choice`, `Row::Preset` | `dropdown`, each option an `item:` |
| `Row::Toggle`, `Row::Pickers`' buttons | `toggle` |
| `Row::Buttons`' buttons, `Row::Action` | `button` |
| `MenuEntry::Pick` in `ToolDefinition::menus` | `button`, run only from the photo's menu, with its point |
| A slider's quad | the slider's `button` element |
| `Row::Setting` | `slider`, `dropdown`, `toggle` or `entry`, by its kind |
| `Row::Custom` (the curve, the equalizer, the wheels) | what darktable defines for that module's widget, listed in its port spec |

The app's actions keep their ids and get a kind: `command` (Undo, Open…), `toggle` (Full Screen, the Map), or `hold` (H's peek, the accels window).

### Kinds, elements and effects

`Kind` is darktable's `dt_action_def_t`: its name, its elements each with its effects, its fallbacks, and `process(target, element, effect, size)`, which performs the effect and returns the value; with no size it only reads, as `DT_READ_ACTION_ONLY` does, for tooltips, a device's feedback and scripts. `Effect` has one variant per darktable word. A test holds every list to darktable's arrays, word for word and in order, but `enable`, below.

| Kind | Elements: effects |
| --- | --- |
| `slider` | `value`, `force` and `zoom`: edit, up, down, reset, top, bottom, set; `button`: toggle, on, off, ctrl-toggle, ctrl-on, right-toggle, right-on |
| `dropdown` | `selection`: popup, next, previous, reset, last, first, `item:<option>`; `button`: the toggle effects |
| `toggle` | toggle, on, off, ctrl-toggle, ctrl-on, right-toggle, right-on |
| `button` | activate, ctrl-activate, right-activate |
| `entry` | focus, start, end, clear |
| `hold` | hold, on, off, toggle |
| `tabs` | one element per tab: activate, next, previous, reset |
| `processing module` | `show`: the toggle effects; `reset`: the activate effects; `presets`: show, previous, next, store, delete, edit, update, preferences; `focus`: the toggle effects; `instance`: show, move up, move down, new, delete, rename, duplicate |
| `utility module` | `show`, `reset`, `presets`, as above |
| `command` | none |
| `modifiers` | shift, ctrl, alt: the hold effects, with input devices |

- **A slider's** up and down move by its step × size × speed × precision, darktable's `_slider_add_step`, within the span shown; `force` goes past the soft range to the hard one; `zoom` changes the span shown, as the popup's wheel does; `edit` opens the slider popup (42), the value field until it is built; `set` sets the speed as the value.
- **A dropdown's** next and previous skip disabled options; `item:` names the option by its id, where darktable uses its untranslated words.
- **A tool's node:** `show` does what darktable's `_show_module_callback` does, a hidden tool shown in the Hidden tab as decision 96's reveals are, its tab chosen, its section opened or closed; `reset` is the header's ↺, `ResetTool` or the utility's own reset (93); `presets`' previous and next move through its presets, as `dt_gui_presets_apply_adjacent_preset` does, with the presets API; `focus` with the focused tool, and `instance` with layers. An element not built yet is listed and does nothing.
- **No `enable`:** darktable's element that turns a module's power button on and off is not ported. A person never turns a tool on or off explicitly ([0032](0032-tools-off-until-first-touched.md)): a switched tool is on once a control changes, and its reset turns it off.
- **The `<focused>` node,** darktable's `iop/focus`, targets the focused tool: its sliders, dropdowns and buttons as the elements `1st` to `20th`, its tabs, `show`, new and delete instance, and darkroom's cycle through tools. It comes with the focused tool.
- **The precision** of every step, darktable's `accel/slider_precision`, is the setting `shortcuts.slider_precision` (normal, fine or coarse, ×1, ×0.1 or ×10, darktable's multipliers), with no row in Settings, as darktable has none, and the action that cycles it, as darktable's.

**What a change makes.** Only an effect that changes a value makes a change: a slider's `set`, `up`, `down`, `reset`, `top` and `bottom`, on its `value` or `force` element; a dropdown's `next`, `previous`, `reset`, `last`, `first` and `item:`; a toggle's `toggle`, `on`, `off` and their ctrl- and right- forms; a button's `activate` and its ctrl- and right- forms, as its button runs it; a tool's `reset` and its presets' `previous` and `next`. On a processing tool's control it is one completed change, `SetControl { done: true }`, `ResetControl`, `ResetTool` or `ApplyPreset`, as darktable's `dt_bauhaus_slider_set` makes one history item; repeated effects on one control merge into one step by decision 78's rule. No gesture is opened, so a shortcut needs nothing of the session the wheel does not. **An effect that opens or shows something makes no change** and never turns a switched tool on: a slider's `edit`, which opens the slider popup (the value field until it is built), a dropdown's `popup`, which opens its list, a tool's `show` and `focus`, a presets' `show`, a zoom of the span shown. What the popup then changes is the popup's own ([0042](0042-numbers-shown-typed-and-stored.md)): live changes as a gesture, Enter keeping them as one step, Esc or a cancel putting the value back (`CancelGesture`). An effect on a utility's setting row goes through the app's settings store (`set`, [0093](0093-one-tool-model-processing-tools-and-utility-tools.md)), never through the session, as the row itself does. A control whose rows the current mode all hides, or one the photo cannot take, is not active: the shortcut says "not active", as darktable's toast does.

**Every menu item, toolbar item and context menu item** names an action and runs it at its default element and effect (91, 92). A menu shows an item's key only for a plain key at the default element and effect.

### The shortcut

```rust
pub struct Shortcut {
    pub trigger: Option<Trigger>,    // None: a speed adjustment
    pub action: ActionId,
    pub element: u8,                 // 0: the kind's first
    pub effect: Option<Effect>,      // None: the default
    pub speed: f32,                  // 1.0; for `set`, the value
    pub instance: i8,                // 0 preferred, 1 first, -1 last; only 0 until layers
    pub default: bool,
}

pub struct Trigger {
    pub key: Option<Input>,          // none only for a fallback: a shortcut's click or move needs a key held
    pub mods: Mods,                  // Ctrl (Cmd on a Mac), Meta, Alt, Shift
    pub press: Presses,              // one to three, or long
    pub buttons: Buttons,            // left, middle, right
    pub click: Presses,
    pub moved: Option<(Device, Move)>,
    pub direction: Option<Direction>,// up or down: half of a move
}

pub enum Input { Key(egui::Key), Device { device: Device, code: u32 } }
pub enum Move { Scroll, Pan, Horizontal, Vertical, Diagonal, Skew }
```

- **Moves:** a key held while the wheel turns is `scroll`, the size the notches; a key held with a drag is `horizontal`, `vertical`, `diagonal` or `skew`, a step per 10 points, chosen as darktable's motion branch chooses; a sideways wheel is `pan`. A move needs a key held.
- **Presses and clicks:** a double, triple or long press of the key, and a click, double, triple or long, of a mouse button while the key is held, as darktable has them.
- **No key, no shortcut:** as darktable's `dt_shortcut_dispatcher`, which passes a scroll or a button event on to the widget under the pointer while no key is held, a move or a click with no key held is never a shortcut. A trigger of modifiers alone, with or without a click or a move, exists only as a fallback, which the widgets read themselves (`dt_accel_get_speed_multiplier` and the fallback tables); a speed adjustment has no trigger at all.
- **A speed adjustment** is a shortcut with no trigger, an action and a speed, darktable's `_shortcut_is_speed`: it scales that action's every move.
- **The device** is part of the shortcut from the start, darktable's numbering: 0 the keyboard and mouse, from 10 each driver's devices.

### Which shortcut runs

- **Scopes keep darktable's meaning,** the scope coming from the action as darktable's views come from its owner. With one view there are three, the focused widget's fixed list (`Focus::keeps`), the view's and the app's, and the narrowest wins.
- **The dispatcher runs in eframe's `raw_input_hook`,** before egui reads the input: only while a key that a shortcut with a click or a move uses is held does it take the wheel, pointer and button events out of the frame, as darktable's seat grab does; with no such key held they go to the widget under the pointer, so a row keeps Ctrl+wheel by its own wheel rule (42; [gui.md](../gui.md#the-scroll-wheel-over-a-row)); it keeps the keys held, the presses, the move's start and a delayed release across frames; it asks for a repaint after the double-click time; its clock is injected for tests.
- **A key acts at press** only where nothing longer is defined on it: no move, no click and no double, triple or long press shortcut on that key and modifiers, and fallbacks off or a kind without them. A key with any longer shortcut on it acts at release, as darktable's, after the double-click time where a multi-press exists, and a move or a click while it is held cancels its plain action: with Right bound to the next photo and Right with the wheel to a slider, Right pressed and released goes to the next photo, and Right held while the wheel turns moves the slider alone. A hold acts on press and release; a move acts at each step. A key that holds an action, as H holds peek, takes no longer shortcut: the Shortcuts page and mapping mode refuse a move, a click or a longer press on it as a clash, naming the hold, and load refuses one in `[keys]` with a warning, the same test, so a held key never starts its action under a move meant for something else. So the arrows act at once and repeat only while no move or click shortcut uses them.
- **The closest match wins,** darktable's `_shortcut_closest_match`: a move, a click or a longer press while the key is held matches the shortcut that has it, never the key's plain action, which it cancels; what is left over goes to the fallbacks, when they are on.
- **Fallbacks** are darktable's tables, off by default (`shortcuts.fallbacks`, darktable's `accel/enable_fallbacks`), turned on from the Shortcuts page. The value node's are darktable's: Ctrl a tenth of a step, Shift ten, Ctrl+Shift ten, a horizontal move a tenth, a vertical move ten, a left double-click reset, with a vertical move top. They are the same modifiers as a slider's fine step on its arrows and its wheel (42), one table, as darktable's `dt_accel_get_speed_multiplier` reads its fallbacks for the mouse; on the Mac Option takes Ctrl's place, as decision 42 gives it. The processing module's left click, darktable's `enable`, is not ported. A person may add, change and delete fallbacks of their own on the Shortcuts page's fallbacks branch, as darktable lets them, listed with darktable's; each is recorded as darktable records one, its recording starting with a keyless press (`_grab_in_tree_view`), so modifiers, a button or a move with no key become its trigger, which needs step 5's recording. The slider's Ctrl+Shift to `force` waits on Ctrl+Shift's steps past the soft range, still open.
- **Conflicts** on the page and in mapping mode ask darktable's questions as alerts ([0041](0041-dialogs-as-native-modal-windows.md)): remove the clashing shortcuts, reset a shortcut's settings, disable a default or remove a shortcut, split a move into up and down; Ctrl-clicking the mapping button skips them for the session. A clash is two shortcuts with the same whole trigger, compared part by part: the key and its modifiers together by Apertrust's matching (`actions::conflict`: logical key, layouts, AltGr, the digit row, and the modifiers a symbol may need, so Ctrl++ and Ctrl+Shift++ are one), then the presses, the buttons and click, the move and its direction. Right alone and Right with the wheel are two triggers, no clash, and both survive load; the page and load apply this one test, so a pair the page accepts is never refused at a restart. A speed adjustment has no trigger and clashes with nothing. The keys kept from actions stay kept (39).
- **On load, a person's shortcut wins** over another action's default it clashes with, the default left out with a warning and listed among the disabled defaults, as darktable's load does, so a person's key survives a later version's new default. Two of a person's own that clash, which only a hand-edited file can hold, are both refused, as now.
- **The Mac's native menus act at press** (39), so an item shows no key equivalent where its key waits for a longer press; the dispatcher keeps that key.

### Kept: `[keys]`, differences only

`settings.toml`'s `[keys]` keeps an action's list of shortcuts where it differs from the built-in defaults, always compared with them whatever `shortcuts.load_defaults` says, the list replacing that action's defaults, an empty one disabling them, as now. A plain trigger stays a string in Qt's portable form (`"Ctrl+Shift+Z"`); anything more is an inline table of darktable's fields by darktable's words:

```toml
[keys]
show-log = ["Ctrl+Shift+L"]
undo = [{ keys = "Ctrl+Z", default = true }]   # assigned while loading defaults was off
tool-exposure-control-exposure = [{ keys = "E", move = "scroll" }, { keys = "Shift+E", effect = "edit" }]
tool-toneequal = [{ keys = "T", press = "double", element = "show", effect = "on" }]
fallbacks-value = [{ keys = "Ctrl", speed = 0.1 }, { keys = "Shift", speed = 10 }]   # in place of darktable's value fallbacks
```

The fields are `keys`, `press`, `button`, `click`, `move`, `direction`, `element`, `effect`, `speed`, `instance`, `device` and `default`; `keys` holds modifiers alone, or is left out, only under a fallback (`fallbacks-value = [{ keys = "Ctrl", button = "left", click = "double", effect = "reset" }]`); every other shortcut has a key. **A speed adjustment is stored apart,** in `[shortcuts.speeds]`, an action's id to its speed (`tool-exposure-control-exposure = 2.0`), which visual mapping's wheel writes and a restart keeps; it never sits in an action's list in `[keys]`, which replaces the action's defaults, so a speed set for Exposure leaves its default shortcuts in place and scales them. Existing files read unchanged.

**What the stored entries mean** depends on Load default shortcuts at startup (`shortcuts.load_defaults`, on), and turning it on or off never rewrites the file:

- **On:** the keymap is the built-in defaults, each action with an entry taking its entry's list instead; an entry equal to the action's defaults means what no entry means.
- **Off:** the keymap is the entries alone, darktable's blank slate; an action with no entry has no shortcut.
- **Written:** an entry equal to its action's built-in defaults is left out, but one whose shortcuts carry `default = true`, darktable's `is_default`: a shortcut a person assigns while loading is off that equals its built-in default is written so (`undo = [{ keys = "Ctrl+Z", default = true }]`), so it is there after a restart with loading still off.

darktable's file-based behaviour stays:

- **Restore…** offers darktable's three states, the defaults, as the app started, and as the page opened, kept in memory, with "clear all newer shortcuts";
- **Load default shortcuts at startup,** `shortcuts.load_defaults`, as above;
- **Export…** writes the same TOML, the keyboard's or one device's; **Import…** merges or replaces from such a file or from a darktable `shortcutsrc`.

**darktable's own files** come through one reader of darktable's grammar, mapping darktable's paths to Apertrust's ids through a table each port fills: the module to the tool, the widget's label and section to the control, `global/…` and `views/…` to the app's ids. **Import extras** reads `data/shortcutsrc` and `data/shortcutsrc.de`, darktable's, verbatim ([0045](0045-a-top-level-data-folder.md)), by language as darktable chooses; **Import…** also reads a darktable user's own `shortcutsrc`, so a person coming from darktable keeps their shortcuts. A fallback line for a kind Apertrust has is imported as the person's own fallback; a line naming a module not ported, a widget with no counterpart, or a fallback of a kind not ported, is listed, never guessed.

### The interface

- **Settings › Shortcuts** is darktable's page (`dt_shortcuts_prefs`): darktable's notice recommending visual mapping, with a close button (`shortcuts.hide_notice`); the actions tree, its columns the action and its kind; a splitter; the shortcuts list grouped in active view, fallbacks (while on), speed adjustments and disabled defaults, its columns shortcut, action, element, effect, speed (−1000 to 1000, three decimals) and instance (preferred, first, last, second, last but one, or ±1 to ±9), the last four edited in place; then Search actions and Search shortcuts, darktable's incremental searches; ?; Restore…; Enable fallbacks from step 6; Export…, Import extras and Import… from step 8, an export carrying `[keys]` and `[shortcuts.speeds]` together and an import merging or replacing both the same way; and Load default shortcuts at startup, and from Phase 4 the instance preferences. Settings stays one window (43), modal on Windows and Linux as darktable's dialog is, modeless on the Mac.
- **Recording** is darktable's `_grab_in_tree_view`. From step 3 the page records a key with its modifiers. From step 5, with the dispatcher, it records what darktable's does: a key's double, triple or long press, and a click or a move with a key held; a device's input with the devices. A trigger with no key, modifiers alone with or without a click or a move, is recorded only on the page's fallbacks branch, from step 6, as above; mapping assigns to a control's action, never to a fallback. A right-click with nothing held, or Esc, cancels, but on the fallbacks branch Esc alone, so a plain right-click can be recorded as a fallback's trigger; a key kept from actions is refused with its reason. A recording is announced as it starts, and each assignment as darktable logs it. Every part of a shortcut but its trigger can be edited without a mouse; the tree is an AccessKit tree and the list a grid.
- **Tooltips** end with each of the control's shortcuts and what it does, the element under the pointer first, hidden while a modifier or a button is held, as darktable's `dt_shortcut_tooltip_callback`.
- **Feedback** is darktable's toast as the message over the photo and a polite live region: "Exposure: +1.20 EV", "Exposure: not active", "Shift+E not assigned".
- **Visual mapping** is darktable's: while it is on, the cursor says whether the control under it can be mapped; a trigger over a control assigns it to the control's action at the element under the pointer, one already there removed with the confirmations; the wheel with nothing held sets its speed adjustment, ×10 a notch, and a middle click resets it; a click or a move over a control with a key held is recorded as a trigger, as on the page, and with no key held nothing is assigned; a plain left click, nothing held, ends the mode and opens Settings › Shortcuts at that control's action, as darktable's does; a right click, Esc or Map Shortcuts again ends it without opening Settings. Started from the Shortcuts page's button, Settings closes, being modal on Windows and Linux, and mapping runs in the main window; the Mac does the same, for one rule. Each row, Toolbar item and viewer bar toggle registers its rectangle, action and element for the frame; menus are not mappable, as darktable has none. Its action, Map Shortcuts (`map-shortcuts`), a toggle, is a Toolbar item outside the default list, so decision 92's default is unchanged, Help › Map Shortcuts, and a button on the Shortcuts page.
- **The accels window,** darktable's `show accels window`, every shortcut of the view in columns by category, is a hold action with no default key: H stays the peek. Help › Shortcuts Overview (`shortcuts-overview`) toggles it, its hold kind's toggle effect, and a person may give it a key.
- **Help › Keyboard Shortcuts** (`keyboard-shortcuts`) opens Settings at Shortcuts.

### Input devices and scripts

- **MIDI and gamepads** come later, on demand: darktable's drivers' behaviour on Rust crates, `midir` and `gilrs`, where darktable uses PortMidi and SDL2 ([0005](0005-rust-and-no-c-or-c-unless-no-rust-option-exists.md)), with a `Driver` trait of darktable's five functions, its devices numbered as darktable's, and `modifiers`, "reinitialise input devices" and `all_actions` with them. midir's libasound on Linux is confirmed under decision 5 then.
- **The engine never sees an action.** A control's action sends the session's own commands, so a script or an MCP tool already does through the session protocol what a control action does. The registry is the app's, as darktable's tree exists only with a GUI. When scripting is designed, the app offers darktable's `dt.gui.action` by Apertrust's ids to the scripts it runs, and `mimic`'s run-time actions under Scripts.

### Where the code lives

No new crate. `crates/app/src/actions/` splits today's `actions.rs`: `registry.rs` (the owned `Registry`, built once at start from `ACTIONS`, the tools, the utilities and the workspaces, taking actions added at run time, replacing the leaked generated ones; `ActionId` stays a `&'static str` newtype, so the built-in constants and every `Shortcut::action` keep their type: an id the app did not compile in, a tool's generated action at start or a preset added or renamed while it runs, is interned by `actions::intern`, a process-wide interner, a `static` set of texts behind a mutex (`LazyLock<Mutex<HashSet<&'static str>>>`), not a field of the `Registry`: the first time a text comes it is leaked once with `Box::leak`, safe Rust with no `unsafe`, and every later time the same `&'static str` is handed back, so the texts live for the whole run, each stored once however often it comes back, a rename adding one, the memory bounded by the number of distinct ids; every file, `[keys]`, `[shortcuts.speeds]` and an export, stores the text), `kinds.rs`, `shortcut.rs`, `keymap.rs`, `dispatch.rs`, `darktable.rs` and later `devices/`; the page is `settings_dialog/shortcuts.rs`, mapping mode the shell's. `apertrust-tools` gains `ToolDefinition::shortcuts: &'static [ShortcutDef]`, a tool's default shortcuts as plain data of its own, no app type: the target (the tool's node, a control or an action by id), darktable's element and effect words, and the trigger as text, which the app converts into its registry's `DefaultShortcut`, a test failing on one it cannot parse, so `apertrust-tools` never depends on the app; `apertrust-ui` a tree list, a grid with editable cells, a recording field and the mapping cursor, with no action types; `data/` darktable's `shortcutsrc` and `shortcutsrc.de`. The engine and the protocol do not change.

### Deviations from darktable

| # | darktable at the reference release | Apertrust | Why |
| --- | --- | --- | --- |
| 1 | Actions stored by untranslated label path | Stable ASCII ids, the translated path shown | Ids apart from words (43); a relabelled widget keeps its shortcuts |
| 2 | "Definition", `dt_action_def_t` | "Kind", `Kind` | "Definition" names tool declarations (93) |
| 3 | `shortcutsrc`, the whole set, with `.defaults`, `.backup` and `.edit` | `[keys]`, differences only, extended; restore points in memory; `load_defaults` a setting | Decision 43; a later version's new defaults reach a person |
| 4 | Triggers in GTK's key names, `b;ctrl` | Qt's portable `Ctrl+B`, darktable's words for the rest | The built form, the same on every platform; darktable's grammar still read by the importer |
| 5 | Keys act at release, waiting where a double press exists | At press where nothing longer can follow, else at release | The Mac's menus act at press (39); a press feels immediate |
| 6 | Only the arrows repeat | Undo, Redo, the zoom, F6 and the next and previous photo repeat too | Platform convention, built |
| 7 | The focused widget gets each event first and may decline it | A fixed list per kind of focus, `Focus::keeps` | egui's widgets run after the keys are read |
| 8 | The keypad's keys apart | The keypad is the number row | egui-winit gives both one `Key` |
| 9 | Matched by the first layout's keyval | Logical key, Cyrillic and the digit row by physical key, AltGr guarded | egui's key model; Ukrainian and Polish layouts |
| 10 | Any trigger can be assigned | The clipboard's keys, Alt with a letter, F10 and the Mac's system keys kept from actions | Decision 39 and the platforms |
| 11 | A processing module's `enable` element and its left-click fallback | Not ported | A person never turns a tool on or off explicitly (32) |
| 12 | A right-click cancels a recording | A right-click or Esc; on the fallbacks branch Esc alone, so a plain right-click can be a fallback's trigger | No keyboard trap, and a right-click fallback can be recorded |
| 13 | Ctrl-click in mapping mode toggles the quick access panel | Nothing | No quick access panel: workspaces take the module groups' place (96) |
| 14 | The shortcuts page is a preferences tab and a dialog of its own | Settings › Shortcuts alone, opened at the action from mapping mode | Settings is one window (41, 43) |
| 15 | The notice dismissed by three clicks | A close button | A hidden gesture is no affordance |
| 16 | Load default shortcuts at startup and the instance preferences under miscellaneous | On the Shortcuts page | They are about shortcuts |
| 17 | A tablet's buttons above 7 as a device | Not yet | egui reports two extra buttons |
| 18 | The moves leftright, updown and pgupdown declared | Not ported | Unused in darktable |
| 19 | PortMidi and SDL2 | midir and gilrs | Decision 5 |
| 20 | Lua's `dt.gui.action` and `dt.gui.mimic` | An app-side call when scripting is designed; the protocol's commands meanwhile | Engine–UI separation |
| 21 | One set of defaults, Ctrl the primary modifier everywhere | Some defaults per platform (Ctrl+Y on Windows and Linux, Close Window on the Mac; planned, rotation's Ctrl+R, Ctrl+[ and Ctrl+] on Windows and Linux, Alt+R being the menus' there (deviation 10) and the three kept together) | Platform conventions (39), built |
| 22 | Shortcuts per view, "in other views" listed | The scope from the action, no other views | One view (37) |

Everything else is darktable's: every control an action, fallbacks off by default, multiple presses, speed and its adjustments, slider precision, instances from Phase 4, `<focused>`, visual mapping, the modal page on Windows and Linux, Import extras, a shortcut's step merged as darktable's undo merges, tooltips listing every shortcut, the toasts, and darktable's default keys as their features land, exposure's slider on E and crop on C first. **Rotation's defaults,** darktable's Alt+R, Alt+[ and Alt+], follow deviations 10 and 21: Alt with a letter opens the menus on Windows and Linux, so there the three are Ctrl+R, Ctrl+[ and Ctrl+], kept together and held by no built default, and on the Mac Option+R, Option+[ and Option+], darktable's; the audit confirms them. **The default keyboard map is audited** against darktable's 247 default registrations before Phase 3's defaults land: each difference moves to darktable's key or becomes a listed deviation with its reason.

## Options considered

- **Flattening element × effect into action ids** (`control-<tool>-<control>-<effect>`), with no fallbacks, multiple presses, instances or visual mapping: simpler, but darktable's model with its words is what a darktable user and a device mapping expect, and nothing in Apertrust forces the cut.
- **darktable's label paths as ids,** or paths of stable ids with `/`: a relabelled control loses its shortcuts, and the second breaks every id the registry has.
- **darktable's `shortcutsrc` and its three companion files:** a later version's defaults never reach a person, decision 43 changes, four files and a migration from `[keys]`.
- **A `shortcuts.toml` of its own,** differences only: a second file and decision 43 changed, for an export that `[keys]`'s table gives already.
- **Every key acting at release,** as darktable's: every key equivalent would leave the Mac's menus.
- **An `enable` element,** on writing one control at its own value and off resetting the tool: a person never turns a tool on or off explicitly (32), and an explicit switch is a power button by another name.
- **Alt for a tenth in the value fallbacks:** darktable's users reach for Ctrl, and window managers on Linux desktops take Alt with the mouse.
- **Fallbacks on by default:** darktable keeps them off because they surprise; **no fallbacks:** a person who wants darktable's behaviour could not have it.
- **"Assign Shortcut…" in a tool's ⋯, a "Try it" field on the page, a built-in "darktable set":** none is darktable's; visual mapping is darktable's way to try a key, and with darktable's defaults as Apertrust's the set adds nothing beyond Import extras.
- **H for the accels window,** peek moved: peek is used constantly, the overview is a reference. **No accels window:** a darktable user's overview lost.
- **Map Shortcuts in the default Toolbar:** decision 92's default changes for a tool used rarely.
- **Input devices now, or on PortMidi and SDL2:** C libraries (5), and no demand yet.
- **Scripts calling actions through the engine:** the engine knows no actions; the protocol already reaches every control.
- **A crate for actions:** no FFI, dependency direction or plugin boundary asks for one.

## Consequences

**The steps,** each one pull request usable alone:

- **Phase 2:**
  1. The model: `Shortcut`, `Trigger`, `DefaultShortcut` (and `apertrust_tools::ShortcutDef` converted into it), `Kind`, `Effect`, `Action::owner` and `kind`, the built `Trigger::Hold` becoming the hold kind and `Repeat` a flag; the owned registry; `[keys]`' inline form, the load rule and `default = true`. Tests: every kind's lists against darktable's; a version-2 `settings.toml` of today read and written byte for byte; each form round-tripped; a person's override beating a default; two clashing overrides refused; with loading off, an assigned shortcut equal to its default written with `default = true` and there after a restart, and turning loading on or off writing nothing; the Mac's menu showing only plain-key, default-effect equivalents.
  2. The tools' actions: each tool's node (show and reset acting; presets, focus and instance listed), its tabs, controls (one action per control, acting on the row the current mode shows), preset rows, toggles, buttons, pickers and setting rows, a processing tool's effects run as session commands from the mirror, a utility's setting rows through the app's settings store (`set`, 93). Tests: one action per control, ids unique, words in every locale; a table of effects to commands per kind, and to the store for a utility's settings; a control in a row per mode acting on the current mode's; a control every row of which the mode hides not active; ten effects on one control within 10 s one step.
  3. Settings › Shortcuts: the tree, the grouped list, both searches, recording a key with its modifiers, editing element, effect and speed (the instance column shows the preferred instance alone, read-only, until instances come in Phase 4), Delete, the clash alerts, Restore…, Load default shortcuts at startup, the notice, and Help › Keyboard Shortcuts. Tests: kittest flows, snapshots in every theme, the AccessKit tree, Esc and right-click cancelling, the Mac's menu following a change.
  4. Tooltips and toasts. Tests: the tooltip per kind, the announcements.
- **Phase 3:**
  5. The dispatcher in `raw_input_hook`: release timing, multiple and long presses, clicks and the six moves with a key held, the input taken from widgets only while such a key is held; recording double, triple and long presses, and clicks and moves with a key held, on the Shortcuts page. Tests: synthetic input against an injected clock, darktable's cases: a key with a move shortcut acting at release and a move cancelling its plain action, a move or a click with no key held reaching the row under the pointer, the double-press wait only where one exists, the arrows repeating while no move uses them.
  6. Fallbacks, with a person's own on the page's fallbacks branch, recorded with no key, modifiers alone with or without a click or a move, through step 5's recording; speed adjustments and slider precision.
  7. Visual mapping, with Help › Map Shortcuts and its Toolbar item.
  8. The files: Export… and Import… by device, Import extras, importing a darktable `shortcutsrc`. Tests: every line of darktable's sample files mapped for the ported tools, the rest listed.
  9. darktable's defaults: the keymap audit; the tools' defaults (exposure's E, crop's C, rotation's three) as each lands; `<focused>` with the focused tool; the accels window and Help › Shortcuts Overview.
- **Phase 4:** instances, darktable's preferred-instance settings mapped onto layers; `<blending>` as layers' and masks' actions; mapping mode's instance.
- **Later, on demand:** MIDI and gamepads, `modifiers`, "reinitialise input devices" and `all_actions`; scripting's `action` and `mimic`, and copying an action's call.

**Elsewhere:**

- `settings.toml`'s `[keys]` (43) holds inline tables beside strings: the schema's `keys` list takes items of any TOML value, which the store keeps as written and the app parses; no version changes. The settings `shortcuts.load_defaults`, `shortcuts.fallbacks`, `shortcuts.hide_notice` and `shortcuts.slider_precision` join the schema with their steps ([settings.md](../settings.md#the-settings-dialog)).
- Decision 42's fine step and the value fallbacks are one table; a change to one is a change to both.
- The menus' rule that a hold action is never a menu item ([gui.md](../gui.md#who-owns-the-keys), under 39) narrows: the overview's item runs its toggle effect; H's peek stays in no menu. A menu item shows a key equivalent only where its key acts at press.
- `[keys]`' load rule changes towards darktable's: a person's shortcut beats another action's clashing default, where it is refused today.
- Decisions 91 and 92 hold: items run their action's default element and effect, and Map Shortcuts is offered outside the Toolbar's default.
- Each port spec lists darktable's actions and default shortcuts for its module, and the table from darktable's paths to Apertrust's ids, so a custom widget's actions are part of the port.
- Still open, each settled in its step ([gui.md](../gui.md#open-questions)): the focused tool, a write per shortcut step, Ctrl+Shift's `force`, scripts' action call, and preset actions' ids.
