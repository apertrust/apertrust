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

//! Drag and drop onto the main window (`docs/gui.md`, The menu bar, File): what is dropped
//! opens as Open Folder… or Open… would open it.
//!
//! - A folder opens as Open Folder… does, the first of several, and raws as Open… does,
//!   every raw dropped together in one photo (`docs/library.md`, One photo), from any
//!   folders, the first active.
//! - An edit, `IMG.ARW.apxmp`, opens its photo, `IMG.ARW` beside it, as the raw would;
//!   one whose photo is not there is refused with a message naming the photo. So do the
//!   edits the system opens with the app, Open With's and a double-click's.
//! - The app's command line is taken as a drop is ([`Drops::look_given`]), each path
//!   resolved first, a refusal's reason also written to stderr.
//! - Anything that is neither a folder nor a raw the decoder reads is refused with a
//!   message, a `Name.aplibrary` among them: no file is a library (decision 101).
//! - Nothing is taken while a modal dialog is up, the open dialog and an alert among them.
//! - Whether a path is a folder is asked of the disk on a thread of its own, never the
//!   interface's, as the recents are checked, so a slow share never holds a frame; what it
//!   opens opens a later frame ([`Drops`]). As with the recents' check, a share that hangs
//!   keeps that thread until it answers; a later drop starts a thread of its own and is
//!   the one taken, the hung one's answer, when it comes, going nowhere.
//! - While something is dragged over the window, the photo and the Browser, where it
//!   opens, are outlined ([`show_target`]).
//!
//! The paths come from egui's input, which egui-winit fills from winit's dropped and
//! hovered files on macOS, Windows and X11; winit 0.30 has no drops on Wayland.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};

use apertrust_engine::{session, sidecar};
use apertrust_ui::theme;
use eframe::egui;

/// What a dropped path is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Folder,
    /// A file whose extension the decoder reads.
    Raw,
    /// Anything else, or a path that cannot be asked about.
    Other,
}

/// What a drop opens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Dropped {
    /// A folder, opened as Open Folder… opens one.
    Folder(PathBuf),
    /// Raws, opened together as Open… opens them, in one photo: the first, active, and the
    /// others dropped after it, from any folders, in the order dropped.
    Raws {
        first: PathBuf,
        others: Vec<PathBuf>,
    },
    /// Nothing that opens: the first path dropped, and why.
    Refused { path: PathBuf, why: Refusal },
}

/// Why nothing dropped opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// An edit was among them whose photo, named as a raw, is not beside it.
    NoPhoto,
    /// Nothing was a folder or a raw.
    Unknown,
}

/// Whether `path`'s extension, in any case, is one of `extensions`.
fn has_extension(path: &Path, extensions: &[&str]) -> bool {
    let extension = path.extension().and_then(|e| e.to_str());
    extension.is_some_and(|e| extensions.iter().any(|x| x.eq_ignore_ascii_case(e)))
}

/// Whether `path` is named as a raw the decoder reads.
fn named_raw(path: &Path) -> bool {
    has_extension(path, session::raw_extensions())
}

/// What `path` is, asked of the disk: a folder, else a file told by its name.
pub fn kind(path: &Path) -> Kind {
    found(path).unwrap_or(Kind::Other)
}

/// What `path` is, as [`kind`] says, or why the disk cannot say, as for nothing there.
fn found(path: &Path) -> std::io::Result<Kind> {
    let metadata = std::fs::metadata(path)?;
    Ok(if metadata.is_dir() {
        Kind::Folder
    } else {
        file_kind(path)
    })
}

/// What dropping `paths` opens, each asked with `kind` in order until the first that
/// opens, an edit standing for its photo ([`sidecar::photo_for`]): a folder; or a raw,
/// with every raw dropped after it, from any folders, each once, an edit's photo where
/// `kind` finds it a raw; or, where none opens, the first edit without its photo refused,
/// where one was among them, else the first path. None for nothing dropped.
pub fn opens(paths: &[PathBuf], mut kind: impl FnMut(&Path) -> Kind) -> Option<Dropped> {
    let photos: Vec<PathBuf> = paths.iter().map(|path| photo(path)).collect();
    let mut refused = None;
    for (i, (path, photo)) in paths.iter().zip(&photos).enumerate() {
        // A folder named like an edit, `Album.apxmp`, is the folder it is.
        let edit = photo != path && kind(path) != Kind::Folder;
        let photo = if edit { photo } else { path };
        match kind(photo) {
            Kind::Folder if !edit => return Some(Dropped::Folder(path.clone())),
            Kind::Raw => {
                // Each once: a raw dropped with its edit is one photo. A folder, named like
                // a raw or an edit, and an edit whose photo is not there stand for none.
                // A folder named like an edit is asked as itself, never as a raw beside it.
                let mut others: Vec<PathBuf> = Vec::new();
                let later = paths[i + 1..].iter().zip(&photos[i + 1..]);
                for (given, other) in later {
                    let folder = other != given && kind(given) == Kind::Folder;
                    if !folder
                        && other != photo
                        && !others.contains(other)
                        && kind(other) == Kind::Raw
                    {
                        others.push(other.clone());
                    }
                }
                let first = photo.clone();
                return Some(Dropped::Raws { first, others });
            }
            // Named as a raw, yet no raw there: the photo is gone, or was never beside it.
            _ if edit && named_raw(photo) => refused = refused.or(Some((path, Refusal::NoPhoto))),
            _ => {}
        }
    }
    let (path, why) = refused.or_else(|| Some((paths.first()?, Refusal::Unknown)))?;
    let path = path.clone();
    Some(Dropped::Refused { path, why })
}

/// What `path` opens as: an edit's photo, `IMG.ARW` for `IMG.ARW.apxmp`, else the path.
fn photo(path: &Path) -> PathBuf {
    sidecar::photo_for(path).unwrap_or_else(|| path.to_owned())
}

/// What Open…'s answer `paths` opens, told by their names alone, the dialog having shown
/// only files: the raws chosen, together, the first active, or the first path refused as
/// a drop of it would be, should a name typed into the dialog be a file that is no raw.
/// None for no path.
pub fn chosen(paths: &[PathBuf]) -> Option<Dropped> {
    opens(paths, file_kind)
}

/// What a file is, told by its name alone.
fn file_kind(path: &Path) -> Kind {
    if named_raw(path) {
        Kind::Raw
    } else {
        Kind::Other
    }
}

/// What the paths the app's command line gave open, as a drop of them would, each
/// [`resolved`] first as `kind` finds it; where nothing opens, the reason is written to
/// stderr too, for a terminal, the alert in the window saying it for a launcher, which
/// has none (`docs/packaging.md`, Opening a raw from the system). Where something opens,
/// each path left out that would open nothing on its own is written there with its reason,
/// as a photo not there among photos that are; the window says nothing of them.
fn given_opens(paths: &[PathBuf], mut kind: impl FnMut(&Path) -> Kind) -> Option<Dropped> {
    let paths: Vec<PathBuf> = paths
        .iter()
        .map(|path| resolved(path, kind(path) == Kind::Folder))
        .collect();
    let dropped = opens(&paths, &mut kind);
    let refused = match &dropped {
        Some(Dropped::Refused { path, why }) => vec![(path.clone(), *why)],
        Some(_) => left_out(&paths, &mut kind),
        None => Vec::new(),
    };
    for (path, why) in refused {
        let missing = std::fs::metadata(&path).err();
        eprintln!("apertrust: {}", refused_line(&path, why, missing.as_ref()));
    }
    dropped
}

/// What hand-offs joined while the app started open (decision 89, `instance::Join`),
/// `groups` each one's paths in the order they came, the command line's first: each opens
/// as [`given_opens`] has it, and they follow one another as they would once the window
/// had shown, but that raws join raws. So raws after raws open with them in one photo,
/// the first active; a folder replaces what opened before it, and raws after a folder
/// replace it, never ignored for it; what opens nothing changes nothing, unless nothing
/// opens at all. None for no path.
fn joined(groups: &[Vec<PathBuf>], mut kind: impl FnMut(&Path) -> Kind) -> Option<Dropped> {
    let mut joined: Option<Dropped> = None;
    for group in groups {
        let Some(next) = given_opens(group, &mut kind) else {
            continue;
        };
        joined = Some(match (joined, next) {
            (
                Some(Dropped::Raws { first, mut others }),
                Dropped::Raws {
                    first: more,
                    others: rest,
                },
            ) => {
                for photo in std::iter::once(more).chain(rest) {
                    if photo != first && !others.contains(&photo) {
                        others.push(photo);
                    }
                }
                Dropped::Raws { first, others }
            }
            (Some(before), Dropped::Refused { .. }) => before,
            (_, next) => next,
        });
    }
    joined
}

/// The paths of `paths` that would open nothing on their own, each with why, as `kind`
/// finds them: what a drop of them all leaves out without a word.
fn left_out(paths: &[PathBuf], mut kind: impl FnMut(&Path) -> Kind) -> Vec<(PathBuf, Refusal)> {
    let alone = |path: &PathBuf, kind: &mut dyn FnMut(&Path) -> Kind| match opens(
        std::slice::from_ref(path),
        kind,
    ) {
        Some(Dropped::Refused { path, why }) => Some((path, why)),
        _ => None,
    };
    paths
        .iter()
        .filter_map(|path| alone(path, &mut kind))
        .collect()
}

/// Why `path`, the command line's, opens nothing, as stderr says it: what the disk said
/// where it could not be asked about, `missing`, as for nothing there; else `why`.
pub fn refused_line(path: &Path, why: Refusal, missing: Option<&std::io::Error>) -> String {
    let given = path.display();
    if let Some(error) = missing {
        return format!("{given}: {error}");
    }
    match why {
        Refusal::NoPhoto => {
            let photo = photo(path);
            let photo = crate::file_name(&photo);
            format!("{given}: its photo, {photo}, is not in the same folder")
        }
        Refusal::Unknown => format!("{given}: only folders and raw photos open"),
    }
}

/// `path`, a `folder` or a photo the command line names, as the app keeps it: absolute,
/// from the folder the app starts in, its `.` and `..` and links resolved, a photo's in its
/// folder alone, so a photo that is a link keeps its own name, and its folder is where the
/// link lies. On Windows only made absolute, since a resolved path there is a `\\?\` one
/// the file manager and the dialogs show as it is. One that cannot be resolved is kept
/// absolute.
pub fn resolved(path: &Path, folder: bool) -> PathBuf {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_owned());
    if cfg!(windows) {
        return absolute;
    }
    let canonical = |path: &Path| std::fs::canonicalize(path).ok();
    let resolved = if folder {
        canonical(&absolute)
    } else {
        let (parent, name) = (absolute.parent(), absolute.file_name());
        parent
            .zip(name)
            .and_then(|(parent, name)| Some(canonical(parent)?.join(name)))
    };
    resolved.unwrap_or(absolute)
}

/// A drop being looked at, on its thread, until the frame that takes what it opens; or the
/// files the system asked the app to open, Open With's, and the paths its command line
/// gave, which open as a drop of them would.
#[derive(Default)]
pub struct Drops {
    looking: Option<Receiver<Option<Dropped>>>,
}

impl Drops {
    /// Takes what was dropped onto the window this frame, unless `modal`, and looks at it
    /// ([`Drops::look`]).
    pub fn take(&mut self, ctx: &egui::Context, modal: bool) {
        let paths: Vec<PathBuf> = ctx.input(|i| {
            let files = i.raw.dropped_files.iter();
            files.map(|file| file.path().to_owned()).collect()
        });
        if paths.is_empty() || modal {
            return;
        }
        self.look(paths, ctx);
    }

    /// Looks at what `paths` open, on a thread of its own, in place of paths still being
    /// looked at, so a share that hangs never holds a frame; `ctx` is asked for a frame
    /// once they have been.
    pub fn look(&mut self, paths: Vec<PathBuf>, ctx: &egui::Context) {
        self.spawn(vec![paths], false, ctx);
    }

    /// Looks at the paths the app's command line and the hand-offs joined with it gave,
    /// `groups` each one's, as [`Drops::look`] does, each resolved first, a refusal's
    /// reason also on stderr, a folder never taking the place of raws handed in after it
    /// ([`joined`]).
    pub fn look_given(&mut self, groups: Vec<Vec<PathBuf>>, ctx: &egui::Context) {
        self.spawn(groups, true, ctx);
    }

    fn spawn(&mut self, groups: Vec<Vec<PathBuf>>, given: bool, ctx: &egui::Context) {
        let (answer, looking) = mpsc::channel();
        let ctx = ctx.clone();
        let looked = std::thread::Builder::new()
            .name("drop".into())
            .spawn(move || {
                let dropped = if given {
                    joined(&groups, kind)
                } else {
                    opens(&groups.concat(), kind)
                };
                let _ = answer.send(dropped);
                ctx.request_repaint();
            });
        match looked {
            Ok(_) => self.looking = Some(looking),
            Err(e) => tracing::warn!(target: "control", "drop not opened: no thread: {e}"),
        }
    }

    /// What the drop opens, once it has been looked at; nothing while it is, nor with none.
    pub fn looked(&mut self) -> Option<Dropped> {
        let answer = self.looking.as_ref()?.try_recv();
        match answer {
            Err(TryRecvError::Empty) => None,
            Ok(dropped) => {
                self.looking = None;
                dropped
            }
            Err(TryRecvError::Disconnected) => {
                self.looking = None;
                None
            }
        }
    }

    /// Whether a drop is being looked at.
    pub fn looking(&self) -> bool {
        self.looking.is_some()
    }

    /// Waits until the drop being looked at has been, and its thread has asked for its
    /// frame, so the next frame takes it and no ask comes in the middle of a test's run.
    #[cfg(test)]
    pub fn wait(&mut self) {
        if let Some(looking) = self.looking.take() {
            let (answer, again) = mpsc::channel();
            let _ = answer.send(looking.recv().ok().flatten());
            // The thread's end, its sender dropped, comes after its ask.
            let _ = looking.recv();
            self.looking = Some(again);
        }
    }
}

/// Whether something is being dragged over the window, with no modal dialog up.
pub fn hovering(ctx: &egui::Context, modal: bool) -> bool {
    !modal && ctx.input(|i| !i.raw.hovered_files.is_empty())
}

/// Outlines `rect`, where a drop opens, inside its edge, in the focus ring's colour and
/// the separators' width, over everything drawn there.
pub fn show_target(ctx: &egui::Context, rect: egui::Rect) {
    let t = theme(ctx);
    let layer = egui::LayerId::new(egui::Order::Foreground, egui::Id::unique("drop target"));
    let stroke = egui::Stroke::new(t.shape.separator_width, t.colors.focus_ring);
    ctx.layer_painter(layer)
        .rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Inside);
}

#[cfg(test)]
pub(crate) mod tests {
    use apertrust_ui::Theme;
    use egui_kittest::Harness;

    use super::*;

    /// A file as winit drops it: its path alone.
    #[derive(Debug)]
    pub(crate) struct File(pub PathBuf);

    impl egui::DroppedFile for File {
        fn path(&self) -> &Path {
            &self.0
        }

        fn bytes(&self) -> Result<Vec<u8>, String> {
            std::fs::read(&self.0).map_err(|e| e.to_string())
        }
    }

    fn paths(names: &[&str]) -> Vec<PathBuf> {
        names.iter().map(PathBuf::from).collect()
    }

    fn raws(first: &str, others: &[&str]) -> Dropped {
        let (first, others) = (PathBuf::from(first), paths(others));
        Dropped::Raws { first, others }
    }

    /// The kinds the disk would answer, by name: a folder where the name has no dot.
    pub(crate) fn by_name(path: &Path) -> Kind {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        match name.rsplit_once('.') {
            None => Kind::Folder,
            Some(_) => file_kind(path),
        }
    }

    #[test]
    fn the_first_folder_or_every_raw_from_the_first_on_opens() {
        let opens = |names: &[&str]| opens(&paths(names), by_name);
        assert_eq!(
            opens(&["/a/Wedding", "/a/Portraits"]),
            Some(Dropped::Folder(PathBuf::from("/a/Wedding")))
        );
        // Raws from two folders: all of them, together, in the order dropped.
        assert_eq!(
            opens(&[
                "/a/IMG_2.CR3",
                "/b/DSC_1.NEF",
                "/a/IMG_1.cr3",
                "/a/notes.txt"
            ]),
            Some(raws("/a/IMG_2.CR3", &["/b/DSC_1.NEF", "/a/IMG_1.cr3"]))
        );
        // A raw before a folder opens, and a folder before a raw.
        assert_eq!(
            opens(&["/a/notes.txt", "/a/IMG_1.CR3", "/a/Wedding"]),
            Some(raws("/a/IMG_1.CR3", &[]))
        );
        assert_eq!(
            opens(&["/a/Wedding", "/a/IMG_1.CR3"]),
            Some(Dropped::Folder(PathBuf::from("/a/Wedding")))
        );
        assert_eq!(opens(&[]), None);
    }

    /// Hand-offs joined while the app started (decision 89): raws join raws, the first
    /// active; raws after a folder replace it, never ignored for it, and a folder after
    /// raws replaces them, as each would once the window had shown; what opens nothing
    /// changes nothing.
    #[test]
    fn joined_hand_offs_open_as_they_would_one_after_another() {
        let absolute = |path: &str| std::path::absolute(path).expect("absolute");
        let joined = |each: &[&[&str]]| {
            let groups: Vec<Vec<PathBuf>> = each.iter().map(|names| paths(names)).collect();
            joined(&groups, by_name)
        };
        let raws = |first: &str, others: &[&str]| Dropped::Raws {
            first: absolute(first),
            others: others.iter().map(|other| absolute(other)).collect(),
        };
        let folder_then_raws = joined(&[&["/a/Wedding"], &["/a/IMG_1.CR3", "/b/DSC_2.NEF"]]);
        assert_eq!(
            folder_then_raws,
            Some(raws("/a/IMG_1.CR3", &["/b/DSC_2.NEF"]))
        );
        let raws_joined = joined(&[
            &["/a/IMG_1.CR3"],
            &["/a/IMG_2.CR3", "/a/IMG_1.CR3"],
            &["/a/notes.txt"],
        ]);
        assert_eq!(raws_joined, Some(raws("/a/IMG_1.CR3", &["/a/IMG_2.CR3"])));
        let raws_then_folder = joined(&[&["/a/IMG_1.CR3"], &["/a/Wedding"]]);
        let wedding = Dropped::Folder(absolute("/a/Wedding"));
        assert_eq!(raws_then_folder, Some(wedding));
        // One hand-off is a drop of its own: its first folder opens.
        let one = joined(&[&["/a/Wedding", "/a/IMG_1.CR3"]]);
        assert_eq!(one, Some(Dropped::Folder(absolute("/a/Wedding"))));
        let nothing = joined(&[&["/a/notes.txt"], &["/a/Home.aplibrary"]]);
        let path = absolute("/a/notes.txt");
        let why = Refusal::Unknown;
        assert_eq!(nothing, Some(Dropped::Refused { path, why }));
        assert_eq!(joined(&[]), None);
    }

    fn refused(path: &str, why: Refusal) -> Option<Dropped> {
        let path = PathBuf::from(path);
        Some(Dropped::Refused { path, why })
    }

    #[test]
    fn nothing_that_opens_is_refused_naming_the_first() {
        let opens = |names: &[&str]| opens(&paths(names), by_name);
        assert_eq!(
            opens(&["/a/notes.txt", "/a/IMG_1.jpg"]),
            refused("/a/notes.txt", Refusal::Unknown)
        );
        // No file is a library (decision 101): one is refused as any file that is no raw.
        assert_eq!(
            opens(&["/a/Home.aplibrary", "/a/notes.txt"]),
            refused("/a/Home.aplibrary", Refusal::Unknown)
        );
    }

    #[test]
    fn an_edit_opens_its_photo_as_the_raw_would() {
        let opens = |names: &[&str]| opens(&paths(names), by_name);
        assert_eq!(
            opens(&["/a/IMG_1.CR3.apxmp"]),
            Some(raws("/a/IMG_1.CR3", &[]))
        );
        // Edits and raws together, from two folders: each edit its photo, which is not
        // given twice.
        assert_eq!(
            opens(&[
                "/a/IMG_2.CR3.apxmp",
                "/a/IMG_1.CR3",
                "/a/IMG_2.CR3",
                "/b/DSC_1.NEF.apxmp",
                "/a/IMG_3.CR3.apxmp"
            ]),
            Some(raws(
                "/a/IMG_2.CR3",
                &["/a/IMG_1.CR3", "/b/DSC_1.NEF", "/a/IMG_3.CR3"]
            ))
        );
        assert_eq!(
            opens(&["/a/IMG_1.CR3", "/a/IMG_2.CR3", "/a/IMG_2.CR3.apxmp"]),
            Some(raws("/a/IMG_1.CR3", &["/a/IMG_2.CR3"]))
        );
        assert_eq!(
            opens(&["/a/notes.txt", "/a/IMG_1.CR3.apxmp", "/a/Wedding"]),
            Some(raws("/a/IMG_1.CR3", &[]))
        );
        // Only `.apxmp` in lower case is an edit, as the listing has it; nor is an edit of
        // a file that is no raw, nor of a folder.
        for name in [
            "/a/IMG_1.CR3.APXMP",
            "/a/notes.txt.apxmp",
            "/a/Wedding.apxmp",
        ] {
            assert_eq!(opens(&[name]), refused(name, Refusal::Unknown));
        }
    }

    #[test]
    fn an_edit_whose_photo_is_not_beside_it_is_refused_naming_the_edit() {
        // IMG_1.CR3 is not there: the disk answers as for anything else that is not. Compared
        // as paths, since Windows joins the photo's name with a backslash.
        let gone = |path: &Path| {
            if path == Path::new("/a/IMG_1.CR3") {
                Kind::Other
            } else {
                by_name(path)
            }
        };
        let opens = |names: &[&str]| opens(&paths(names), gone);
        assert_eq!(
            opens(&["/a/IMG_1.CR3.apxmp"]),
            refused("/a/IMG_1.CR3.apxmp", Refusal::NoPhoto)
        );
        assert_eq!(
            opens(&["/a/notes.txt", "/a/IMG_1.CR3.apxmp", "/a/IMG_1.CR3"]),
            refused("/a/IMG_1.CR3.apxmp", Refusal::NoPhoto)
        );
        // Anything that opens still does; of the refusals that say more, the first. After
        // a raw, an edit whose photo is gone stands for nothing.
        assert_eq!(
            opens(&["/a/IMG_1.CR3.apxmp", "/a/IMG_2.CR3"]),
            Some(raws("/a/IMG_2.CR3", &[]))
        );
        assert_eq!(
            opens(&["/a/IMG_2.CR3", "/a/IMG_1.CR3.apxmp", "/a/IMG_3.CR3"]),
            Some(raws("/a/IMG_2.CR3", &["/a/IMG_3.CR3"]))
        );
        assert_eq!(
            opens(&["/a/Home.aplibrary", "/a/IMG_1.CR3.apxmp"]),
            refused("/a/IMG_1.CR3.apxmp", Refusal::NoPhoto)
        );
    }

    #[test]
    fn open_s_answer_is_told_by_its_names_as_a_drop_of_it_would_be() {
        assert_eq!(
            chosen(&paths(&["/a/IMG_2.CR3", "/a/IMG_1.CR3"])),
            Some(raws("/a/IMG_2.CR3", &["/a/IMG_1.CR3"]))
        );
        // Typed into the dialog: files that are no raw, a library's name among them.
        assert_eq!(
            chosen(&paths(&["/a/Home.aplibrary"])),
            refused("/a/Home.aplibrary", Refusal::Unknown)
        );
        assert_eq!(
            chosen(&paths(&["/a/notes.txt"])),
            refused("/a/notes.txt", Refusal::Unknown)
        );
        assert_eq!(chosen(&[]), None);
    }

    /// Paths whose names are not UTF-8, as Linux's file systems allow (APFS refuses them,
    /// so none is made on disk): a raw's extension still makes a raw, the path kept as it
    /// is, and anything else is refused, the alert naming it as best it can.
    #[cfg(unix)]
    #[test]
    fn a_name_that_is_not_utf_8_is_told_by_its_extension() {
        use std::os::unix::ffi::OsStrExt;
        let path = |bytes: &[u8]| PathBuf::from(std::ffi::OsStr::from_bytes(bytes));
        let (raw, notes) = (path(b"/a/IMG_\xff.CR3"), path(b"/a/\xff.txt"));
        let first = raw.clone();
        let others = Vec::new();
        assert_eq!(chosen(&[raw]), Some(Dropped::Raws { first, others }));
        let why = Refusal::Unknown;
        assert_eq!(
            chosen(std::slice::from_ref(&notes)),
            Some(Dropped::Refused {
                path: notes.clone(),
                why
            })
        );
        assert_eq!(crate::file_name(&notes), "\u{fffd}.txt");
        // Not there: nothing to open.
        assert_eq!(kind(&path(b"/nowhere/\xff")), Kind::Other);
    }

    #[test]
    fn the_disk_says_what_a_path_is_and_a_folder_may_have_a_dot() {
        let folder = crate::settings::testing::Folder::new("drop-kinds");
        let dated = folder.0.join("Wedding 2026.10");
        std::fs::create_dir(&dated).expect("a folder");
        for name in ["IMG_1.CR3", "notes.txt", "Home.aplibrary"] {
            std::fs::write(folder.0.join(name), b"").expect("a file");
        }
        assert_eq!(kind(&dated), Kind::Folder);
        assert_eq!(kind(&folder.0.join("IMG_1.CR3")), Kind::Raw);
        assert_eq!(kind(&folder.0.join("notes.txt")), Kind::Other);
        assert_eq!(kind(&folder.0.join("Home.aplibrary")), Kind::Other);
        // Gone, or never there: nothing to open.
        assert_eq!(kind(&folder.0.join("IMG_2.CR3")), Kind::Other);
        // An edit beside its photo opens the photo; one whose photo is gone is refused.
        let edits = [
            folder.0.join("IMG_1.CR3.apxmp"),
            folder.0.join("IMG_2.CR3.apxmp"),
        ];
        for edit in &edits {
            std::fs::write(edit, b"").expect("an edit");
        }
        let first = folder.0.join("IMG_1.CR3");
        let others = Vec::new();
        assert_eq!(
            opens(&edits[..1], kind),
            Some(Dropped::Raws { first, others })
        );
        let (path, why) = (edits[1].clone(), Refusal::NoPhoto);
        assert_eq!(
            opens(&edits[1..], kind),
            Some(Dropped::Refused { path, why })
        );
        // A folder named like an edit opens as the folder it is, nothing named Album there.
        let album = folder.0.join("Album.apxmp");
        std::fs::create_dir(&album).expect("a folder");
        let dropped = opens(std::slice::from_ref(&album), kind);
        assert_eq!(dropped, Some(Dropped::Folder(album)));
        // Nor is one dropped after a raw taken for a photo beside it, nor a folder named
        // like a raw.
        let album = folder.0.join("Album.CR3.apxmp");
        std::fs::create_dir(&album).expect("a folder");
        let named = folder.0.join("Trip.CR3");
        std::fs::create_dir(&named).expect("a folder");
        let first = folder.0.join("IMG_1.CR3");
        let dropped = opens(&[first.clone(), album.clone(), named], kind);
        let others = Vec::new();
        assert_eq!(dropped, Some(Dropped::Raws { first, others }));
        // Even with a raw named as its photo beside it, which was not dropped.
        std::fs::write(folder.0.join("Album.CR3"), b"").expect("a raw");
        let first = folder.0.join("IMG_1.CR3");
        let dropped = opens(&[first.clone(), album], kind);
        let others = Vec::new();
        assert_eq!(dropped, Some(Dropped::Raws { first, others }));
    }

    /// The command line's paths open as a drop of them would, each resolved first: made
    /// absolute, `.` and `..` resolved, to the folder they lead to and a photo in it; on
    /// Windows only made absolute, which resolves them too. A photo that is a link keeps
    /// its name and the folder where it lies.
    #[test]
    fn the_command_line_s_paths_are_resolved_and_open_as_a_drop_s() {
        let scratch = crate::settings::testing::Folder::new("given-paths");
        let days = scratch.0.join("Days");
        std::fs::create_dir(&days).expect("a folder");
        let raw = scratch.0.join("IMG_1.CR3");
        std::fs::write(&raw, b"").expect("a raw");
        let folder = if cfg!(windows) {
            scratch.0.clone()
        } else {
            std::fs::canonicalize(&scratch.0).expect("there")
        };
        for dots in [scratch.0.join("."), days.join("..")] {
            let opened = given_opens(std::slice::from_ref(&dots), kind);
            let given = Some(Dropped::Folder(folder.clone()));
            assert_eq!(opened, given, "{}", dots.display());
        }
        let edit = days.join("..").join("IMG_1.CR3.apxmp");
        std::fs::write(&edit, b"").expect("an edit");
        for given in [days.join("..").join("IMG_1.CR3"), edit] {
            let (first, others) = (folder.join("IMG_1.CR3"), Vec::new());
            let opened = given_opens(&[given], kind);
            assert_eq!(opened, Some(Dropped::Raws { first, others }));
        }
        #[cfg(unix)]
        {
            let link = days.join("Best.CR3");
            std::os::unix::fs::symlink(&raw, &link).expect("a link");
            let (first, others) = (folder.join("Days").join("Best.CR3"), Vec::new());
            let opened = given_opens(&[link], kind);
            assert_eq!(opened, Some(Dropped::Raws { first, others }));
        }
        // A relative path, from the folder the app starts in.
        let relative = given_opens(&[PathBuf::from("nowhere.txt")], kind);
        let path = std::path::absolute("nowhere.txt").expect("absolute");
        let why = Refusal::Unknown;
        assert_eq!(relative, Some(Dropped::Refused { path, why }));
    }

    /// Of the command line's paths, those a drop leaves out that would open nothing on
    /// their own, which stderr names: a photo not there, an edit without its photo, a file
    /// that is no raw; never a raw or a folder that is there.
    #[test]
    fn what_opens_nothing_among_what_opens_is_named() {
        let gone = |path: &Path| {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if name == "IMG_1.CR3" || name == "IMG_9.CR3" {
                Kind::Other
            } else {
                by_name(path)
            }
        };
        let given = paths(&[
            "/a/IMG_2.CR3",
            "/a/IMG_1.CR3.apxmp",
            "/b/IMG_9.CR3",
            "/a/notes.txt",
            "/a/Wedding",
            "/b/IMG_3.CR3",
        ]);
        assert_eq!(
            opens(&given, gone),
            Some(raws("/a/IMG_2.CR3", &["/b/IMG_3.CR3"]))
        );
        let named = |path: &str, why| (PathBuf::from(path), why);
        assert_eq!(
            left_out(&given, gone),
            [
                named("/a/IMG_1.CR3.apxmp", Refusal::NoPhoto),
                named("/b/IMG_9.CR3", Refusal::Unknown),
                named("/a/notes.txt", Refusal::Unknown),
            ]
        );
    }

    /// What stderr says of a path the command line gave that opens nothing: what the disk
    /// said where nothing is there, else the refusal's reason.
    #[test]
    fn a_refusal_s_reason_on_stderr_names_the_path() {
        let edit = Path::new("/a/IMG_1.CR3.apxmp");
        let named = |path: &Path, said: &str| format!("{}: {said}", path.display());
        assert_eq!(
            refused_line(edit, Refusal::NoPhoto, None),
            named(edit, "its photo, IMG_1.CR3, is not in the same folder")
        );
        assert_eq!(
            refused_line(edit, Refusal::Unknown, None),
            named(edit, "only folders and raw photos open")
        );
        let gone = std::io::Error::from(std::io::ErrorKind::NotFound);
        let said = refused_line(edit, Refusal::Unknown, Some(&gone));
        assert_eq!(said, named(edit, &gone.to_string()));
    }

    /// A window in the default theme, the photo's surround over its left part and the
    /// panel's colour in a strip at its right, with a folder dragged over it where
    /// `hovered`, and the target drawn while something is.
    fn window(hovered: bool) -> Harness<'static> {
        let mut harness = Harness::builder()
            .with_size(egui::vec2(360.0, 200.0))
            .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
            .build_ui(|ui| {
                let ctx = ui.ctx().clone();
                let t = theme(&ctx);
                let all = ctx.content_rect();
                let side = egui::Rect::from_min_max(egui::pos2(all.max.x - 100.0, 0.0), all.max);
                let photo = egui::Rect::from_min_max(all.min, egui::pos2(side.min.x, all.max.y));
                ui.painter()
                    .rect_filled(photo, 0.0, t.colors.photo_surround);
                ui.painter().rect_filled(side, 0.0, t.colors.panel);
                if hovering(&ctx, false) {
                    show_target(&ctx, photo);
                }
            });
        if hovered {
            harness.input_mut().hovered_files.push(egui::HoveredFile {
                path: Some(PathBuf::from("/a/Wedding")),
                ..Default::default()
            });
        }
        // egui asks for frame after frame while something is dragged over the window.
        harness.run_steps(2);
        harness
    }

    #[test]
    fn something_dragged_over_the_window_outlines_where_it_opens() {
        let mut harness = window(true);
        assert!(hovering(&harness.ctx, false));
        // A modal dialog up: no outline.
        assert!(!hovering(&harness.ctx, true));
        harness.snapshot("drop-target");
        assert!(!hovering(&window(false).ctx, false));
    }

    #[test]
    fn a_drop_is_taken_unless_a_modal_dialog_is_up_and_opens_once_looked_at() {
        let folder = crate::settings::testing::Folder::new("drop-take");
        let raw = folder.0.join("IMG_1.CR3");
        std::fs::write(&raw, b"").expect("a raw");
        let mut harness = Harness::builder().build_ui_state(
            |ui, drops: &mut (Drops, bool)| drops.0.take(ui.ctx(), drops.1),
            (Drops::default(), true),
        );
        let drop = |harness: &mut Harness<'_, (Drops, bool)>, path: &Path| {
            let file: egui::DroppedFileHandle = std::sync::Arc::new(File(path.to_owned()));
            harness.input_mut().dropped_files.push(file);
            harness.step();
        };
        drop(&mut harness, &raw);
        assert!(!harness.state().0.looking());
        harness.state_mut().1 = false;
        drop(&mut harness, &raw);
        let drops = &mut harness.state_mut().0;
        drops.wait();
        let first = raw;
        assert_eq!(
            drops.looked(),
            Some(Dropped::Raws {
                first,
                others: Vec::new()
            })
        );
        assert_eq!(drops.looked(), None);
    }
}
