// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//! The packages' metadata in `packaging/` against the app: each platform's file types
//! hold every extension rawler decodes and no other, so a rawler update that adds one
//! fails here until the packages open it too, the Windows installer's template in Open
//! With alone; Apertrust's own type, the edit (`.apxmp`), is Apertrust's on macOS and
//! Windows, and never Apertrust Nightly's, and on Linux a MIME type both desktop entries
//! open; the app's id is the same everywhere,
//! Apertrust's and, as `packaging/nightly-identity.txt` makes the files for
//! `tools/package.sh --nightly`, Apertrust Nightly's; the `.deb` and the `.rpm` need the
//! same libraries; and the plist the binary embeds agrees with the bundle's
//! (docs/packaging.md).

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const APP_ID: &str = "org.apertrust.Apertrust";

/// The edit's type on macOS, which the bundle exports.
const EDIT_TYPE: &str = "org.apertrust.edit";
/// The edit's ProgID on Windows, Apertrust's, and Apertrust Nightly's.
const EDIT_PROGID: &str = "Apertrust.Edit";
const NIGHTLY_EDIT_PROGID: &str = "Apertrust.Nightly.Edit";
/// The edit's MIME type on Linux, which the MIME package declares.
const EDIT_MIME: &str = "application/x-apertrust-edit";

/// Each extension's type on macOS: the system's where it declares one (as `mdls` reports
/// for a file of that name), else the one `Info.plist` imports; and on Linux its MIME
/// type, shared-mime-info's where it names one, else ours, as `true` says.
const TYPES: [(&str, &str, &str, bool); 29] = [
    (
        "3fr",
        "com.hasselblad.3fr-raw-image",
        "image/x-hasselblad-3fr",
        false,
    ),
    (
        "ari",
        "org.apertrust.raw-image.ari",
        "image/x-arri-ari",
        true,
    ),
    ("arw", "com.sony.arw-raw-image", "image/x-sony-arw", false),
    ("cr2", "com.canon.cr2-raw-image", "image/x-canon-cr2", false),
    ("cr3", "com.canon.cr3-raw-image", "image/x-canon-cr3", false),
    (
        "crm",
        "org.apertrust.raw-image.crm",
        "image/x-canon-crm",
        true,
    ),
    ("crw", "com.canon.crw-raw-image", "image/x-canon-crw", false),
    ("dcr", "com.kodak.raw-image", "image/x-kodak-dcr", false),
    (
        "dcs",
        "org.apertrust.raw-image.dcs",
        "image/x-kodak-dcs",
        true,
    ),
    ("dng", "com.adobe.raw-image", "image/x-adobe-dng", false),
    ("erf", "com.epson.raw-image", "image/x-epson-erf", false),
    (
        "fff",
        "com.hasselblad.fff-raw-image",
        "image/x-hasselblad-fff",
        false,
    ),
    (
        "iiq",
        "com.phaseone.raw-image",
        "image/x-phaseone-iiq",
        false,
    ),
    (
        "kdc",
        "org.apertrust.raw-image.kdc",
        "image/x-kodak-kdc",
        false,
    ),
    (
        "mef",
        "org.apertrust.raw-image.mef",
        "image/x-mamiya-mef",
        false,
    ),
    (
        "mos",
        "com.leafamerica.raw-image",
        "image/x-leaf-mos",
        false,
    ),
    (
        "mrw",
        "com.konicaminolta.raw-image",
        "image/x-minolta-mrw",
        false,
    ),
    ("nef", "com.nikon.raw-image", "image/x-nikon-nef", false),
    ("nrw", "com.nikon.nrw-raw-image", "image/x-nikon-nrw", false),
    ("orf", "com.olympus.raw-image", "image/x-olympus-orf", false),
    ("ori", "com.olympus.raw-image", "image/x-olympus-ori", true),
    ("pef", "com.pentax.raw-image", "image/x-pentax-pef", false),
    (
        "qtk",
        "org.apertrust.raw-image.qtk",
        "image/x-apple-qtk",
        true,
    ),
    ("raf", "com.fuji.raw-image", "image/x-fuji-raf", false),
    (
        "raw",
        "com.panasonic.raw-image",
        "image/x-panasonic-rw",
        false,
    ),
    (
        "rw2",
        "com.panasonic.rw2-raw-image",
        "image/x-panasonic-rw2",
        false,
    ),
    (
        "rwl",
        "com.leica.rwl-raw-image",
        "image/x-panasonic-rw2",
        false,
    ),
    ("srw", "com.samsung.raw-image", "image/x-samsung-srw", false),
    (
        "x3f",
        "org.apertrust.raw-image.x3f",
        "image/x-sigma-x3f",
        false,
    ),
];

fn read(path: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path);
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    // A Windows checkout may end each line with CR LF; the tests match LF alone.
    text.replace("\r\n", "\n")
}

/// The extensions rawler decodes, lower case, as the packages name them.
fn decoded() -> BTreeSet<String> {
    apertrust_engine::session::raw_extensions()
        .iter()
        .map(|extension| extension.to_lowercase())
        .collect()
}

/// The texts between `open` and `close` in `text`, in order.
fn between<'a>(text: &'a str, open: &str, close: &str) -> Vec<&'a str> {
    text.split(open)
        .skip(1)
        .filter_map(|rest| rest.split_once(close).map(|(inside, _)| inside))
        .collect()
}

/// The strings of the plist array that follows `<key>key</key>`, every one under it.
fn plist_strings<'a>(plist: &'a str, key: &str) -> BTreeSet<&'a str> {
    let after = plist
        .split_once(&format!("<key>{key}</key>"))
        .unwrap_or_else(|| panic!("no {key}"))
        .1;
    let array = after.split_once("</array>").expect("an array").0;
    between(array, "<string>", "</string>")
        .into_iter()
        .collect()
}

#[test]
fn every_decoded_extension_has_its_types() {
    let listed: BTreeSet<String> = TYPES.iter().map(|t| t.0.to_owned()).collect();
    assert_eq!(
        listed,
        decoded(),
        "TYPES against apertrust_raw::extensions()"
    );
}

#[test]
fn the_bundle_opens_every_decoded_extension() {
    let plist = read("packaging/macos/Info.plist");
    let wanted: BTreeSet<&str> = TYPES.iter().map(|t| t.1).collect();
    assert_eq!(plist_strings(&plist, "LSItemContentTypes"), wanted);
    // A folder dropped on the Dock icon opens, at the rank None, so the app never becomes
    // a folder's default; besides, the edit (below), and nothing else is declared.
    let documents = documents(&plist);
    assert_eq!(documents.len(), 3, "the raws', the edit's and the folder's");
    let folder = documents[2];
    let pairs = between(folder, "<string>", "</string>");
    assert_eq!(pairs, ["Folder", "Viewer", "None", "public.folder"]);

    let imports = plist
        .split_once("<key>UTImportedTypeDeclarations</key>")
        .expect("the imported types")
        .1;
    let imported: BTreeSet<(&str, &str)> =
        between(imports, "<dict>\n\t\t\t<key>", "</dict>\n\t\t</dict>")
            .into_iter()
            .map(|declaration| {
                let strings = between(declaration, "<string>", "</string>");
                (strings[0], *strings.last().expect("an extension"))
            })
            .collect();
    let ours: BTreeSet<(&str, &str)> = TYPES
        .iter()
        .filter(|t| t.1.starts_with("org.apertrust."))
        .map(|t| (t.1, t.0))
        .collect();
    assert_eq!(imported, ours, "each imported type with its extension");
}

/// The bundle's document types, `CFBundleDocumentTypes`'s dictionaries, in order.
fn documents(plist: &str) -> Vec<&str> {
    let types = plist
        .split_once("<key>CFBundleDocumentTypes</key>")
        .and_then(|(_, rest)| rest.split_once("<key>UTExportedTypeDeclarations</key>"))
        .expect("the document types")
        .0;
    between(types, "\t\t<dict>", "\t\t</dict>")
}

/// The bundle owns the edit's type, which it exports: an XML file named `.apxmp`, whose
/// icon is the system's document page with the word APXMP, no file or badge of ours.
#[test]
fn the_bundle_owns_the_edit_and_draws_it_as_text() {
    let plist = read("packaging/macos/Info.plist");
    let edit = between(documents(&plist)[1], "<string>", "</string>");
    assert_eq!(edit, ["Apertrust edit", "Editor", "Owner", EDIT_TYPE]);

    let exported = plist
        .split_once("<key>UTExportedTypeDeclarations</key>")
        .and_then(|(_, rest)| rest.split_once("<key>UTImportedTypeDeclarations</key>"))
        .expect("the exported types")
        .0;
    let keys = between(exported, "<key>", "</key>");
    assert_eq!(
        keys,
        [
            "UTTypeIdentifier",
            "UTTypeDescription",
            "UTTypeConformsTo",
            "UTTypeIcons",
            "UTTypeIconText",
            "UTTypeTagSpecification",
            "public.filename-extension"
        ]
    );
    let strings = between(exported, "<string>", "</string>");
    assert_eq!(
        strings,
        [
            EDIT_TYPE,
            "Apertrust edit",
            "public.xml",
            "public.data",
            "APXMP",
            "apxmp"
        ]
    );
    for icon in ["IconFile", "IconName", "Badge"] {
        assert!(
            !plist.contains(icon),
            "{icon}: the system draws the edit's icon"
        );
    }
}

#[test]
fn linux_opens_every_decoded_extension() {
    let desktop = read("packaging/linux/org.apertrust.Apertrust.desktop");
    let line = desktop
        .lines()
        .find_map(|line| line.strip_prefix("MimeType="))
        .expect("a MimeType line");
    let listed: BTreeSet<&str> = line.split(';').filter(|t| !t.is_empty()).collect();
    let wanted: BTreeSet<&str> = TYPES.iter().map(|t| t.2).chain([EDIT_MIME]).collect();
    assert_eq!(listed, wanted, "every raw's type and the edit's");

    let mime = read("packaging/linux/org.apertrust.Apertrust.mime.xml");
    let declared: BTreeSet<(&str, &str)> = between(&mime, "<mime-type type=\"", "</mime-type>")
        .into_iter()
        .map(|declaration| {
            let kind = declaration.split_once('"').expect("a type").0;
            let glob = between(declaration, "<glob pattern=\"*.", "\"/>");
            (kind, glob[0])
        })
        .collect();
    let ours: BTreeSet<(&str, &str)> = TYPES
        .iter()
        .filter(|t| t.3)
        .map(|t| (t.2, t.0))
        .chain([(EDIT_MIME, "apxmp")])
        .collect();
    assert_eq!(
        declared, ours,
        "the MIME types shared-mime-info lacks, and the edit's"
    );
    let edit = between(
        &mime,
        &format!("<mime-type type=\"{EDIT_MIME}\">"),
        "</mime-type>",
    );
    assert!(
        edit[0].contains("<sub-class-of type=\"application/xml\"/>"),
        "the edit is an XML file: {}",
        edit[0]
    );
}

/// Open With and a double-click open the photo alone, as a terminal's `apertrust FILE`
/// does (`docs/packaging.md`, Opening a raw from the system): the desktop entry passes
/// every file chosen, alone, and the installer's ProgID and the app's entry the one
/// opened, never `--in-folder`, which the app now ignores.
#[test]
fn open_with_passes_the_files_alone() {
    let desktop = read("packaging/linux/org.apertrust.Apertrust.desktop");
    let exec: Vec<&str> = desktop
        .lines()
        .filter_map(|line| line.strip_prefix("Exec="))
        .collect();
    assert_eq!(exec, ["apertrust %F"]);
    let template = read("packaging/windows/installer.nsi");
    let opens: Vec<&str> = template
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("WriteRegStr") && line.contains(r"\shell\open\command"))
        .filter(|line| !line.contains("{{protocol}}"))
        .collect();
    assert_eq!(
        opens.len(),
        2,
        "the ProgID's and the app's entry's: {opens:?}"
    );
    for line in opens {
        let wanted = r#"$\"$INSTDIR\${MAINBINARYNAME}.exe$\" $\"%1$\""#;
        assert!(line.ends_with(&format!("\"\" \"{wanted}\"")), "{line}");
    }
    for (file, text) in [("desktop entry", &desktop), ("installer", &template)] {
        assert!(
            !text.contains("--in-folder"),
            "the {file} passes --in-folder"
        );
    }
}

/// Open With is offered for any number of files on Windows (decision 89): each open verb
/// that passes a file, the ProgIDs' and the app's entry's, has `MultiSelectModel` =
/// `Player`, past the 15 files Explorer's default model allows; each start then hands its
/// file to the running app. The uninstaller removes the keys that hold them.
#[test]
fn open_with_takes_any_number_of_files() {
    let template = read("packaging/windows/installer.nsi");
    let lines: Vec<&str> = template.lines().map(str::trim).collect();
    let verbs = |suffix: &str| -> BTreeSet<String> {
        lines
            .iter()
            .filter(|line| line.starts_with("WriteRegStr") && !line.contains("{{protocol}}"))
            .filter_map(|line| {
                let key = line.split('"').nth(1)?;
                let verb = key.strip_suffix(suffix)?;
                line.contains(suffix).then(|| verb.to_owned())
            })
            .collect()
    };
    let commands = verbs(r"\shell\open\command");
    assert_eq!(
        commands.len(),
        2,
        "the ProgID's and the app's entry's: {commands:?}"
    );
    let models = lines
        .iter()
        .filter(|line| line.ends_with(r#"\shell\open" "MultiSelectModel" "Player""#))
        .filter_map(|line| line.split('"').nth(1)?.strip_suffix(r"\shell\open"))
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    assert_eq!(models, commands);
    for removed in [
        r#"DeleteRegKey SHCTX "Software\Classes\\{{association.name}}""#,
        r#"DeleteRegKey SHCTX "${APPKEY}""#,
    ] {
        assert!(lines.contains(&removed), "{removed}");
    }
}

/// The extensions of the association named `name` in `config`, `Packager.toml`.
fn association(config: &toml::Table, name: &str) -> BTreeSet<String> {
    let associations = config["fileAssociations"].as_array().expect("associations");
    let association = associations
        .iter()
        .find(|association| association["name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("no association {name}"));
    let extensions = association["extensions"].as_array().expect("a list");
    extensions
        .iter()
        .map(|extension| extension.as_str().expect("text").to_owned())
        .collect()
}

/// The edit's extension alone.
fn apxmp() -> BTreeSet<String> {
    BTreeSet::from(["apxmp".to_owned()])
}

#[test]
fn the_installer_registers_every_decoded_extension_and_the_edit() {
    let config: toml::Table = read("packaging/Packager.toml").parse().expect("TOML");
    let associations = config["fileAssociations"].as_array().expect("associations");
    assert_eq!(associations.len(), 2, "the raws' and the edit's");
    assert_eq!(association(&config, "Apertrust.RawImage"), decoded());
    assert_eq!(association(&config, EDIT_PROGID), apxmp());
    let description = associations[1]["description"].as_str();
    assert_eq!(description, Some("Apertrust edit"));
}

/// The registry lines the installer's template writes in `section` (`Install` or
/// `Uninstall`) for the associations in `Packager.toml`, rendered as cargo-packager
/// renders its loop over each association's extensions.
fn installer_lines(template: &str, config: &toml::Table, section: &str) -> Vec<String> {
    let body = template
        .split_once(&format!("\nSection {section}\n"))
        .unwrap_or_else(|| panic!("no section {section}"))
        .1
        .split_once("\nSectionEnd")
        .expect("the section's end")
        .0;
    let open = "{{#each association.extensions as |ext| ~}}";
    let lines = between(body, open, "{{/each}}");
    assert_eq!(lines.len(), 1, "one loop over the extensions in {section}");
    let associations = config["fileAssociations"].as_array().expect("associations");
    let mut rendered = Vec::new();
    for association in associations {
        let name = association["name"].as_str().expect("a ProgID");
        for extension in association["extensions"].as_array().expect("a list") {
            let extension = extension.as_str().expect("text");
            rendered.extend(lines[0].lines().map(|line| {
                line.trim()
                    .replace("{{association.name}}", name)
                    .replace("{{ext}}", extension)
            }));
        }
    }
    rendered
}

/// The extensions of `lines` that start with `head`, as `head.EXT tail`.
fn extensions_in(lines: &[String], head: &str, tail: &str) -> BTreeSet<String> {
    lines
        .iter()
        .filter_map(|line| line.strip_prefix(head)?.strip_suffix(tail))
        .map(str::to_owned)
        .collect()
}

#[test]
fn the_installer_offers_every_decoded_extension_in_open_with_alone() {
    let config: toml::Table = read("packaging/Packager.toml").parse().expect("TOML");
    let template = read("packaging/windows/installer.nsi");
    assert_eq!(
        config["nsis"]["template"].as_str(),
        Some("@ROOT@/packaging/windows/installer.nsi")
    );
    assert!(template.contains(r#"StrCpy $INSTDIR "$LOCALAPPDATA\Programs\${PRODUCTNAME}""#));

    let install = installer_lines(&template, &config, "Install");
    let progids = r#"WriteRegNone SHCTX "Software\Classes\."#;
    let tail = r#"\OpenWithProgids" "Apertrust.RawImage""#;
    assert_eq!(extensions_in(&install, progids, tail), decoded());
    let edit_tail = format!(r#"\OpenWithProgids" "{EDIT_PROGID}""#);
    assert_eq!(extensions_in(&install, progids, &edit_tail), apxmp());
    let supported = r#"WriteRegNone SHCTX "${APPKEY}\SupportedTypes" "."#;
    let every: BTreeSet<String> = decoded().into_iter().chain(apxmp()).collect();
    assert_eq!(extensions_in(&install, supported, "\""), every);

    let uninstall = installer_lines(&template, &config, "Uninstall");
    let removed = r#"DeleteRegValue SHCTX "Software\Classes\."#;
    assert_eq!(extensions_in(&uninstall, removed, tail), decoded());
    assert_eq!(extensions_in(&uninstall, removed, &edit_tail), apxmp());

    // Never a raw extension's default value, the type's handler, nor FileAssociation.nsh's
    // macros, which take it: the edit's alone, Apertrust's own type (below).
    assert!(
        !template.contains("!insertmacro APP_"),
        "FileAssociation.nsh's macros"
    );
    let edit_default = format!(r#"WriteRegStr SHCTX "Software\Classes\.apxmp" "" "{EDIT_PROGID}""#);
    for line in template.lines() {
        let line = line.trim();
        let extension = [r#"\Classes\.{{ext}}""#, r#"\Classes\.apxmp""#];
        if line.starts_with("WriteReg") && extension.iter().any(|e| line.contains(e)) {
            assert_eq!(line, edit_default, "an extension's own value");
        }
    }
}

/// The lines of `section` in the block `{{#if (eq CONDITION) ~}}` opens, to its `{{/if`,
/// with no comments.
fn block<'a>(template: &'a str, section: &str, condition: &str) -> Vec<&'a str> {
    let body = template
        .split_once(&format!("\nSection {section}\n"))
        .unwrap_or_else(|| panic!("no section {section}"))
        .1
        .split_once("\nSectionEnd")
        .expect("the section's end")
        .0;
    let open = format!("{{{{#if (eq {condition}) ~}}}}");
    let inside = between(body, &open, "{{/if");
    assert_eq!(inside.len(), 1, "one {open} in {section}");
    inside[0]
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with(';'))
        .collect()
}

/// Apertrust owns the edit's extension on Windows: Apertrust.Edit, with the executable's
/// second icon, is `.apxmp`'s default as well, which the uninstaller clears only while it
/// is still Apertrust's. Apertrust Nightly's ProgID, of another name, never takes it.
#[test]
fn the_installer_makes_the_edit_apertrust_s_and_takes_it_back() {
    let template = read("packaging/windows/installer.nsi");
    let name = format!(r#"association.name "{EDIT_PROGID}""#);
    assert_eq!(
        block(&template, "Install", &name),
        [format!(
            r#"WriteRegStr SHCTX "Software\Classes\.apxmp" "" "{EDIT_PROGID}""#
        )]
    );
    assert_eq!(
        block(&template, "Uninstall", &name),
        [
            r#"ReadRegStr $R0 SHCTX "Software\Classes\.apxmp" """#.to_owned(),
            format!(r#"${{If}} $R0 == "{EDIT_PROGID}""#),
            r#"DeleteRegValue SHCTX "Software\Classes\.apxmp" """#.to_owned(),
            "${EndIf}".to_owned(),
        ]
    );

    // The edit's ProgID, Apertrust's or Apertrust Nightly's, shows the executable's icon 2,
    // which crates/app/build.rs embeds; the raws' its icon 1, the first.
    let icon = |index: &str| {
        let exe = r#"$\"$INSTDIR\${MAINBINARYNAME}.exe$\""#;
        let key = r#"Software\Classes\\{{association.name}}\DefaultIcon"#;
        format!(r#"WriteRegStr SHCTX "{key}" "" "{exe},{index}""#)
    };
    let edit = block(
        &template,
        "Install",
        r#"association.extensions.[0] "apxmp""#,
    );
    assert_eq!(edit, [icon("-2"), "{{else ~}}".to_owned(), icon("0")]);
    let build = read("crates/app/build.rs");
    assert!(build.contains(r#"let edit = "assets/icons/apertrust-apxmp.ico";"#));
    assert!(build.contains(r#".set_icon_with_id(edit, "2")"#));
    let ico = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/icons/apertrust-apxmp.ico");
    assert!(ico.is_file(), "{}", ico.display());
}

/// The program as a build names it, a development build or not.
fn program(dev: bool) -> apertrust_base::log::Program {
    let dev = if dev { "1" } else { "" };
    apertrust_base::log::Program::from_build("apertrust", "0.0.0", "", dev, "", "")
}

#[test]
fn every_package_names_the_app_by_one_id() {
    let config: toml::Table = read("packaging/Packager.toml").parse().expect("TOML");
    assert_eq!(config["identifier"].as_str(), Some(APP_ID));
    let desktop = read("packaging/linux/org.apertrust.Apertrust.desktop");
    assert!(desktop.contains(&format!("\nIcon={APP_ID}\n")), "{desktop}");
    let metainfo = read("packaging/linux/org.apertrust.Apertrust.metainfo.xml");
    assert!(metainfo.contains(&format!("<id>{APP_ID}</id>")));
    assert!(metainfo.contains(&format!(
        "<launchable type=\"desktop-id\">{APP_ID}.desktop</launchable>"
    )));
    assert!(metainfo.contains(&format!("<icon type=\"stock\">{APP_ID}</icon>")));
    assert_eq!(program(false).app_id(), APP_ID, "the window's");
    let main = read("crates/app/src/main.rs");
    assert!(main.contains("const APP_ID: &str = apertrust_base::program!().app_id();"));
}

/// The files `tools/package.sh --nightly` gives Apertrust Nightly's identity.
const NIGHTLY_FILES: [&str; 5] = [
    "packaging/Packager.toml",
    "packaging/macos/Info.plist",
    "packaging/linux/rpm.toml",
    "packaging/linux/org.apertrust.Apertrust.desktop",
    "packaging/linux/org.apertrust.Apertrust.metainfo.xml",
];

/// `packaging/nightly-identity.txt`'s replacements, `FROM => TO`, as the script reads them.
fn nightly_identity() -> Vec<(String, String)> {
    read("packaging/nightly-identity.txt")
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (from, to) = line.split_once(" => ").expect("FROM => TO");
            (from.to_owned(), to.to_owned())
        })
        .collect()
}

/// A file of `packaging/` as `tools/package.sh --nightly` reads it: each replacement made
/// wherever its text occurs, in order.
fn as_nightly(path: &str) -> String {
    let mut text = read(path);
    for (from, to) in nightly_identity() {
        text = text.replace(&from, &to);
    }
    text
}

#[test]
fn every_nightly_identity_line_still_matches() {
    let files: Vec<String> = NIGHTLY_FILES.iter().map(|path| read(path)).collect();
    for (from, _) in nightly_identity() {
        assert!(
            files.iter().any(|text| text.contains(&from)),
            "{from:?} in none of {NIGHTLY_FILES:?}: packaging/nightly-identity.txt is stale"
        );
    }
    // The edit's rank, replaced wherever it occurs, is the one Owner.
    let owner = "<string>Owner</string>";
    assert!(nightly_identity().iter().any(|(from, _)| from == owner));
    let count: usize = files.iter().map(|text| text.matches(owner).count()).sum();
    assert_eq!(count, 1, "{owner} is the edit's rank alone");
}

/// Apertrust Nightly's packages name it by its own id, name, binaries, Open With entry
/// and icon, so that it installs beside Apertrust with no file in common.
#[test]
fn apertrust_nightly_is_named_by_its_own_id_in_every_package() {
    let id = program(true).app_id();
    assert_eq!(id, format!("{APP_ID}.Nightly"));
    assert_eq!(program(true).app_name(), "Apertrust Nightly");

    let config: toml::Table = as_nightly("packaging/Packager.toml").parse().expect("TOML");
    assert_eq!(config["identifier"].as_str(), Some(id));
    assert_eq!(config["name"].as_str(), Some("apertrust-nightly"));
    assert_eq!(config["productName"].as_str(), Some("Apertrust Nightly"));
    let binaries: Vec<(&str, bool)> = config["binaries"]
        .as_array()
        .expect("binaries")
        .iter()
        .map(|binary| {
            let main = binary.get("main").and_then(toml::Value::as_bool);
            (binary["path"].as_str().expect("a path"), main == Some(true))
        })
        .collect();
    assert_eq!(
        binaries,
        [
            ("apertrust-nightly", true),
            ("apertrust-nightly-cli", false)
        ]
    );
    let associations = config["fileAssociations"].as_array().expect("associations");
    let names: Vec<&str> = associations
        .iter()
        .map(|association| association["name"].as_str().expect("a ProgID"))
        .collect();
    // Its edit's ProgID is not Apertrust's, which alone the installer makes the default.
    assert_eq!(names, ["Apertrust.Nightly.RawImage", NIGHTLY_EDIT_PROGID]);
    // The AppImage's desktop file, named after the binary as cargo-packager links it; the
    // .deb's files and the AppImage's others are the prefix's, which the script names.
    assert_eq!(
        config["linux"]["generateDesktopEntry"].as_bool(),
        Some(false)
    );
    let files = config["appimage"]["files"].as_table().expect("files");
    assert_eq!(
        files
            .get(&format!("@STAGE@/{id}.desktop"))
            .and_then(toml::Value::as_str),
        Some("usr/share/applications/apertrust-nightly.desktop")
    );
    let deb = config["deb"]["files"].as_table().expect("files");
    assert_eq!(
        deb.get("@PREFIX@/share").and_then(toml::Value::as_str),
        Some("usr/share")
    );
    let rpm: toml::Table = as_nightly("packaging/linux/rpm.toml")
        .parse()
        .expect("TOML");
    assert_eq!(rpm["package"]["name"].as_str(), Some("apertrust-nightly"));

    let plist = as_nightly("packaging/macos/Info.plist");
    for key in ["CFBundleName", "CFBundleDisplayName"] {
        let pair = format!("<key>{key}</key>\n\t<string>Apertrust Nightly</string>");
        assert!(plist.contains(&pair), "{key}");
    }
    // It opens edits, never taking them from Apertrust.
    let edit = between(documents(&plist)[1], "<string>", "</string>");
    assert_eq!(edit, ["Apertrust edit", "Editor", "Alternate", EDIT_TYPE]);
    assert!(!plist.contains("Owner"));

    let desktop = as_nightly("packaging/linux/org.apertrust.Apertrust.desktop");
    for line in ["Name=Apertrust Nightly", "Exec=apertrust-nightly %F"] {
        assert!(
            desktop.contains(&format!("\n{line}\n")),
            "{line} in {desktop}"
        );
    }
    assert!(
        desktop.contains(&format!("\nIcon={id}\n")),
        "its own icon in {desktop}"
    );
    let metainfo = as_nightly("packaging/linux/org.apertrust.Apertrust.metainfo.xml");
    for element in [
        format!("<id>{id}</id>"),
        "<name>Apertrust Nightly</name>".to_owned(),
        format!("<launchable type=\"desktop-id\">{id}.desktop</launchable>"),
        format!("<icon type=\"stock\">{id}</icon>"),
        "<binary>apertrust-nightly</binary>".to_owned(),
        "<binary>apertrust-nightly-cli</binary>".to_owned(),
    ] {
        assert!(metainfo.contains(&element), "{element} in {metainfo}");
    }
}

/// The libraries the app loads at run time on Linux, never linked (decision 5): each
/// library's file, which the AppImage leaves out and the `.rpm` requires by its capability,
/// and Debian's package of it, which the `.deb` depends on.
const LOADED: [(&str, &str); 10] = [
    ("libvulkan.so.1", "libvulkan1"),
    ("libwayland-client.so.0", "libwayland-client0"),
    ("libxkbcommon.so.0", "libxkbcommon0"),
    ("libxkbcommon-x11.so.0", "libxkbcommon-x11-0"),
    ("libX11.so.6", "libx11-6"),
    ("libX11-xcb.so.1", "libx11-xcb1"),
    ("libxcb.so.1", "libxcb1"),
    ("libXcursor.so.1", "libxcursor1"),
    ("libXi.so.6", "libxi6"),
    ("libdbus-1.so.3", "libdbus-1-3"),
];

/// The `.deb` depends on and the `.rpm` requires the same libraries, those the app loads
/// and, for the `.deb`, those it links, which cargo-generate-rpm finds for the `.rpm`.
#[test]
fn the_linux_packages_need_the_libraries_the_app_loads() {
    let config: toml::Table = read("packaging/Packager.toml").parse().expect("TOML");
    let depends: BTreeSet<&str> = config["deb"]["depends"]
        .as_array()
        .expect("depends")
        .iter()
        .map(|name| name.as_str().expect("a package"))
        .collect();
    let mut expected: BTreeSet<&str> = LOADED.iter().map(|(_, package)| *package).collect();
    expected.extend(["libc6 (>= @GLIBC@)", "libgcc-s1"]);
    assert_eq!(depends, expected);

    let rpm: toml::Table = read("packaging/linux/rpm.toml").parse().expect("TOML");
    let requires: BTreeSet<&str> = rpm["package"]["metadata"]["generate-rpm"]["requires"]
        .as_table()
        .expect("requires")
        .keys()
        .map(String::as_str)
        .collect();
    let capabilities: Vec<String> = LOADED
        .iter()
        .map(|(library, _)| format!("{library}()(64bit)"))
        .collect();
    let expected: BTreeSet<&str> = capabilities.iter().map(String::as_str).collect();
    assert_eq!(requires, expected);

    // The AppImage leaves every one of them to the system: each pattern's text before its
    // first `*` starts one of them.
    let excluded: Vec<&str> = config["appimage"]["excludedLibs"]
        .as_array()
        .expect("excludedLibs")
        .iter()
        .map(|pattern| pattern.as_str().expect("a pattern"))
        .map(|pattern| pattern.split('*').next().unwrap_or_default())
        .collect();
    for (library, _) in LOADED {
        assert!(
            excluded.iter().any(|stem| library.starts_with(stem)),
            "{library} in the AppImage"
        );
    }
}

#[test]
fn the_embedded_plist_agrees_with_the_bundles() {
    let pairs = |plist: &str| -> Vec<(String, String)> {
        between(plist, "<key>", "</string>")
            .into_iter()
            .filter_map(|pair| {
                let (key, rest) = pair.split_once("</key>")?;
                let value = rest.trim().strip_prefix("<string>")?;
                Some((key.to_owned(), value.to_owned()))
            })
            .collect()
    };
    let embedded = read("crates/app/assets/macos/Info.plist");
    let bundle = pairs(&read("packaging/macos/Info.plist"));
    let embedded = pairs(&embedded);
    assert!(!embedded.is_empty());
    for pair in &embedded {
        assert!(bundle.contains(pair), "{pair:?} is not the bundle's");
    }
}
