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

//! Edits to darktable XMP sidecars.

use std::io::Read;
use std::ops::Range;

use anyhow::{Context, Result, ensure};
use base64::Engine;

/// Switches off every history entry for `operation` (e.g. `"filmicrgb"`), so darktable
/// renders the sidecar without that module.
pub fn disable_operation(xmp: &str, operation: &str) -> Result<String> {
    edit_entries(xmp, operation, |entry| {
        ensure!(
            entry.contains("darktable:enabled="),
            "history entry for {operation} has no enabled flag"
        );
        Ok(entry.replace("darktable:enabled=\"1\"", "darktable:enabled=\"0\""))
    })
}

/// Removes every history entry for `operation`, renumbering the later entries and the
/// history's end, so darktable renders the module with its own defaults for the image.
pub fn remove_operation(xmp: &str, operation: &str) -> Result<String> {
    let ranges = entries(xmp, operation);
    ensure!(
        !ranges.is_empty(),
        "sidecar has no history entry for {operation}"
    );
    let removed: Vec<u32> = ranges
        .iter()
        .map(|range| Ok(attribute(&xmp[range.clone()], "darktable:num")?.parse()?))
        .collect::<Result<_>>()?;
    let mut out = String::with_capacity(xmp.len());
    let mut copied = 0;
    for range in &ranges {
        // The entry's line, its indentation and line break included.
        let start = xmp[..range.start]
            .rfind('\n')
            .map_or(range.start, |at| at + 1);
        out.push_str(&xmp[copied..start]);
        copied = range.end + usize::from(xmp[range.end..].starts_with('\n'));
    }
    out.push_str(&xmp[copied..]);
    let earlier = |num: u32| removed.iter().filter(|&&r| r < num).count() as u32;
    let mut renumbered = renumber(&out, "darktable:num", &earlier)?;
    let end: u32 = attribute(&renumbered, "darktable:history_end")?.parse()?;
    renumbered = renumbered.replacen(
        &format!("darktable:history_end=\"{end}\""),
        &format!("darktable:history_end=\"{}\"", end - earlier(end)),
        1,
    );
    Ok(renumbered)
}

/// Appends `from`'s active history entry for `operation`, another sidecar's, to `xmp`'s
/// history as its last entry, numbered after the others and within its history's end, so
/// darktable renders the module as `from` sets it. `xmp` must hold no entry for it.
pub fn append_entry(xmp: &str, from: &str, operation: &str) -> Result<String> {
    ensure!(
        entries(xmp, operation).is_empty(),
        "the sidecar already has a history entry for {operation}"
    );
    let entry = active_entry(from, operation)?;
    let end: u32 = attribute(xmp, "darktable:history_end")?.parse()?;
    let num = attribute(entry, "darktable:num")?;
    let entry = entry.replacen(
        &format!("darktable:num=\"{num}\""),
        &format!("darktable:num=\"{end}\""),
        1,
    );
    let history = xmp
        .find("</darktable:history>")
        .context("the sidecar has no history")?;
    let close = xmp[..history]
        .rfind("</rdf:Seq>")
        .context("the sidecar's history has no sequence")?;
    let line = xmp[..close].rfind('\n').map_or(close, |at| at + 1);
    let mut out = String::with_capacity(xmp.len() + entry.len() + 8);
    out.push_str(&xmp[..line]);
    out.push_str("     ");
    out.push_str(&entry);
    out.push('\n');
    out.push_str(&xmp[line..]);
    Ok(out.replacen(
        &format!("darktable:history_end=\"{end}\""),
        &format!("darktable:history_end=\"{}\"", end + 1),
        1,
    ))
}

/// Lowers every `name="N"` attribute by `shift(N)`.
fn renumber(xmp: &str, name: &str, shift: &dyn Fn(u32) -> u32) -> Result<String> {
    let key = format!("{name}=\"");
    let mut out = String::with_capacity(xmp.len());
    let mut rest = xmp;
    while let Some(at) = rest.find(&key) {
        let start = at + key.len();
        let len = rest[start..]
            .find('"')
            .with_context(|| format!("unterminated {name}"))?;
        let num: u32 = rest[start..start + len].parse()?;
        out.push_str(&rest[..start]);
        out.push_str(&(num - shift(num)).to_string());
        rest = &rest[start + len..];
    }
    out.push_str(rest);
    Ok(out)
}

/// Gives every `to` entry the blend settings of the first `from` entry. Copying from an
/// unmasked module of the same blend version removes `to`'s mask, so darktable applies
/// it to the whole image.
pub fn copy_blend_params(xmp: &str, from: &str, to: &str) -> Result<String> {
    let source = entries(xmp, from)
        .first()
        .map(|range| &xmp[range.clone()])
        .with_context(|| format!("sidecar has no history entry for {from}"))?;
    let (version, params) = (
        attribute(source, "darktable:blendop_version")?,
        attribute(source, "darktable:blendop_params")?,
    );
    edit_entries(xmp, to, |entry| {
        ensure!(
            attribute(entry, "darktable:blendop_version")? == version,
            "{from} and {to} use different blend versions"
        );
        let old = attribute(entry, "darktable:blendop_params")?;
        Ok(entry.replace(
            &format!("darktable:blendop_params=\"{old}\""),
            &format!("darktable:blendop_params=\"{params}\""),
        ))
    })
}

/// Gives every history entry for `operation` these params, written as hex. The bytes
/// must be the layout of the entries' `modversion`.
pub fn set_params(xmp: &str, operation: &str, bytes: &[u8]) -> Result<String> {
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    edit_entries(xmp, operation, |entry| {
        let old = attribute(entry, "darktable:params")?;
        Ok(entry.replace(
            &format!("darktable:params=\"{old}\""),
            &format!("darktable:params=\"{hex}\""),
        ))
    })
}

/// Gives every history entry for `operation` these params, written as hex, and their
/// layout's `version` as its `modversion`, so darktable reads them without migrating
/// them.
pub fn set_params_and_version(
    xmp: &str,
    operation: &str,
    version: u32,
    bytes: &[u8],
) -> Result<String> {
    let xmp = set_params(xmp, operation, bytes)?;
    edit_entries(&xmp, operation, |entry| {
        let old = attribute(entry, "darktable:modversion")?;
        Ok(entry.replace(
            &format!("darktable:modversion=\"{old}\""),
            &format!("darktable:modversion=\"{version}\""),
        ))
    })
}

/// Whether the active history entry for `operation`, the last one before
/// `history_end`, is switched on.
pub fn is_enabled(xmp: &str, operation: &str) -> Result<bool> {
    Ok(attribute(active_entry(xmp, operation)?, "darktable:enabled")? == "1")
}

/// Every operation with a history entry, once each, in the order of their first entry.
pub fn operations(xmp: &str) -> Vec<&str> {
    let mut found: Vec<&str> = Vec::new();
    for (at, needle) in xmp.match_indices("darktable:operation=\"") {
        let start = at + needle.len();
        if let Some(len) = xmp[start..].find('"') {
            let operation = &xmp[start..start + len];
            if !found.contains(&operation) {
                found.push(operation);
            }
        }
    }
    found
}

/// The version and params of every history entry for `operation`.
pub fn all_params(xmp: &str, operation: &str) -> Result<Vec<ModuleParams>> {
    entries(xmp, operation)
        .into_iter()
        .map(|range| {
            let entry = &xmp[range];
            Ok(ModuleParams {
                version: attribute(entry, "darktable:modversion")?.parse()?,
                bytes: decode_params(attribute(entry, "darktable:params")?)
                    .with_context(|| format!("decoding {operation} params"))?,
            })
        })
        .collect()
}

/// A module's parameters as darktable stores them: its params struct, raw bytes.
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleParams {
    pub version: u32,
    pub bytes: Vec<u8>,
}

/// The parameters of the active, enabled history entry for `operation`: the last one
/// before `history_end`.
pub fn module_params(xmp: &str, operation: &str) -> Result<ModuleParams> {
    let entry = active_entry(xmp, operation)?;
    ensure!(
        attribute(entry, "darktable:enabled")? == "1",
        "{operation} is disabled in the sidecar"
    );
    let version = attribute(entry, "darktable:modversion")?.parse()?;
    let bytes = decode_params(attribute(entry, "darktable:params")?)
        .with_context(|| format!("decoding {operation} params"))?;
    Ok(ModuleParams { version, bytes })
}

/// The last history entry for `operation` before `history_end`.
fn active_entry<'a>(xmp: &'a str, operation: &str) -> Result<&'a str> {
    let history_end: u32 = attribute(xmp, "darktable:history_end")?.parse()?;
    entries(xmp, operation)
        .into_iter()
        .map(|range| &xmp[range])
        .rfind(|entry| {
            attribute(entry, "darktable:num")
                .ok()
                .and_then(|num| num.parse::<u32>().ok())
                .is_some_and(|num| num < history_end)
        })
        .with_context(|| format!("no active history entry for {operation}"))
}

/// darktable stores params as hex, or as `gz` + a two-digit compression ratio + base64
/// of a zlib stream when that is shorter.
fn decode_params(text: &str) -> Result<Vec<u8>> {
    if let Some(compressed) = text.strip_prefix("gz") {
        let base64 = compressed.get(2..).context("truncated compressed params")?;
        let zlib = base64::engine::general_purpose::STANDARD.decode(base64)?;
        let mut bytes = Vec::new();
        flate2::read::ZlibDecoder::new(zlib.as_slice()).read_to_end(&mut bytes)?;
        return Ok(bytes);
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(text.get(i..i + 2).unwrap_or("?"), 16))
        .collect::<Result<_, _>>()
        .context("params are neither hex nor compressed")
}

/// Byte ranges of the history entries for `operation`, each one self-closing
/// `<rdf:li .../>` element.
fn entries(xmp: &str, operation: &str) -> Vec<Range<usize>> {
    let needle = format!("darktable:operation=\"{operation}\"");
    xmp.match_indices(&needle)
        .map(|(at, _)| {
            let start = xmp[..at].rfind("<rdf:li").unwrap_or(at);
            let end = xmp[at..].find("/>").map_or(xmp.len(), |e| at + e + 2);
            start..end
        })
        .collect()
}

fn edit_entries(
    xmp: &str,
    operation: &str,
    mut edit: impl FnMut(&str) -> Result<String>,
) -> Result<String> {
    let ranges = entries(xmp, operation);
    ensure!(
        !ranges.is_empty(),
        "sidecar has no history entry for {operation}"
    );
    let mut out = String::with_capacity(xmp.len());
    let mut copied = 0;
    for range in ranges {
        out.push_str(&xmp[copied..range.start]);
        out.push_str(&edit(&xmp[range.clone()])?);
        copied = range.end;
    }
    out.push_str(&xmp[copied..]);
    Ok(out)
}

fn attribute<'a>(entry: &'a str, name: &str) -> Result<&'a str> {
    let key = format!("{name}=\"");
    let start = entry
        .find(&key)
        .map(|at| at + key.len())
        .with_context(|| format!("history entry has no {name}"))?;
    let len = entry[start..]
        .find('"')
        .with_context(|| format!("unterminated {name}"))?;
    Ok(&entry[start..start + len])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_entry_appended_from_another_sidecar_is_its_active_one() {
        let base = include_str!("../cases/agx-default.xmp");
        let grain = include_str!("../cases/grain.xmp");
        let xmp = append_entry(base, grain, "grain").unwrap();
        assert_eq!(
            module_params(&xmp, "grain").unwrap(),
            module_params(grain, "grain").unwrap()
        );
        assert_eq!(attribute(&xmp, "darktable:history_end").unwrap(), "12");
        assert_eq!(operations(&xmp).last(), Some(&"grain"));
        assert!(append_entry(&xmp, grain, "grain").is_err());
    }

    const XMP: &str = r#"<rdf:Seq>
     <rdf:li
      darktable:operation="exposure"
      darktable:enabled="1"
      darktable:blendop_version="11"
      darktable:blendop_params="plain"/>
     <rdf:li
      darktable:operation="filmicrgb"
      darktable:enabled="1"
      darktable:blendop_version="11"
      darktable:blendop_params="plain"/>
     <rdf:li
      darktable:operation="blurs"
      darktable:enabled="1"
      darktable:blendop_version="11"
      darktable:blendop_params="masked"/>
     <rdf:li
      darktable:operation="exposure"
      darktable:enabled="1"
      darktable:blendop_version="11"
      darktable:blendop_params="plain"/>
    </rdf:Seq>"#;

    fn entry_of<'a>(xmp: &'a str, operation: &str) -> &'a str {
        &xmp[entries(xmp, operation)[0].clone()]
    }

    #[test]
    fn operations_are_listed_once_in_history_order() {
        assert_eq!(operations(XMP), ["exposure", "filmicrgb", "blurs"]);
    }

    #[test]
    fn set_params_rewrites_every_entry_as_hex() {
        let xmp = XMP.replace(
            "darktable:operation=\"exposure\"",
            "darktable:operation=\"exposure\" darktable:modversion=\"6\" darktable:params=\"gz00old\"",
        );
        let edited = set_params(&xmp, "exposure", &[0x00, 0xab, 0x10]).unwrap();
        let params = all_params(&edited, "exposure").unwrap();
        assert_eq!(params.len(), 2);
        assert!(
            params
                .iter()
                .all(|p| p.version == 6 && p.bytes == [0x00, 0xab, 0x10])
        );
        assert!(edited.contains("darktable:params=\"00ab10\""));
    }

    #[test]
    fn set_params_and_version_rewrites_both() {
        let xmp = r#"darktable:history_end="2">
     <rdf:li darktable:num="0" darktable:operation="exposure" darktable:enabled="1"
      darktable:modversion="6" darktable:params="0000803f"/>
     <rdf:li darktable:num="1" darktable:operation="agx" darktable:enabled="0"
      darktable:modversion="4" darktable:params="00"/>"#;
        let edited = set_params_and_version(xmp, "exposure", 7, &[0x01, 0x02]).unwrap();
        let params = module_params(&edited, "exposure").unwrap();
        assert_eq!((params.version, params.bytes), (7, vec![0x01, 0x02]));
        assert!(edited.contains("darktable:modversion=\"4\""));
        assert!(is_enabled(&edited, "exposure").unwrap());
        assert!(!is_enabled(&edited, "agx").unwrap());
    }

    #[test]
    fn disables_every_entry_of_one_operation() {
        let out = disable_operation(XMP, "exposure").unwrap();
        assert_eq!(out.matches("darktable:enabled=\"0\"").count(), 2);
        assert!(entry_of(&out, "filmicrgb").contains("darktable:enabled=\"1\""));
    }

    #[test]
    fn missing_operation_is_an_error() {
        assert!(disable_operation(XMP, "sigmoid").is_err());
    }

    #[test]
    fn reads_the_last_active_entry() {
        let xmp = r#"darktable:history_end="2">
     <rdf:li darktable:num="0" darktable:operation="exposure" darktable:enabled="1"
      darktable:modversion="5" darktable:params="0000803f"/>
     <rdf:li darktable:num="1" darktable:operation="exposure" darktable:enabled="1"
      darktable:modversion="6" darktable:params="00000040"/>
     <rdf:li darktable:num="2" darktable:operation="exposure" darktable:enabled="1"
      darktable:modversion="6" darktable:params="00004040"/>"#;
        let params = module_params(xmp, "exposure").unwrap();
        assert_eq!(params.version, 6);
        assert_eq!(params.bytes, 2.0f32.to_le_bytes());
    }

    #[test]
    fn removes_an_operation_and_renumbers_the_history() {
        let xmp = r#"darktable:history_end="3">
     <rdf:li darktable:num="0" darktable:operation="rawprepare" darktable:enabled="1"/>
     <rdf:li darktable:num="1" darktable:operation="demosaic" darktable:enabled="1"/>
     <rdf:li darktable:num="2" darktable:operation="exposure" darktable:enabled="1"/>
    </rdf:Seq>"#;
        let out = remove_operation(xmp, "rawprepare").unwrap();
        assert_eq!(
            out,
            r#"darktable:history_end="2">
     <rdf:li darktable:num="0" darktable:operation="demosaic" darktable:enabled="1"/>
     <rdf:li darktable:num="1" darktable:operation="exposure" darktable:enabled="1"/>
    </rdf:Seq>"#
        );
        assert!(remove_operation(&out, "rawprepare").is_err());
    }

    #[test]
    fn decodes_compressed_params() {
        // darktable's encoding of the colorout params in the test sidecars.
        let bytes = decode_params("gz35eJxjZBgFo4CBAQAEEAAC").unwrap();
        assert!(!bytes.is_empty());
        assert_eq!(decode_params("0000803f").unwrap(), 1.0f32.to_le_bytes());
    }

    #[test]
    fn copies_blend_params_to_remove_a_mask() {
        let out = copy_blend_params(XMP, "filmicrgb", "blurs").unwrap();
        assert!(entry_of(&out, "blurs").contains("darktable:blendop_params=\"plain\""));
        assert!(!out.contains("masked"));
        assert_eq!(out.len(), XMP.len() - "masked".len() + "plain".len());
    }
}
