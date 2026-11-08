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
//! With alone; the app's id is the same everywhere; and the plist the binary embeds
//! agrees with the bundle's (docs/packaging.md).

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const APP_ID: &str = "org.apertrust.Apertrust";

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

#[test]
fn linux_opens_every_decoded_extension() {
    let desktop = read("packaging/linux/org.apertrust.Apertrust.desktop");
    let line = desktop
        .lines()
        .find_map(|line| line.strip_prefix("MimeType="))
        .expect("a MimeType line");
    let listed: BTreeSet<&str> = line.split(';').filter(|t| !t.is_empty()).collect();
    let wanted: BTreeSet<&str> = TYPES.iter().map(|t| t.2).collect();
    assert_eq!(listed, wanted);

    let mime = read("packaging/linux/org.apertrust.Apertrust.mime.xml");
    let declared: BTreeSet<(&str, &str)> = between(&mime, "<mime-type type=\"", "</mime-type>")
        .into_iter()
        .map(|declaration| {
            let kind = declaration.split_once('"').expect("a type").0;
            let glob = between(declaration, "<glob pattern=\"*.", "\"/>");
            (kind, glob[0])
        })
        .collect();
    let ours: BTreeSet<(&str, &str)> = TYPES.iter().filter(|t| t.3).map(|t| (t.2, t.0)).collect();
    assert_eq!(declared, ours, "the MIME types shared-mime-info lacks");
}

#[test]
fn the_installer_registers_every_decoded_extension() {
    let config: toml::Table = read("packaging/Packager.toml").parse().expect("TOML");
    let associations = config["fileAssociations"].as_array().expect("associations");
    let registered: BTreeSet<String> = associations
        .iter()
        .flat_map(|association| association["extensions"].as_array().expect("a list"))
        .map(|extension| extension.as_str().expect("text").to_owned())
        .collect();
    assert_eq!(registered, decoded());
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
    let supported = r#"WriteRegNone SHCTX "${APPKEY}\SupportedTypes" "."#;
    assert_eq!(extensions_in(&install, supported, "\""), decoded());

    let uninstall = installer_lines(&template, &config, "Uninstall");
    let removed = r#"DeleteRegValue SHCTX "Software\Classes\."#;
    assert_eq!(extensions_in(&uninstall, removed, tail), decoded());

    // Never an extension's default value, the type's handler, nor FileAssociation.nsh's
    // macros, which take it.
    assert!(
        !template.contains("!insertmacro APP_"),
        "FileAssociation.nsh's macros"
    );
    for line in template.lines() {
        let line = line.trim();
        if line.starts_with("WriteReg") && line.contains(r#"\Classes\.{{ext}}""#) {
            panic!("an extension's own value: {line}");
        }
    }
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
    let main = read("crates/app/src/main.rs");
    assert!(main.contains(&format!("const APP_ID: &str = \"{APP_ID}\";")));
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
