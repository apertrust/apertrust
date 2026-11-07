# Port spec: darktable's variables

## Source

darktable at the reference release, commit 20891f6c6f. darktable's code is the reference, checked against darktable-cli itself where it was probed (Results).

| Part | Files and functions |
| --- | --- |
| The variables | `src/common/variables.c` (`_init_expansion()`, `_get_base_value()`, `_variable_get_value()`, `_expand_source()`, `_legacy_aliases()`, `_get_parameters_n_m()`, `_get_var_parameter()`, `_variables_get_longitude()`, `_variables_get_latitude()`, `dt_variables_expand()`) and `src/common/variables.h` |
| Their list | `src/gui/gtkentry.c` (`_default_path_compl_list`, `_init_completion_model()`, `dt_gtkentry_variables_add_metadata()`) |
| Their use | `src/imageio/storage/disk.c` (`gui_init()`'s path field, `store()`) |
| Their helpers | `src/common/utility.c` (`dt_util_format_exposure()`, `dt_util_str_replace()`, `dt_util_str_occurence()`, `dt_util_gps_rationale_to_number()`, `dt_util_gps_elevation_to_number()`); `src/common/image.c` (`dt_image_print_exif()`, `dt_image_film_roll_name_levels()`, `dt_image_init()`); `src/common/datetime.c` (`dt_datetime_img_to_gdatetime()`, `dt_datetime_exif_to_numbers()`, `dt_datetime_add_subsec_to_exif()`, `dt_datetime_gdatetime_to_exif()`); `src/common/metadata.c` and `src/common/database.c` (the default metadata fields) |
| What the image holds | `src/common/exif.cc` (`_exif_decode_exif_data()`, `_find_datetime_taken()`, `_exif_value_str()`, `_strlcpy_to_utf8()`) |
| darktable's own tests | `src/tests/variables.c` |

## Status

**Ported**, for the Export dialog (decision 56; `docs/gui.md`, The Export dialog):

- `apertrust-export`'s `variables` (`crates/export/src/variables/`);
- `apertrust-raw`'s `read_exif_file` and `read_exif` (`crates/raw/src/exif.rs`).

Their users, built apart from this port:

- File on disk, which expands the path pattern for each photo: `apertrust-export`'s `disk`, with the values filled in `apertrust-cli`'s `names.rs`;
- the dialog's `$(` list, which takes [`variables::list`](#the-list).

No kernel: this is no darktable module a person edits, but darktable's text expansion of an export's path, ported as its size is (`docs/ports/finalscale.md`).

## Purpose

An export's path is a pattern, `$(FILE_FOLDER)/apertrust_exported/$(FILE_NAME)` by default (D6): text and variables, which each photo expands to its own file name.

darktable's variables name the photo's file and folder, its shooting data, the time, the export's sizes and the person's folders. Bash-like operations cut and change them: `$(FILE_NAME/IMG_/)`, `$(EXIF.YEAR)/$(EXIF.MONTH)`, `$(SEQUENCE[3,1])`, `$(TITLE-untitled)`.

Patterns written for darktable keep working (D7), and a pattern darktable would expand wrongly stops the export at the field instead (D8).

## Where darktable expands them

- **File on disk** (`disk.c`, `store()`): `dt_variables_expand(vp, pattern, TRUE)` for each photo, with
  - the job code "export";
  - the image's full path as `filename`;
  - the photo's number in the export, from 1, as `sequence`;
  - the format's largest size and Allow upscaling (`dt_variables_set_max_width_height()`, `dt_variables_set_upscale()`) as the export's.

  The time is taken once, as the storage's parameters are made (`dt_variables_params_init()`: `g_date_time_new_now_local()`), so every photo of one export has the same.

  Before it expands: with more than one photo and no `$` in the pattern, `_$(SEQUENCE)` is added, and a leading `~` is the home folder (`dt_util_fix_path()`). After it, a result ending in a separator takes `$(FILE_NAME)` after the original pattern. Apertrust's File on disk (`apertrust-export`'s `disk`) does these.
- darktable's image information, overlays and watermarks expand with the job code "infos", which a few variables take ([Variables](#the-variables)); the import with its own. Apertrust expands for exports only.

## The grammar

As `_expand_source()`, `_variable_get_value()` and `_get_base_value()` read a pattern, darktable reading as it expands and Apertrust reading it whole first:

- **Text** is copied as it is, up to a stop: the end at the top, `)` inside an operation's text, `/` inside a substitution's pattern. A backslash takes the character after it as it is, the stops and `$` among them (`\)`, `\/`, `\$`); a backslash at the end is itself. `$(` starts a variable; a `$` before anything else is text.
- **A name** follows `$(`. darktable tests its names in its order (`_has_prefix()`, which also moves past the name) and takes the first the text starts with. No name starts with one before it in that order, so that is the longest; Apertrust takes the longest (`names::NAMES`, held by a test). Names are case-sensitive, and are letters, digits, `.` and `_`.
- **Parameters** follow some names:
  - `SEQUENCE`: one digit, the older form, `$(SEQUENCE4)`, its value the digit's (`& 0b1111`); or `[n,m]`, `n` the digits and `m` the start, each read by `g_ascii_strtoll()` and taken where above 0, the digits into a byte;
  - `ROLL.NAME`, `IMAGE.ID`, `ID` and `IMAGE.ID.NEXT`: `[n]` (`_get_var_parameter()`), above 0 into a byte, then for ROLL.NAME clamped to 1 to 5;
  - `CATEGORY`: one digit and `(category)`, the older form, or `[n,category]`.

  `_get_parameters_n_m()` takes the text up to the first `]` and splits it at its first `,`. `[` followed by `,` takes nothing more, so the `,` is then the operation (darktable's own list inserts `CATEGORY[,]`, which is so until `n` is filled in). `[]` sets nothing.
- **One operation** may follow, then `)`. darktable takes any other character as an operation it does not have and drops it.

| Operation | Meaning | darktable's rule |
| --- | --- | --- |
| `$(VAR-text)` | `text` where the value is empty | `text` expanded up to `)` |
| `$(VAR+text)` | `text` where the value is not empty | as `-` |
| `$(VAR:offset)`, `$(VAR:offset:length)` | characters from `offset`, at most `length` | each read by `strtol()`, so spaces, a sign and digits, 0 where none, the length then into an `int`, so 4294967296 is 0 (darktable-cli writes `ab` for `a$(FILE_NAME:0:4294967296)b`); in UTF-8 characters; a negative `offset` from the end, a negative `length` a place from the end |
| `$(VAR#text)` | the value without `text` at its start | a plain comparison, as every pattern here |
| `$(VAR%text)` | the value without `text` at its end | |
| `$(VAR/pattern/replacement)` | the first `pattern` replaced | `pattern` expanded up to `/`, so it may hold `)`; `replacement` up to `)` |
| `$(VAR//pattern/replacement)` | every `pattern` | `dt_util_str_replace()` |
| `$(VAR/#pattern/replacement)` | `pattern` at the start | |
| `$(VAR/%pattern/replacement)` | `pattern` at the end | |
| `$(VAR^)`, `$(VAR^^)` | the first character, or every one, in upper case | `g_unichar_toupper()` of the first, `g_utf8_strup()` of all |
| `$(VAR,)`, `$(VAR,,)` | in lower case | `g_unichar_tolower()`, `g_utf8_strdown()` |

Every operation's text may hold variables, which expand first (`$(FILE_FOLDER/test/$(SEQUENCE//0/o))`).

- **darktable's legacy aliases** (`_legacy_aliases()`): before it expands, darktable replaces `$(TITLE)`, `$(DESCRIPTION)`, `$(PUBLISHER)`, `$(CREATOR)` and `$(RIGHTS)` whole by `$(Xmp.dc.title)` and the others.
- **darktable's error case:** where no `)` comes where a variable should end, `_variable_get_value()` gives up, and `_expand_source()` writes the `$` as text and reads on from the `(`. So `$(FILE_NAME` stays as it is and `foo$(bar$(SEQUENCE)baz` gives `foo$(bar0023baz`. A name it does not know expands to nothing, the letters after it skipped (`while(g_ascii_isalpha(**variable))`).

Apertrust reads the same grammar into parts (`variables::parse`) and expands them later (`variables::expand`), so a pattern's errors are known before any photo is exported (D8, below).

## The variables

Every name `_get_base_value()` knows, with its legacy aliases, in its order; what each expands to; and where Apertrust's value comes from ([`Values`](#values), filled by the export's File on disk).

Formats are C's, in the C locale ([Deliberate differences](#deliberate-differences)). "Nothing here" is D7: it expands to nothing, and `Pattern::not_here` says why, for the dialog's line.

| Name and aliases | darktable's value | Apertrust's value |
| --- | --- | --- |
| `YEAR.SHORT`, `SHORT_YEAR`, `DATE.SHORT_YEAR` | the export's time, `%y` | the export's start, local time |
| `YEAR`, `DATE.LONG_YEAR` | `%Y`, the year unpadded | the same |
| `MONTH.SHORT`, `DATE.SHORT_MONTH`; `MONTH.LONG`, `DATE.LONG_MONTH` | `%b`, `%B`, the locale's month names | the C locale's, Jan, January |
| `MONTH`, `DATE.MONTH`; `DAY`, `DATE.DAY` | `%m`, `%d` | the same |
| `HOUR.AMPM`, `DATE.HOUR_AMPM` | `%I %p`, "02 PM" | the C locale's AM and PM |
| `HOUR`, `DATE.HOUR`; `MINUTE`, `DATE.MINUTE`; `SECOND`, `DATE.SECOND` | `%H`, `%M`, `%S` | the same |
| `MSEC` | `%f` cut to three digits | the same |
| `DATE` | `%Y:%m:%d %H:%M:%S`, with `.%03d` milliseconds only under `lighttable/ui/milliseconds`, off by default | without, darktable's default |
| `EXIF.DATE.REGIONAL`, `EXIF.TIME.REGIONAL` | the photo's time, `%x` and `%X` | the C locale's `%m/%d/%y` and `%H:%M:%S` |
| `EXIF.YEAR.SHORT` … `EXIF.SECOND`, `EXIF.DATE`, with `EXIF.DATE.*` and `EXIF_*` aliases | the photo's time, as the export's | from DateTimeOriginal ([EXIF](#the-exif-fields)) |
| `EXIF.MSEC`, `EXIF_MSEC` | always `000` | the same: darktable rebuilds the image's time from whole seconds (`dt_datetime_img_to_gdatetime()`), mirrored |
| `EXIF.ISO`, `EXIF_ISO` | `%d` of the ISO, truncated | `RawImage::iso` (decision 24), 0 where none |
| `NL` | a new line, in the image information only | nothing here, as in darktable's export |
| `EXIF.EXPOSURE.BIAS`, `EXIF_EXPOSURE_BIAS` | `%+.2f`, nothing where the file records none | `RawImage::exposure_bias` |
| `EXIF.EXPOSURE.PROGRAM` | Exiv2's name of ExposureProgram, nothing for 0 | Exiv2 0.28's names restated |
| `EXIF.EXPOSURE`, `EXIF_EXPOSURE` | `dt_util_format_exposure()`: `1/250`, `1/2.5`, `2″`, `0.7″`, its `/` written `_` but in the image information | the same, in f32 as darktable |
| `EXIF.APERTURE`, `EXIF_APERTURE` | `%.1f` | the same |
| `EXIF.CROP_FACTOR` | `%.1f` of the crop factor, from FocalLengthIn35mmFilm or the focal plane's resolution, 1.0 by default | nothing here: rawler reads neither tag |
| `EXIF.FOCAL.LENGTH.EQUIV` | the focal length times the crop factor | nothing here, as above |
| `EXIF.FOCAL.LENGTH`, `EXIF_FOCAL_LENGTH` | `%.1f` | the same |
| `EXIF.FOCUS.DISTANCE`, `EXIF_FOCUS_DISTANCE` | `%.2f`, 0 where unknown | SubjectDistance ([EXIF](#the-exif-fields)) |
| `EXIF.FLASH.ICON`, `EXIF.FLASH` | ⚡ where Flash's bit 0 is set; yes, no, or "no info" without the tag | the same, in English |
| `EXIF.METERING`, `EXIF.WHITEBALANCE` | Exiv2's names of MeteringMode, nothing for 0, and of the white balance | Exiv2 0.28's names restated |
| `LONGITUDE`, `GPS.LONGITUDE`; `LATITUDE`, `GPS.LATITUDE` | `E%010.6f` or W, `N%09.6f` or S; in the image information, prettier with `pretty_location` | the export's form |
| `ELEVATION`, `GPS.ELEVATION` | `%.2f`, so `nan` without a location | the same, `nan` mirrored |
| `GPS.LOCATION.ICON`, `GPS.LOCATION` | 📌 with a latitude and longitude; those known of the three, joined by ", " | the same |
| `EXIF.MAKER`, `MAKER`; `EXIF.MODEL`, `MODEL` | `camera_maker`, `camera_alias` | `RawImage::camera`'s `make` and `alias` (decision 24) |
| `EXIF.LENS`, `LENS` | `exif_lens` | [EXIF](#the-exif-fields) |
| `IMAGE.ID.NEXT`, `ID`, `IMAGE.ID` | the library's ids, `%0*u` with `[n]` digits | nothing here: no library yet (D7) |
| `IMAGE.EXIF` | `dt_image_print_exif()`: "1/250 f/8.0 18mm ISO 500", its `/` kept | the same |
| `VERSION.NAME`, `VERSION_NAME`, `VERSION.IF_MULTI`, `VERSION_IF_MULTI`, `VERSION` | the duplicate's version and name | nothing here: no duplicates yet (D7) |
| `JOBCODE` | the job code, "export" | the same |
| `ROLL.NAME`, `ROLL_NAME` | the folder's last `[n]` parts, 1 by default | the same |
| `FILE.DIRECTORY`, `FILE_DIRECTORY`, `FILE.FOLDER`, `FILE_FOLDER` | `g_path_get_dirname()` of the path | the same |
| `IMAGE.FILENAME` | the path | the same |
| `FILE.NAME`, `FILE_NAME`, `IMAGE.BASENAME` | `g_path_get_basename()` to its last `.` | the same |
| `FILE.EXTENSION`, `FILE_EXTENSION` | after the path's last `.` | the same, from the whole path as darktable finds it |
| `SEQUENCE` | `%.*u` of the photo's number plus the start less 1, 4 digits and a start of 1 by default | the same |
| `USERNAME` | `g_get_user_name()` | the person's login name |
| `FOLDER.HOME`, `HOME_FOLDER`, `HOME`; `FOLDER.PICTURES`, `PICTURES_FOLDER`; `FOLDER.DESKTOP`, `DESKTOP_FOLDER`, `DESKTOP` | the home folder; Pictures, or home's `Pictures`; the desktop, nothing where the system has none | the same, which the export fills |
| `STARS`, `RATING.ICONS`, `RATING_ICONS`, `Xmp.xmp.Rating` | the star rating, -1 rejected, and as ★ | nothing here: no ratings yet (D7) |
| `LABELS.ICONS`, `LABELS_ICONS`, `LABELS.COLORICONS`, `LABELS_COLORICONS` | colour labels as coloured dots, in the image information only | nothing here, as in darktable's export |
| `LABELS` | colour labels as text | nothing here: no labels yet (D7) |
| `OPENCL.ACTIVATED`, `OPENCL_ACTIVATED` | yes or no | nothing here (D7) |
| `WORKSPACE.LABEL` | darktable's workspace | nothing here (D7) |
| `WIDTH.MAX`, `MAX_WIDTH`; `HEIGHT.MAX`, `MAX_HEIGHT` | the export's largest size as the storage is given it | the same, which the export fills |
| `WIDTH.SENSOR`, `SENSOR_WIDTH`, and its height | the raw's whole readout | the same |
| `WIDTH.RAW`, `RAW_WIDTH`, and its height | after raw preparation's crop (`p_width`) | the same |
| `WIDTH.CROP`, `CROP_WIDTH`, and its height | after every geometry (`final_width`) | the same |
| `WIDTH.EXPORT`, `EXPORT_WIDTH`, and its height | the crop scaled in float to fit the largest size, where that shrinks it or upscaling is on, `roundf`, which can differ from the file by one (decision 56) | the same |
| `CATEGORY` | the subtag at level `n` under `category` | nothing here: no tags yet (D7) |
| `IMAGE.TAGS.HIERARCHY`, `TAGS`, `IMAGE.TAGS` | the tags | nothing here (D7) |
| `SIDECAR_TXT` | the `.txt` sidecar, in the image information only | nothing here, as in darktable's export |
| `DARKTABLE.VERSION`, `DARKTABLE_VERSION`; `DARKTABLE.NAME`, `DARKTABLE_NAME` | darktable's version and name | Apertrust's (D7) |
| `APERTRUST.VERSION`, `APERTRUST.NAME` | none | Apertrust's version, as the banner shows it, and "Apertrust" (D7) |
| `Xmp.dc.creator`, `CREATOR` | the creator, which the import takes from Artist | Artist ([EXIF](#the-exif-fields)) |
| `Xmp.dc.rights`, `RIGHTS` | the rights, from Copyright | Copyright |
| `Xmp.dc.description`, `DESCRIPTION` | the description, from UserComment or ImageDescription | an ASCII UserComment, the one rawler reads |
| `Xmp.dc.title`, `TITLE`; `Xmp.dc.publisher`, `PUBLISHER`; `Xmp.acdsee.notes`; `Xmp.darktable.version_name`; `Xmp.darktable.image_id`; `Xmp.xmpMM.PreservedFileName` | the photo's metadata | nothing here: no metadata yet (D7) |

The metadata names are darktable's nine default fields (`data.meta_data`, made by `database.c`'s schema version 13), which `_get_base_value()` tests last, in their display order. darktable's person can add fields of their own (`dt_metadata_add_metadata()`); Apertrust knows only the nine.

## Values

`variables::Values` holds what `_init_expansion()` gathers, for one photo of an export.

Its EXIF values hold what darktable's image holds when the file records none (`dt_image_init()`):

- 0;
- `None` where darktable keeps the case apart (the bias's `DT_EXIF_TAG_UNINITIALIZED`, Flash's -1, the location's NaN);
- for a photo with no valid DateTimeOriginal, darktable's origin, 0001-01-01 00:00:00.

The export fills it (`apertrust-cli export` does, `crates/cli/src/names.rs`): the path, the sequence, the start time, the largest size and upscaling, the version, the person's folders, `RawImage`'s camera, ISO and bias, `read_exif_file`'s fields, and the sizes.

## The EXIF fields

`apertrust_raw::read_exif_file(path)` and `read_exif(bytes)` read a raw's shooting data through rawler's metadata, without decoding its image, so the dialog can name photos it has not decoded (`docs/engine.md`, Export jobs, `PreviewExport`). The camera's names, the ISO and the bias stay `RawImage`'s own.

| `Exif` field | darktable reads, through Exiv2 | rawler 0.8.0 reaches |
| --- | --- | --- |
| `taken` | Exif.Image or Exif.Photo DateTimeOriginal, Exiv2's size at least 19 with its NUL, its first 19 characters copied over darktable's origin's text, with SubSecTimeOriginal's first three characters, trimmed, as a fraction; validated by glib's ISO 8601 parser (`_find_datetime_taken()`, `dt_datetime_exif_to_numbers()`) | DateTimeOriginal and SubSecTimeOriginal, read so: a year 1 to 9999, a real day, hours to 23, minutes and seconds to 59, the fraction digits only; all 19 characters, where darktable takes 18 and the origin's last digit |
| `exposure_time` | `Exiv2::exposureTime()`, ExposureTime first; else 2^−ShutterSpeedValue | ExposureTime, else ShutterSpeedValue |
| `aperture` | `Exiv2::fNumber()`, FNumber first; else 2^(ApertureValue / 2) | FNumber, else ApertureValue |
| `focal_length` | `Exiv2::focalLength()`, FocalLength first | FocalLength |
| `focus_distance` | Nikon's lens data, Olympus's and Canon's maker notes, then `Exiv2::subjectDistance()`, then Sony's | SubjectDistance only |
| `exposure_program`, `metering_mode`, `flash` | Exif.Photo's tags | the same tags |
| `white_balance` | `Exiv2::whiteBalance()`: a maker note's first (Canon, Fujifilm, Nikon, Olympus, Panasonic, Sony and others), Exif.Photo.WhiteBalance last | Exif.Photo.WhiteBalance only |
| `lens` | Canon's LensType or LensModel, Pentax's, Panasonic's, Olympus's, Nikon's lens id, then `Exiv2::lensName()`, LensModel as the last resort; then RF and RF-S named in full, NIKKOR, TAMRON and VILTROX capitalised, "Sigma " added with LensMake SIGMA | rawler's lens data, from the maker notes it reads, else LensModel; darktable's changes to the name restated |
| `latitude`, `longitude`, `elevation` | GPSLatitude and GPSLongitude with their refs, degrees + minutes / 60 + seconds / 3600 in double, negative S and W, 0/0 seconds read as 0; GPSAltitude negative unless its ref writes "0" | the same tags |
| `artist`, `copyright`, `user_comment` | Artist (else Canon's OwnerName), Copyright, UserComment's comment (else ImageDescription), which the import takes as the creator, rights and description | Artist, Copyright, and UserComment where it is written as ASCII, as rawler reads it |

Every Exiv2 rational with a denominator of 0 reads as 0, as Exiv2's `toFloat()` reads it. The corpus shows how far rawler reaches against darktable's own reading ([Results](#results)).

## D8: what stops a pattern

`Pattern::parse` refuses, at the field and before any photo is exported, where darktable expands to nothing or reads past (`variables::PatternError`, each with its place in the text):

- **An unknown variable** (`Unknown`):
  - no name the text after `$(` starts with;
  - a name with more name characters after it that make no name (`$(FILE_NAMEX)`, where darktable takes FILE_NAME and drops the X);
  - a name in the wrong case (`$(file_name)`).

  It carries the nearest name, for "Did you mean $(FILE_NAME)?" and the dialog's Use button: the list's names, then their aliases, ignoring case, within an edit distance of 1 up to four characters, 2 up to eight and 3 beyond, the first of the nearest. `$()` and a `$(` at the end are a name that is missing.
- **A variable not closed** (`Unclosed`), where darktable's error case writes the text as it is:
  - no `)` where it ends, at the end or before other text (`$(FILE_NAME:5x)`);
  - no `/` after a substitution's pattern (`$(FILE_FOLDER/home)`, whose pattern runs to the end, as darktable reads past its end there);
  - no `]` before the variable's `)` (`$(SEQUENCE[3)`, where darktable reads the `3` as an operation and drops it).
- **A character that is no operation** (`NoOperation`) after a name and its parameters (`$(FILE_NAME=)`), which darktable drops.
- **Variables nested more than 64 deep** (`TooDeep`, `MAX_DEPTH`), so that no pattern reads, expands or drops deeper than the stack takes: 30,000 deep would overflow it. darktable recurses as deep as its own stack lets it.

Two of these go beyond D8's own two errors, an unknown variable and a missing `)`: a name with more name characters after it, and a character that is no operation. Planned: both read the pattern on as darktable reads it, with a warning in the Export dialog's feedback line rather than a refusal.

The name or variable a message shows is cut at 64 characters with an ellipsis, and a typed name is only compared with names whose length is within the suggestion's reach of its own, so a megabyte's name is refused at once.

An empty path is File on disk's to refuse (`disk::Error::Empty`).

## D7: Apertrust's names

- `$(DARKTABLE.NAME)` and `$(DARKTABLE.VERSION)` read as Apertrust's, "Apertrust" and its version; `$(APERTRUST.NAME)` and `$(APERTRUST.VERSION)` are added.
- **Nothing here:** a variable darktable has and Apertrust cannot fill expands to nothing, and the export goes on. `Pattern::not_here` lists each such variable the pattern holds, once, with why (`NotHere`), for the dialog's line. They are:
  - OPENCL.ACTIVATED, WORKSPACE.LABEL, IMAGE.ID and the VERSION family, which D7 names;
  - the ratings, colour labels, tags and the metadata no EXIF tag seeds, until the library and Metadata bring them;
  - the crop factor and its equivalent focal length, which rawler cannot read;
  - `NL`, `LABELS.ICONS` and `SIDECAR_TXT`, which darktable expands only in its image information and so to nothing in its export too.

## The list

`variables::list()` is darktable's list, which its path field offers after "$(" (`_default_path_compl_list`), in its order. Each entry has:

- what it inserts (`varname`: `SEQUENCE[4,1]`, `ROLL.NAME[]`, `CATEGORY[,]`);
- darktable's description in English as its list shows it ("$(SEQUENCE[n,m]) - sequence number, n: number of digits, m: start number");
- why it has no value here, where it has none.

darktable's own "darktable name" and "current darktable version" are listed as `APERTRUST.NAME`, "Apertrust name", and `APERTRUST.VERSION`, "current Apertrust version".

Then come the default metadata fields that are not internal, in their display order, each "from metadata" (`dt_gtkentry_variables_add_metadata()`): eight, darktable's image id being internal. 94 in all. The descriptions' translations for the dialog are planned.

## Deliberate differences

- **D8's refusals**, above, where darktable expands an unknown variable to nothing, leaves a variable it cannot close as text, or drops a character it does not know. darktable's own tests expand some such patterns; restated, they are refused ([Test plan](#test-plan)). Two of them are planned to become warnings ([D8](#d8-what-stops-a-pattern)).
- **The legacy names are names of their own:** `TITLE`, `DESCRIPTION`, `PUBLISHER`, `CREATOR` and `RIGHTS` take operations, so `$(TITLE-untitled)` reads the title, where darktable replaces only `$(TITLE)` whole and takes `$(TITLE-untitled)` as an unknown name, always "untitled". With no title yet, both give "untitled", as darktable's own recursive tests expect. An escaped `\$(TITLE)` gives `$(TITLE)`, where darktable's gives `$(Xmp.dc.title)`.
- **Only darktable's nine metadata fields:** a field the person added in darktable is an unknown variable.
- **The C locale:** month names, AM and PM, and `%x` and `%X` as the C locale writes them ("Jul", "07/20/17"), where darktable takes the system's locale. darktable also translates "yes", "no" and "no info". Planned: the names follow the interface language, through CLDR's data with ICU4X, as the interface's numbers do.
- **No milliseconds in `$(DATE)`:** darktable adds them under its `lighttable/ui/milliseconds`, off by default; Apertrust has no such setting.
- **Sizes always known:** `$(WIDTH.RAW)`, `$(WIDTH.CROP)` and `$(WIDTH.EXPORT)` and their heights are the photo's own, where darktable's come from its image's `p_width` and `final_width`, which its darkroom fills: darktable-cli writes `0x0` for a photo it has just imported (Results).
- **Substitutions darktable cannot make:**
  - a global substitution's pattern that expands to nothing changes nothing, where darktable reads past its text;
  - two matches that overlap (`$(VAR//aa/b)` on "aaa") are replaced left to right, as bash replaces them, where darktable's `dt_util_str_replace()` fails;
  - a suffix longer than the value matches nothing, where darktable reads before its start.
- **A name stops at its name characters:** after a name the parser wants an operation, a parameter or `)`. darktable's `$(NLFILE_NAME)` in an export, which skips the infos-only NL and then reads FILE_NAME, is an unknown variable.
- **EXIF as far as rawler reaches** ([The EXIF fields](#the-exif-fields)): the focus distance, the white balance and the lens where darktable reads a maker note first, and the crop factor, which is nothing here.
- **The creator and rights from the EXIF:** darktable's import takes Artist and Copyright at its default sidecar mode, and darktable-cli, which turns sidecars off, takes neither. Apertrust follows darktable's import, so a comparison of names with darktable-cli differs there (Results).
- **Paths on Windows:** `\` and `/` both separate, as glib's. A drive's root and UNC paths are not given glib's special cases in `$(FILE.FOLDER)` yet, which matter only for a raw at a drive's root.

Kept as darktable has them, though odd (darktable-cli does each, Results):

- `$(EXIF.MSEC)` is always `000`;
- a photo with no EXIF time is dated 1-01-01;
- `$(GPS.ELEVATION)` writes `nan` without a location;
- an exposure time of 0 writes `1_inf`;
- `$(FILE.EXTENSION)` takes the last `.` of the whole path, so a file with no extension in a folder with a `.` takes the folder's;
- `$(IMAGE.EXIF)` keeps its `/`, which makes folders.

## Test plan

- darktable's own `src/tests/variables.c`, its six tests case by case, each with its file name, job code and sequence (`crates/export/tests/variables.rs`): every case darktable expands as it expects, expanded the same; its cases on `NONEXISTANT`, `$(FILE_NAME`, `$(FILE_FOLDER/home` and `foo$(bar…`, which darktable expands to nothing or leaves as text, refused with their errors.
- Every name, in darktable's order, the table held to `NAMES` whole, expanded for a photo with every value set; every one with no value here expanding to nothing and listed by `not_here`; a photo without EXIF data expanding as darktable's image without them (`crates/export/src/variables/tests.rs`).
- D8: unknown names with and without a suggestion, its place, wrong case, a name with more after it, inside an operation, no name; each `Unclosed` and `NoOperation`; the messages.
- Parameters: SEQUENCE's digits and start, its older form, 0 digits, text that is no number; ROLL.NAME's levels clamped; the others read and empty; every entry of the list read once filled.
- The helpers: `dt_util_format_exposure()`'s branches, glib's dirname and basename, ROLL.NAME's levels, the export size, `strtol()`; that no name starts with one before it; the suggestions (unit tests).
- In `apertrust-raw`: the time taken, its sub-second time and invalid times; GPS's degrees, sides and elevation; darktable's changes to lens names.

## Results

### Probes

darktable-cli at the reference release on the Apple M4, its disk storage expanding a pattern as the output name of a 16-pixel export, each `/` in a value replaced by `_` where it would make a folder:

- On a copy of the Pixel 4a's DNG with DateTimeOriginal and SubSecTimeOriginal removed by exiftool: `$(EXIF.YEAR)-$(EXIF.MONTH)-$(EXIF.DAY)` is `1-01-01`, `$(EXIF.DATE)` `1:01:01 00:00:00`, `$(EXIF.MSEC)` `000`, `$(EXIF.DATE.REGIONAL)` `01/01/01`, `$(EXIF.HOUR.AMPM)` `12 AM`.
- On the same file: `$(ROLL.NAME)` the folder, `$(ROLL.NAME[2])` its last two parts, `$(SEQUENCE)` `0001`, `$(SEQUENCE[2,5])` `05`, `$(SEQUENCE3)` `001`, `$(JOBCODE)` `export`, `$(LABELS.ICONS)`, `$(SIDECAR_TXT)` and `$(NL)` nothing, `$(IMAGE.EXIF)` `1/250 f/1.7 4mm ISO 55`, `$(GPS.LOCATION)` `N47.958509, E011.846224, 602.35`, `$(DARKTABLE.NAME)` `darktable`; `$(FILE_NAMEX)` and `$(FILE_NAME=)` the file's name, `$(NONEXISTANT-def)` `def`, as this spec reads darktable's code; and, darktable's library's own, `$(STARS)` 1 (its import's initial rating), `$(VERSION)` 0, `$(IMAGE.ID)` 1, `$(OPENCL.ACTIVATED)` `no`, `$(CATEGORY[0,darktable])` `issue,format` (its own tags).
- On the Nikon D5100: `$(EXIF.MSEC)` `000` where SubSecTimeOriginal is 50; `$(EXIF.EXPOSURE)` `1_50`; `$(WIDTH.RAW)x$(HEIGHT.RAW)`, the crop's and the export's `0x0`; `$(WIDTH.SENSOR)x$(HEIGHT.SENSOR)` `4992x3280`; `$(LENS)` `Nikon AF-S DX Zoom-Nikkor 18-70mm f/3.5-4.5G IF-ED`, Exiv2's name, where rawler's is the same without "Nikon "; `$(EXIF.WHITEBALANCE)` `AUTO`, Nikon's maker note, where EXIF's is Auto; `$(EXIF.FOCUS.DISTANCE)` `0.40`, from Nikon's lens data; `$(EXIF.CROP_FACTOR)` `1.5`.

### The corpus

The corpus (`crates/raw/tests/corpus/files.sha256`): darktable-cli expanding two patterns for each raw, against `Pattern::expand` with `Values` filled from `apertrust_raw::decode_file`'s camera, ISO and bias and `read_exif_file`'s fields, as File on disk fills them.

- **The first pattern:** `$(EXIF.YEAR)` to `$(EXIF.SECOND)`, `$(EXIF.MSEC)`, `$(EXIF.ISO)`, `$(EXIF.EXPOSURE)`, `$(EXIF.EXPOSURE.BIAS)`, `$(EXIF.EXPOSURE.PROGRAM)`, `$(EXIF.APERTURE)`, `$(EXIF.FOCAL.LENGTH)`, `$(EXIF.FOCUS.DISTANCE)`, `$(EXIF.FLASH)`, `$(EXIF.METERING)` and `$(EXIF.WHITEBALANCE)`.
- **The second:** `$(MAKER)`, `$(MODEL)`, `$(LENS)`, `$(GPS.LOCATION)`, `$(ELEVATION)`, `$(CREATOR)`, `$(RIGHTS)` and `$(IMAGE.EXIF)`.

darktable-cli exported no file for six of the 40 (the iPhone 12 Pro's DNG, the X-E4's lossy RAF, the Z 9's high efficiency NEF, the IIQ, the X3F and the 7R V's small ARW). rawler reads neither the GoPro's GPR nor the Hasselblad's 3FR, which Apertrust then cannot export. That leaves 32 raws both read.

| Variables | As darktable, of 32 | Where not |
| --- | --- | --- |
| The time taken, `$(EXIF.MSEC)` (`000`), the ISO, exposure, bias, exposure program, aperture, focal length, flash, metering mode, location, elevation (`nan` without one, as darktable), `$(MODEL)` and `$(IMAGE.EXIF)` | 32 each | |
| `$(MAKER)` | 31 | the OM-1 Mark II: rawler's clean maker, "OM Digital Solutions", where darktable's is "OM System"; `RawImage::camera`'s, where darktable's spelling is planned (decision 24), not this port's |
| `$(EXIF.WHITEBALANCE)` | 19 | 13 from maker notes, which Exiv2 prefers: Canon's "Auto (ambience priority)" and "Manual Temperature (Kelvin)", Nikon's "AUTO", "AUTO1" and "SUNNY", Olympus's, Sony's "Cloudy", and the Panasonics', which record no EXIF white balance |
| `$(EXIF.FOCUS.DISTANCE)` | 18 | 14 from Canon's, Nikon's, Olympus's and Sony's maker notes, where rawler's EXIF has no SubjectDistance, so 0.00 |
| `$(LENS)` | 16 | Exiv2's lens tables: the maker's name before Canon's, Nikon F's, Olympus's and Sigma's lenses ("Canon EF 50mm f/1.4 USM", rawler's "EF 50mm f/1.4 USM"), but not before Nikon Z's or Sony's; its "Unknown Lens (169)", "Unknown (0x0821)" (rawler names the Sigma) and the 5D Mark IV's "Sigma 24-70mm … *OR* Tamron SP 24-70mm …" (rawler's EXIF has "24-70mm"); Panasonic's and Pentax's lens types, which rawler does not read; Samsung's shorter name |
| `$(RIGHTS)`, `$(CREATOR)` | 20, 14 | darktable-cli writes nothing for any raw: it runs with `write_sidecar_files=never` (`src/cli/main.c`), under which `dt_metadata_set_import()` takes no EXIF metadata. darktable's import at its default, "on import", takes Artist and Copyright, which Apertrust follows |

Where darktable's value comes from a maker note, the gap is rawler's reach; none is closed by guessing at Exiv2's tables. The lens's name matters more to lens correction, whose Lensfun search darktable makes by it, than to a file's name, so the lens names are settled with the Lensfun port. A request to rawler for lens names from maker notes is to be filed upstream once Apertrust's repository is public.
