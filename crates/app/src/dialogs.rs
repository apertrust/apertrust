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

//! The platform's file dialogs (`docs/gui.md`, The menu bar, File): File › Open Image…'s,
//! which asks for one raw, and File › Open Folder…'s, which asks for a folder.
//!
//! They are rfd's asynchronous dialogs, so the window keeps drawing and the engine keeps
//! rendering while one is open: a blocking dialog would run its modal loop inside an
//! eframe callback, holding the engine's submit gate until it closed (decision 18). On
//! macOS it is AppKit's open panel, as a sheet on the window; on Windows the system's
//! dialog, through COM on a thread of rfd's; on Linux the desktop's file chooser, through
//! the XDG desktop portal on a thread of rfd's, never GTK (decision 5). Where no portal
//! answers, rfd runs zenity, a program of its own, instead; with neither, the dialog cannot
//! be shown, and that reads as a cancel. The app polls the answer each frame, and a repaint
//! brings that frame when the answer comes.
//!
//! [`OpenDialog`] stands between, so the tests answer the dialog as a person would.

use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

use apertrust_engine::session;
use apertrust_ui::tr;
use eframe::egui;

/// What a dialog asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    /// One raw, for File › Open Image….
    Image,
    /// A folder, for File › Open Folder….
    Folder,
}

/// What the person chose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chosen {
    pub pick: Pick,
    pub path: PathBuf,
}

/// The dialog that asks for a raw or a folder to open: the platform's ([`Native`]), or a
/// test's.
pub trait OpenDialog {
    /// Whether it is asking now, between [`OpenDialog::ask`] and the answer.
    fn asking(&self) -> bool;

    /// Starts asking for `pick`, in `folder`, and returns at once.
    fn ask(&mut self, pick: Pick, folder: Option<&Path>);

    /// What the person chose, once they have; nothing while it asks, nor when they cancel
    /// it.
    fn chosen(&mut self) -> Option<Chosen>;
}

/// Asks `dialog` for `asked` when Open Image… or Open Folder… ran, starting in `folder()`,
/// unless it is asking already, and returns what the person chose once they have, for the
/// app to open; nothing while it asks, nor when they cancel.
pub fn open(
    dialog: &mut impl OpenDialog,
    asked: Option<Pick>,
    folder: impl FnOnce() -> Option<PathBuf>,
) -> Option<Chosen> {
    if let Some(pick) = asked
        && !dialog.asking()
    {
        dialog.ask(pick, folder().as_deref());
    }
    dialog.chosen()
}

/// Where the open dialog starts: in `open`, the folder open, else the folder of `photo`,
/// the photo open, or in `pictures`, the Pictures folder, when none is.
pub fn folder(
    open: Option<&Path>,
    photo: Option<&Path>,
    pictures: Option<&Path>,
) -> Option<PathBuf> {
    if let Some(open) = open {
        return std::path::absolute(open).ok();
    }
    // A raw named on the command line may be relative to where the app started.
    let photo = photo.and_then(|photo| std::path::absolute(photo).ok());
    photo
        .as_deref()
        .and_then(Path::parent)
        .or(pictures)
        .map(Path::to_owned)
}

/// The answer on its way from the platform's dialog, kept from frame to frame.
pub struct Asking(Pick, Pin<Box<dyn Future<Output = Option<rfd::FileHandle>>>>);

/// The platform's open dialog over the app's window, for one frame.
pub struct Native<'a> {
    pub window: &'a eframe::Frame,
    pub ctx: &'a egui::Context,
    /// The answer on its way, while the dialog asks.
    pub asking: &'a mut Option<Asking>,
}

impl OpenDialog for Native<'_> {
    fn asking(&self) -> bool {
        self.asking.is_some()
    }

    fn ask(&mut self, pick: Pick, folder: Option<&Path>) {
        let mut dialog = rfd::AsyncFileDialog::new().set_parent(self.window);
        let answer: Pin<Box<dyn Future<Output = Option<rfd::FileHandle>>>> = match pick {
            Pick::Image => {
                // The decoder's extensions are upper case; a portal's chooser may match case.
                let extensions: Vec<String> = session::raw_extensions()
                    .iter()
                    .flat_map(|e| [e.to_string(), e.to_lowercase()])
                    .collect();
                dialog = dialog
                    .set_title(tr(self.ctx, "open-title"))
                    .add_filter(tr(self.ctx, "open-filter"), &extensions);
                if let Some(folder) = folder {
                    dialog = dialog.set_directory(folder);
                }
                Box::pin(dialog.pick_file())
            }
            Pick::Folder => {
                dialog = dialog.set_title(tr(self.ctx, "open-folder-title"));
                if let Some(folder) = folder {
                    dialog = dialog.set_directory(folder);
                }
                Box::pin(dialog.pick_folder())
            }
        };
        // On macOS the sheet is up once this returns; elsewhere rfd's thread shows it.
        *self.asking = Some(Asking(pick, answer));
    }

    fn chosen(&mut self) -> Option<Chosen> {
        let Asking(pick, answer) = self.asking.as_mut()?;
        let pick = *pick;
        let waker = Waker::from(Arc::new(Repaint(self.ctx.clone())));
        let Poll::Ready(file) = answer.as_mut().poll(&mut Context::from_waker(&waker)) else {
            return None;
        };
        *self.asking = None;
        file.map(|file| Chosen {
            pick,
            path: file.path().to_owned(),
        })
    }
}

/// Asks for a frame when the dialog's answer comes, so the app looks at it.
struct Repaint(egui::Context);

impl Wake for Repaint {
    fn wake(self: Arc<Self>) {
        self.0.request_repaint();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_open_dialog_starts_in_the_open_folder_the_photo_s_folder_or_else_in_pictures() {
        let pictures = Path::new("pictures");
        let here = std::env::current_dir().expect("a working folder");
        // A raw from the command line, relative to where the app started.
        assert_eq!(
            folder(None, Some(Path::new("raws/IMG_2041.CR3")), Some(pictures)),
            Some(here.join("raws"))
        );
        assert_eq!(
            folder(None, Some(&here.join("IMG_2041.CR3")), Some(pictures)),
            Some(here.clone())
        );
        assert_eq!(
            folder(None, None, Some(pictures)),
            Some(PathBuf::from("pictures"))
        );
        assert_eq!(folder(None, None, None), None);
        // The folder open comes first, a photo from elsewhere notwithstanding.
        assert_eq!(
            folder(
                Some(&here.join("Wedding")),
                Some(Path::new("/other/IMG.CR3")),
                Some(pictures)
            ),
            Some(here.join("Wedding"))
        );
    }
}
