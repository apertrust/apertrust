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

//! The trees the check walks, worked out from the patterns, and the candidates it finds.

use tempfile::TempDir;

use super::*;

/// `xmpMM:PreservedFileName`, keeping the camera name `name`, as darktable writes it.
pub(crate) fn preserved(name: &str) -> crate::xmp::Property {
    let name_of = crate::xmp::Name::new(
        "xmpMM",
        "http://ns.adobe.com/xap/1.0/mm/",
        "PreservedFileName",
    );
    crate::xmp::Property::simple(name_of, name)
}

/// The person's folders, under a home at `home`, its folders joined with `/`.
fn places(home: &str) -> Places {
    Places {
        home: Some(PathBuf::from(home)),
        pictures: Some(PathBuf::from(format!("{home}/Pictures"))),
        desktop: Some(PathBuf::from(format!("{home}/Desktop"))),
        user_name: "ana".to_owned(),
    }
}

/// `$(HOME)`, `$(PICTURES_FOLDER)` and a leading `~` come from the places; a pattern is cut
/// at its first other variable, back to the last whole folder; what is no absolute folder,
/// or the file system's root, `/`, is no tree, as on macOS and Linux, on every system.
#[test]
fn a_tree_is_its_pattern_up_to_its_first_other_variable() {
    let ana = places("/home/ana");
    let root = |pattern: &str| tree_root_as(pattern, &ana, false, "C:");
    let path = |text: &str| Some(PathBuf::from(text));
    assert_eq!(
        root("$(PICTURES_FOLDER)/Apertrust"),
        path("/home/ana/Pictures/Apertrust")
    );
    assert_eq!(root("$(HOME)/Photos/$(YEAR)"), path("/home/ana/Photos"));
    assert_eq!(
        root("$(HOME)/Photos/Shoot_$(JOBCODE)"),
        path("/home/ana/Photos")
    );
    assert_eq!(root("~/Archive"), path("/home/ana/Archive"));
    assert_eq!(
        root("/Volumes/NAS/$(USERNAME)/in"),
        path("/Volumes/NAS/ana/in")
    );
    assert_eq!(root("$(EXIF.YEAR)/x"), None);
    assert_eq!(root("relative/folder"), None);
    assert_eq!(root("$(UNKNOWN_VARIABLE)/x"), None);
    assert_eq!(root("$(PICTURES_FOLDER)/x"), path("/home/ana/Pictures/x"));
    // The file system's root is refused; any other volume is a tree, its root too.
    assert_eq!(root("/$(YEAR)"), None);
    assert_eq!(root("/"), None);
    assert_eq!(root("/Volumes/Photos/$(YEAR)"), path("/Volumes/Photos"));
    assert_eq!(root("/Volumes/Photos"), path("/Volumes/Photos"));
    assert_eq!(root("/media/ana/Backup/$(YEAR)"), path("/media/ana/Backup"));
    assert_eq!(root("C:/Photos"), None, "no drive on macOS and Linux");
}

/// Where the system names no Pictures folder, as Linux without its user folders,
/// `$(PICTURES_FOLDER)` is the home's Pictures, as darktable's is; with no home either, no
/// tree.
#[cfg(unix)]
#[test]
fn with_no_pictures_folder_the_home_s_pictures_stands_in() {
    let no_pictures = Places {
        pictures: None,
        ..places("/home/ana")
    };
    assert_eq!(
        tree_root_as("$(PICTURES_FOLDER)/Apertrust", &no_pictures, false, "C:"),
        Some(PathBuf::from("/home/ana/Pictures/Apertrust"))
    );
    let nobody = Places::default();
    assert_eq!(
        tree_root_as("$(PICTURES_FOLDER)/$(YEAR)", &nobody, false, "C:"),
        None
    );
}

/// On Windows, as on every system: the system drive's root is refused, in any case and
/// with or without its separator; any other drive's root is a tree, as a folder is, and so
/// is a folder on the system drive or a share's.
#[test]
fn on_windows_only_the_system_drive_s_root_is_refused() {
    let ana = places("C:/Users/ana");
    let root = |pattern: &str| tree_root_as(pattern, &ana, true, "C:");
    let path = |text: &str| Some(PathBuf::from(text));
    assert_eq!(root("C:/$(YEAR)"), None);
    assert_eq!(root("c:/"), None);
    assert_eq!(root("C:"), None);
    assert_eq!(
        root("$(PICTURES_FOLDER)/Apertrust"),
        path("C:/Users/ana/Pictures/Apertrust")
    );
    assert_eq!(root("C:/Photos/$(YEAR)"), path("C:/Photos"));
    assert_eq!(root("D:/$(YEAR)"), path("D:/"));
    assert_eq!(root("D:/"), path("D:/"));
    assert_eq!(root("E:/Photos/$(EXIF.YEAR)"), path("E:/Photos"));
    assert_eq!(root("//nas/photos/$(YEAR)"), path("//nas/photos"));
    assert_eq!(root("/Photos"), None, "no drive");
    // Windows set on another drive.
    let on_d = |pattern: &str| tree_root_as(pattern, &ana, true, "D:");
    assert_eq!(on_d("C:/$(YEAR)"), path("C:/"));
    assert_eq!(on_d("D:/$(YEAR)"), None);
}

/// The default folder's tree is walked where Folder lies elsewhere, once where Folder lies
/// inside it or it inside Folder's.
#[cfg(unix)]
#[test]
fn the_default_folder_s_tree_is_walked_unless_folder_s_holds_it() {
    let ana = places("/home/ana");
    let default = "$(PICTURES_FOLDER)/Apertrust";
    let trees = |base: &str| trees(base, default, &ana);
    let path = PathBuf::from;
    assert_eq!(
        trees("/Volumes/Albums/Wedding"),
        [
            path("/Volumes/Albums/Wedding"),
            path("/home/ana/Pictures/Apertrust")
        ]
    );
    assert_eq!(
        trees("$(PICTURES_FOLDER)/Apertrust/2026"),
        [path("/home/ana/Pictures/Apertrust")]
    );
    assert_eq!(trees("$(PICTURES_FOLDER)"), [path("/home/ana/Pictures")]);
    assert_eq!(trees(default), [path("/home/ana/Pictures/Apertrust")]);
    assert_eq!(trees("$(YEAR)"), [path("/home/ana/Pictures/Apertrust")]);
}

/// Candidates by size and name, or by the name a sidecar keeps, read only for a raw of a
/// size sought; the source's own folder left out; a tree not there holds nothing.
#[test]
fn candidates_are_found_by_size_and_name() {
    let dir = TempDir::new().unwrap();
    let tree = dir.path().join("Apertrust");
    let day = tree.join("20261108_Studio");
    let source = tree.join("incoming");
    for folder in [&day, &source] {
        fs::create_dir_all(folder).unwrap();
    }
    fs::write(day.join("A.CR3"), b"aaaa").unwrap();
    fs::write(day.join("other.CR3"), b"bbbbbb").unwrap();
    // Of another size, its sidecar never read.
    fs::write(day.join("big.CR3"), b"a much larger raw").unwrap();
    fs::write(day.join("big.CR3.apxmp"), b"not read").unwrap();
    fs::write(source.join("A.CR3"), b"aaaa").unwrap();
    let mut edits = Edits::new("test");
    edits.other.push(preserved("B.CR3"));
    fs::write(day.join("other.CR3.apxmp"), edits.to_xml()).unwrap();
    let sought = Sought::new([("A.CR3", 4), ("B.CR3", 6), ("C.CR3", 4)]);
    let gone = dir.path().join("gone");
    let found = candidates(
        &[tree.clone(), gone],
        &source,
        std::slice::from_ref(&source),
        sought,
        &|| false,
    );
    let Walked::Found(found) = found else {
        panic!("{found:?}");
    };
    let paths: Vec<Vec<&Path>> = (found.iter())
        .map(|found| found.iter().map(|c| c.path.as_path()).collect())
        .collect();
    let (a, other) = (day.join("A.CR3"), day.join("other.CR3"));
    assert_eq!(paths, [vec![a.as_path()], vec![other.as_path()], vec![]]);
    assert!(found[0][0].local);
    assert_eq!(crate::testing::reads(&day.join("big.CR3.apxmp")), 0);
    assert_eq!(crate::testing::reads(&day.join("other.CR3.apxmp")), 1);
}
