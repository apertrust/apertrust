# Locales

The interface's words, as [Fluent](https://projectfluent.org) messages, one folder per language (`docs/gui.md`, Translation). `src/i18n.rs` compiles them in.

| Folder | Language | Status |
| --- | --- | --- |
| `en-GB` | British English, the source | Written with the code |
| `en-US` | American English | An overlay of `en-GB`: only the messages it spells differently |
| `uk` | Ukrainian | Complete; a draft, not yet reviewed by a native speaker |
| `pl` | Polish | Complete; a draft, not yet reviewed by a native speaker |

American English is not a copy. Its `ui.ftl` and `modules.ftl` hold only the messages whose words American English spells differently (colour → color, grey → gray, centre → center, -ise → -ize, cancelled → canceled, licence → license), with en-GB's ids, variables, selectors and access keys; every other message is en-GB's. Nothing is reworded but the one word American English says differently, anticlockwise → counterclockwise, and proper names, file formats and darktable's identifiers stay. darktable's module names are American upstream, so where en-GB writes one in British spelling, en-US writes darktable's: Color balance RGB. The tests fail on an en-US message en-GB lacks, on one that copies en-GB's, on one whose variables differ, and on an en-GB message saying colour, grey or centre that en-US leaves so.

The pseudo-locale (`en-XA`) has no folder: it is English, accented, 40% longer and in brackets, made at run time to show clipped text and strings that escaped translation.

## Adding a message

- Write it in `en-GB/ui.ftl` under its group, with an id that says where it appears: `action-…`, `tab-…`, `section-…`, `slider-…`, `unit-…`, `error-…`.
- A module's words go in `en-GB/modules.ftl`, with the ids its definition in `apertrust-modules` implies (decision 28): `module-<module>-title`, `-auto`, `-tab-<tab>`, `-group-<group>`, `-collapsible-<group>`, `module-<module>-control-<control>-label`, `-full`, `-promise` and `-option-<option>`, for a row with words of its own in one of darktable's modes `module-<module>-control-<control>-as-<words>-label`, `-full` and `-promise`, for one of darktable's own settings for a tool, which no edit stores, `module-<module>-setting-<setting>-label`, `-full`, `-promise` and `-option-<option>`, and for a preset row `module-<module>-preset-<preset>-label`, `-full`, `-promise` and `-option-<option>`, its Custom being every module's `preset-custom` in `ui.ftl`. A test in `crates/modules/tests` fails on one missing.
- Translate it in `uk` and `pl` in the same change, with the same variables, and add it to `en-US` only if American English spells one of its words differently. The tests fail on a message missing from a language, on a translation English no longer has, and on a translation whose variables differ from English's.
- Let the language choose its own forms: a count is a variable with a selector (`{ $count -> [one] … *[other] … }`), and Ukrainian and Polish add `few` and `many`. Never build a sentence from pieces in code.
- Numbers passed as variables are written in the language's format (`1,5` in Ukrainian and Polish) and pick its plural form.

## The menus' access keys

The top-level menus' names (`menu-file`, `menu-edit` and the rest) carry an access key: the `&` marks the letter after it, which Alt opens the menu at on Windows and Linux (`docs/gui.md`, The menu bar, Translation; decision 39). The Mac shows no `&`.

- Each top-level menu has one, and no two share a letter within a language, Window's included, though only the Mac has that menu.
- Choose a letter typed without AltGr: in Polish never ą, ć, ę, ł, ń, ó, ś, ź or ż.
- Write `&&` for an ampersand that is shown.
- Where the first letter is taken, mark another: Polish Help is `Po&moc`, since Plik has P.
- Only the top-level menus have one; the items inside need none.
- File's word is also a term, `-file-menu`, and `menu-file` is `&{ -file-menu }`, so a sentence that names the menu uses the term and shows no `&`.

A test fails when a menu lacks an access key, when two share one in a language, or when one needs AltGr.

## Adding a language

A new folder named by its language tag, every message translated, the language added to `Locale` in `src/i18n.rs`, and its alphabet checked against the font (a test draws every message in every language). Right-to-left languages wait for bidirectional layout in egui.
