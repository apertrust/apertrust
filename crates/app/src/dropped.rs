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
//! - A folder opens as Open Folder… does, and raws as Open… does, from the first one's
//!   folder. Of several folders, or raws from several folders, the first opens.
//! - A library's or a project's file, `Name.aplibrary` or `Name.approject`, is refused with
//!   a message until libraries and projects open (Phase 5), and so is anything else that
//!   is neither a folder nor a raw the decoder reads.
//! - Nothing is taken while a modal dialog is up, the open dialog and an alert among them.
//! - Whether a path is a folder is asked of the disk on a thread of its own, never the
//!   interface's, as the recents are checked, so a slow share never holds a frame; what it
//!   opens opens a later frame ([`Drops`]). As with the recents' check, a share that hangs
//!   keeps that thread until it answers; a later drop starts a thread of its own and is
//!   the one taken, the hung one's answer, when it comes, going nowhere.
//! - While something is dragged over the window, the photo and the grid panel, where it
//!   opens, are outlined ([`show_target`]).
//!
//! The paths come from egui's input, which egui-winit fills from winit's dropped and
//! hovered files on macOS, Windows and X11; winit 0.30 has no drops on Wayland.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};

use apertrust_engine::session;
use apertrust_ui::theme;
use eframe::egui;

/// The extensions of a library's and a project's file (decision 46).
const LIBRARY_EXTENSIONS: [&str; 2] = ["aplibrary", "approject"];

/// What a dropped path is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Folder,
    /// A file whose extension the decoder reads.
    Raw,
    /// A library's or a project's file.
    Library,
    /// Anything else, or a path that cannot be asked about.
    Other,
}

/// What a drop opens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Dropped {
    /// A folder, opened as Open Folder… opens one.
    Folder(PathBuf),
    /// Raws from one folder, opened as Open… opens them: the first, and those dropped with
    /// it from its folder, in the order dropped.
    Raws {
        first: PathBuf,
        beside: Vec<PathBuf>,
    },
    /// Nothing that opens: the first path dropped, and why.
    Refused { path: PathBuf, why: Refusal },
}

/// Why nothing dropped opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// A library's or a project's file was among them; they open from Phase 5.
    Library,
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
    match std::fs::metadata(path) {
        Ok(metadata) if metadata.is_dir() => Kind::Folder,
        Ok(_) => file_kind(path),
        Err(_) => Kind::Other,
    }
}

/// What dropping `paths` opens, each asked with `kind` in order until the first that
/// opens: a folder; or a raw, with the raws dropped after it from its folder, told by
/// their names alone; or, where none opens, the first library's or project's file refused,
/// where one was among them, else the first path. None for nothing dropped.
pub fn opens(paths: &[PathBuf], mut kind: impl FnMut(&Path) -> Kind) -> Option<Dropped> {
    let mut library = None;
    for (i, path) in paths.iter().enumerate() {
        match kind(path) {
            Kind::Folder => return Some(Dropped::Folder(path.clone())),
            Kind::Raw => {
                let folder = path.parent();
                let beside = paths[i + 1..]
                    .iter()
                    .filter(|other| other.parent() == folder && named_raw(other))
                    .cloned()
                    .collect();
                let first = path.clone();
                return Some(Dropped::Raws { first, beside });
            }
            Kind::Library => library = library.or(Some(path)),
            Kind::Other => {}
        }
    }
    let (path, why) = match library {
        Some(path) => (path, Refusal::Library),
        None => (paths.first()?, Refusal::Unknown),
    };
    let path = path.clone();
    Some(Dropped::Refused { path, why })
}

/// What Open…'s answer `paths` opens, told by their names alone, the dialog having shown
/// only files: the first raw, or the first path refused as a drop of it would be, should
/// a name typed into the dialog be a library's file or another that is no raw. None for
/// no path.
pub fn chosen(paths: &[PathBuf]) -> Option<Dropped> {
    opens(paths, file_kind)
}

/// What a file is, told by its name alone.
fn file_kind(path: &Path) -> Kind {
    if named_raw(path) {
        Kind::Raw
    } else if has_extension(path, &LIBRARY_EXTENSIONS) {
        Kind::Library
    } else {
        Kind::Other
    }
}

/// A drop being looked at, on its thread, until the frame that takes what it opens.
#[derive(Default)]
pub struct Drops {
    looking: Option<Receiver<Option<Dropped>>>,
}

impl Drops {
    /// Takes what was dropped onto the window this frame, unless `modal`, and looks at it
    /// on a thread of its own, in place of a drop still being looked at; `ctx` is asked for
    /// a frame once it has been.
    pub fn take(&mut self, ctx: &egui::Context, modal: bool) {
        let paths: Vec<PathBuf> = ctx.input(|i| {
            let files = i.raw.dropped_files.iter();
            files.map(|file| file.path().to_owned()).collect()
        });
        if paths.is_empty() || modal {
            return;
        }
        let (answer, looking) = mpsc::channel();
        let ctx = ctx.clone();
        let looked = std::thread::Builder::new()
            .name("drop".into())
            .spawn(move || {
                let _ = answer.send(opens(&paths, kind));
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
    #[cfg(test)]
    pub fn looking(&self) -> bool {
        self.looking.is_some()
    }

    /// Waits until the drop being looked at has been, so the next frame takes it.
    #[cfg(test)]
    pub fn wait(&mut self) {
        if let Some(looking) = self.looking.take() {
            let (answer, again) = mpsc::channel();
            let _ = answer.send(looking.recv().ok().flatten());
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

    fn raws(first: &str, beside: &[&str]) -> Dropped {
        let (first, beside) = (PathBuf::from(first), paths(beside));
        Dropped::Raws { first, beside }
    }

    /// The kinds the disk would answer, by name: a folder where the name has no dot.
    fn by_name(path: &Path) -> Kind {
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
    fn the_first_folder_or_the_raws_of_the_first_raw_s_folder_open() {
        let opens = |names: &[&str]| opens(&paths(names), by_name);
        assert_eq!(
            opens(&["/a/Wedding", "/a/Portraits"]),
            Some(Dropped::Folder(PathBuf::from("/a/Wedding")))
        );
        // Raws from two folders: the first's, with those beside it, in the order dropped.
        assert_eq!(
            opens(&[
                "/a/IMG_2.CR3",
                "/b/DSC_1.NEF",
                "/a/IMG_1.cr3",
                "/a/notes.txt"
            ]),
            Some(raws("/a/IMG_2.CR3", &["/a/IMG_1.cr3"]))
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

    fn refused(path: &str, why: Refusal) -> Option<Dropped> {
        let path = PathBuf::from(path);
        Some(Dropped::Refused { path, why })
    }

    #[test]
    fn nothing_that_opens_is_refused_naming_the_first_or_the_first_library() {
        let opens = |names: &[&str]| opens(&paths(names), by_name);
        assert_eq!(
            opens(&["/a/notes.txt", "/a/IMG_1.jpg"]),
            refused("/a/notes.txt", Refusal::Unknown)
        );
        // The alert says libraries come later, so it names the library.
        assert_eq!(
            opens(&["/a/notes.txt", "/a/Trip.approject", "/a/Home.aplibrary"]),
            refused("/a/Trip.approject", Refusal::Library)
        );
        assert_eq!(
            opens(&["/a/Home.APLIBRARY"]),
            refused("/a/Home.APLIBRARY", Refusal::Library)
        );
    }

    #[test]
    fn open_s_answer_is_told_by_its_names_as_a_drop_of_it_would_be() {
        assert_eq!(
            chosen(&paths(&["/a/IMG_2.CR3", "/a/IMG_1.CR3"])),
            Some(raws("/a/IMG_2.CR3", &["/a/IMG_1.CR3"]))
        );
        // Typed into the dialog: a library's file, a file that is no raw.
        assert_eq!(
            chosen(&paths(&["/a/Home.aplibrary"])),
            refused("/a/Home.aplibrary", Refusal::Library)
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
        let beside = Vec::new();
        assert_eq!(chosen(&[raw]), Some(Dropped::Raws { first, beside }));
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
        assert_eq!(kind(&folder.0.join("Home.aplibrary")), Kind::Library);
        // Gone, or never there: nothing to open.
        assert_eq!(kind(&folder.0.join("IMG_2.CR3")), Kind::Other);
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
                beside: Vec::new()
            })
        );
        assert_eq!(drops.looked(), None);
    }
}
