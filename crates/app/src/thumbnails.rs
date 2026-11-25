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

//! The Browser's thumbnails on the app's side (`docs/gui.md`, The Browser): which to
//! ask the session for, and the textures made of those the mirror holds.
//!
//! - **What is asked for:** the photos whose cells the Browser drew this frame that have
//!   no thumbnail at the cell's size, each once while it stays in sight, in the panel's
//!   order, as one `LibraryCommand::Thumbnails`, or `LibraryCommand::PhotoThumbnails` by
//!   their paths for photos opened alone. A photo that leaves the panel's sight and comes
//!   back is asked for again if its thumbnail has not come, so one the session's stack of
//!   30 dropped comes in the end, and one still queued moves to the stack's top.
//! - **The size:** the cell's side in physical pixels, rounded up to one of [`SIZES`], so
//!   a change of the interface's scale asks again only past a step.
//! - **The textures** are a cache derived from the mirror (`docs/gui.md`, Derived caches):
//!   brought in line whenever the folder's generation changes, a texture made for each
//!   thumbnail that has come, by its cell's key ([`FolderState::key`]), and dropped with
//!   its folder, or with a photo alone as it leaves them.

use std::collections::{HashMap, HashSet};

use apertrust_engine::session::LibraryCommand;
use apertrust_ui::components::CellPicture;
use eframe::egui;

use crate::folder::{CellKey, FolderState};

/// The sizes thumbnails are asked for at, in pixels on their longer side.
pub const SIZES: [u32; 4] = [128, 256, 512, 1024];

/// The size to ask for a cell `side` points wide at `pixels_per_point`: the smallest of
/// [`SIZES`] that covers it, the largest past them.
pub fn size_for(side: f32, pixels_per_point: f32) -> u32 {
    let pixels = (side * pixels_per_point).ceil();
    SIZES
        .into_iter()
        .find(|&size| size as f32 >= pixels)
        .unwrap_or(SIZES[SIZES.len() - 1])
}

/// What has been asked for, so a photo in sight is asked for once.
#[derive(Debug, Default)]
pub struct Asking {
    /// The open or close the asks belong to.
    open: u64,
    /// The photos asked for at a size since they came into sight, with no answer yet.
    asked: HashSet<(CellKey, u32)>,
}

impl Asking {
    /// The photos among `shown`, the cells drawn in the panel's order, by their keys, to ask
    /// for at `size` now, for the folder of the open numbered `open`: those `has` says have
    /// no thumbnail at it and that have not been asked for since they came into sight.
    pub fn ask(
        &mut self,
        open: u64,
        shown: Vec<CellKey>,
        size: u32,
        has: impl Fn(&CellKey) -> bool,
    ) -> Vec<CellKey> {
        if open != self.open {
            self.open = open;
            self.asked.clear();
        }
        // Out of sight, at another size or answered: asked for again should it be wanted.
        self.asked
            .retain(|(photo, at)| *at == size && shown.contains(photo) && !has(photo));
        shown
            .into_iter()
            .filter(|photo| !has(photo))
            .filter(|photo| self.asked.insert((photo.clone(), size)))
            .collect()
    }

    /// What to ask the session for, for the cells drawn this frame, `shown`, the photos'
    /// places in `folder`'s listing in the panel's order, at `size` ([`Asking::ask`]): the
    /// open folder's photos by name, the photos alone by path; none where none is wanted.
    pub fn command(
        &mut self,
        folder: &FolderState,
        shown: &[usize],
        size: u32,
    ) -> Option<LibraryCommand> {
        let keys = shown.iter().map(|&i| folder.key(i)).collect();
        let has = |key: &CellKey| folder.thumbnail(key, size).is_some();
        let wanted = self.ask(folder.latest(), keys, size, has);
        let (mut photos, mut paths) = (Vec::new(), Vec::new());
        for key in wanted {
            match key {
                CellKey::Name(name) => photos.push(name),
                CellKey::Path(path) => paths.push(path),
            }
        }
        if !paths.is_empty() {
            Some(LibraryCommand::PhotoThumbnails { paths, size })
        } else if !photos.is_empty() {
            Some(LibraryCommand::Thumbnails { photos, size })
        } else {
            None
        }
    }
}

/// The textures of the thumbnails the mirror holds.
#[derive(Default)]
pub struct Textures {
    /// The folder's generation they were brought in line with.
    generation: Option<u64>,
    /// The open they belong to.
    open: u64,
    /// Each thumbnail's texture, by its cell's key and size, with the generation the
    /// thumbnail came at.
    textures: HashMap<(CellKey, u32), (u64, egui::TextureHandle)>,
}

impl Textures {
    /// Brings the textures in line with `folder`, once its generation has changed: a
    /// texture for each thumbnail that has come, and none for another folder's or for
    /// photos alone no longer open.
    pub fn sync(&mut self, ctx: &egui::Context, folder: &FolderState) {
        if self.generation == Some(folder.generation()) {
            return;
        }
        self.generation = Some(folder.generation());
        // Another folder's names are other photos; a path is the same photo whatever opened
        // it, so a photo alone kept among the next set keeps its texture.
        if self.open != folder.latest() {
            self.open = folder.latest();
            (self.textures).retain(|(key, _), _| matches!(key, CellKey::Path(_)));
        }
        self.textures.retain(|(key, size), (generation, _)| {
            folder
                .thumbnail(key, *size)
                .is_some_and(|kept| kept.generation == *generation && kept.image.is_some())
        });
        for (key, size, kept) in folder.thumbnails() {
            let Some(image) = &kept.image else { continue };
            let key = (key, size);
            if self.textures.contains_key(&key) {
                continue;
            }
            let pixels = [image.width as usize, image.height as usize];
            let pixels = egui::ColorImage::from_rgba_unmultiplied(pixels, &image.pixels);
            let name = format!("thumbnail {size} {:?}", key.0);
            let texture = ctx.load_texture(name, pixels, egui::TextureOptions::LINEAR);
            self.textures.insert(key, (kept.generation, texture));
        }
    }

    /// The picture to draw for the photo whose cell's key is `key` at `size`: its texture at
    /// that size, else one at another size while the right one comes.
    pub fn picture(&self, key: &CellKey, size: u32) -> Option<CellPicture> {
        let at = |size: u32| self.textures.get(&(key.clone(), size));
        let texture = at(size)
            .or_else(|| SIZES.iter().rev().find_map(|&other| at(other)))
            .map(|(_, texture)| texture)?;
        Some(CellPicture {
            texture: texture.id(),
            size: texture.size_vec2(),
        })
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.textures.len()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use apertrust_engine::session::library::{EditState, Photo, Thumbnail};
    use apertrust_engine::session::{Event, LibraryEvent};

    use super::*;

    /// The key of a folder's photo of this name.
    fn name(photo: &str) -> CellKey {
        CellKey::Name(photo.to_owned())
    }

    fn names(photos: &[&str]) -> Vec<CellKey> {
        photos.iter().map(|photo| name(photo)).collect()
    }

    #[test]
    fn a_cell_is_asked_for_at_the_size_that_covers_it() {
        // A Medium cell, 120 points, at 1, 1.5 and 2 pixels a point, and beyond the sizes.
        assert_eq!(size_for(120.0, 1.0), 128);
        assert_eq!(size_for(120.0, 1.5), 256);
        assert_eq!(size_for(120.0, 2.0), 256);
        assert_eq!(size_for(120.0, 2.2), 512);
        assert_eq!(size_for(120.0, 10.0), 1024);
        // A zoom within a step asks for nothing new.
        assert_eq!(size_for(120.0, 1.6), size_for(120.0, 2.0));
    }

    #[test]
    fn a_photo_in_sight_is_asked_for_once_and_again_after_it_comes_back() {
        let mut asking = Asking::default();
        let none = |_: &CellKey| false;

        assert_eq!(
            asking.ask(1, names(&["a", "b", "c"]), 256, none),
            names(&["a", "b", "c"])
        );
        // The next frame shows the same: nothing more.
        assert!(asking.ask(1, names(&["a", "b", "c"]), 256, none).is_empty());
        // Scrolled on: only the new one; then back, and the one that left is asked again.
        assert_eq!(
            asking.ask(1, names(&["b", "c", "d"]), 256, none),
            names(&["d"])
        );
        assert_eq!(
            asking.ask(1, names(&["a", "b", "c"]), 256, none),
            names(&["a"])
        );
        // One that has come is not asked for.
        let has_b = |photo: &CellKey| *photo == name("b");
        assert!(
            asking
                .ask(1, names(&["a", "b", "c"]), 256, has_b)
                .is_empty()
        );
        // Another size asks for all again, and so does another folder.
        assert_eq!(
            asking.ask(1, names(&["a", "c"]), 512, none),
            names(&["a", "c"])
        );
        assert_eq!(
            asking.ask(2, names(&["a", "c"]), 512, none),
            names(&["a", "c"])
        );
    }

    /// A photo's thumbnail at a size: its picture's width and height, or none.
    type Came<'a> = (&'a str, u32, Option<(u32, u32)>);

    /// A folder open with these photos, each with the thumbnail that came.
    fn folder_with(thumbnails: &[Came<'_>]) -> FolderState {
        let mut folder = FolderState::default();
        folder.opening(PathBuf::from("/photos"), 1);
        let photos = thumbnails
            .iter()
            .map(|(name, ..)| Photo {
                name: (*name).to_owned(),
                size: 1,
                modified: None,
                edit: EditState::None,
                rating: Default::default(),
            })
            .collect();
        for event in [
            LibraryEvent::Opened {
                open: 1,
                name: "photos".to_owned(),
                folder: PathBuf::from("/photos"),
            },
            LibraryEvent::Listed {
                open: 1,
                photos,
                done: true,
            },
        ] {
            folder.apply(&Event::Library(event));
        }
        for &(photo, size, image) in thumbnails {
            let image = image.map(|(width, height)| Thumbnail {
                width,
                height,
                pixels: vec![128; (width * height * 4) as usize].into(),
            });
            folder.apply(&Event::Library(LibraryEvent::Thumbnail {
                open: 1,
                photo: photo.to_owned(),
                size,
                image,
            }));
        }
        folder
    }

    #[test]
    fn textures_follow_the_mirror_s_thumbnails_and_go_with_the_folder() {
        let ctx = egui::Context::default();
        let mut folder = folder_with(&[("a", 256, Some((4, 3))), ("b", 256, None)]);
        let mut textures = Textures::default();

        textures.sync(&ctx, &folder);

        let a = textures.picture(&name("a"), 256).expect("a's texture");
        assert_eq!(a.size, egui::vec2(4.0, 3.0));
        // A photo with none has no texture; another size stands in while one comes.
        assert!(textures.picture(&name("b"), 256).is_none());
        assert_eq!(textures.picture(&name("a"), 128), Some(a));
        assert_eq!(textures.len(), 1);

        // The folder closes: its textures go.
        folder.closing(2);
        textures.sync(&ctx, &folder);
        assert!(textures.picture(&name("a"), 256).is_none());
        assert_eq!(textures.len(), 0);
    }

    #[test]
    fn a_rendered_thumbnail_event_replaces_the_embedded_cell_texture() {
        let ctx = egui::Context::default();
        let mut folder = folder_with(&[("a", 256, Some((4, 3)))]);
        let mut textures = Textures::default();
        textures.sync(&ctx, &folder);
        let embedded = textures.picture(&name("a"), 256).unwrap();
        let generation = folder.generation();
        let rendered = Thumbnail {
            width: 4,
            height: 3,
            pixels: [220, 40, 10, 255].repeat(12).into(),
        };
        folder.apply(&Event::Library(LibraryEvent::Thumbnail {
            open: 1,
            photo: "a".to_owned(),
            size: 256,
            image: Some(rendered.clone()),
        }));
        assert!(folder.generation() > generation);
        assert_eq!(
            folder.thumbnail(&name("a"), 256).unwrap().image.as_ref(),
            Some(&rendered)
        );
        textures.sync(&ctx, &folder);
        let updated = textures.picture(&name("a"), 256).unwrap();
        assert_ne!(updated.texture, embedded.texture);
        assert_eq!(updated.size, embedded.size);
        assert_eq!(textures.len(), 1);
        textures.sync(&ctx, &folder);
        assert_eq!(textures.picture(&name("a"), 256), Some(updated));
        // A completion from an earlier folder cannot replace this cell.
        folder.apply(&Event::Library(LibraryEvent::Thumbnail {
            open: 0,
            photo: "a".to_owned(),
            size: 256,
            image: None,
        }));
        textures.sync(&ctx, &folder);
        assert_eq!(textures.picture(&name("a"), 256), Some(updated));
    }

    /// The Browser asks for a folder's photos by name, and for photos opened alone by
    /// path, two of the same name apart, each once while in sight; what comes is drawn in
    /// its cell, and goes with the photos alone.
    #[test]
    fn photos_alone_are_asked_for_by_path_and_drawn_in_their_cells() {
        let ctx = egui::Context::default();
        let mut asking = Asking::default();
        let folder = folder_with(&[("a", 256, None), ("b", 128, None)]);
        // A's has come at the size, as none, b's at another: only b is asked for.
        let asked = asking.command(&folder, &[0, 1], 256);
        let photos = vec!["b".to_owned()];
        assert_eq!(
            asked,
            Some(LibraryCommand::Thumbnails { photos, size: 256 })
        );

        let (x, y) = (PathBuf::from("/a/x.ARW"), PathBuf::from("/b/x.ARW"));
        let mut folder = FolderState::default();
        folder.alone(2, vec![x.clone(), y.clone()]);
        let asked = asking.command(&folder, &[1, 0], 512);
        let paths = vec![y.clone(), x.clone()];
        assert_eq!(
            asked,
            Some(LibraryCommand::PhotoThumbnails { paths, size: 512 })
        );
        assert_eq!(asking.command(&folder, &[1, 0], 512), None);
        folder.apply(&Event::Library(LibraryEvent::PhotoThumbnail {
            open: 2,
            path: y.clone(),
            size: 512,
            image: Some(Thumbnail {
                width: 4,
                height: 3,
                pixels: vec![128; 48].into(),
            }),
        }));
        let mut textures = Textures::default();
        textures.sync(&ctx, &folder);
        let drawn = textures.picture(&folder.key(1), 512).expect("y's texture");
        assert_eq!(drawn.size, egui::vec2(4.0, 3.0));
        assert!(textures.picture(&folder.key(0), 512).is_none());

        // Opened alone again with y first, by command 3, which cancelled the reads not yet
        // sent: y's texture stays, and x alone is asked for again; then without y, its
        // texture goes.
        folder.alone(3, vec![y.clone(), x.clone()]);
        textures.sync(&ctx, &folder);
        assert_eq!(textures.picture(&folder.key(0), 512), Some(drawn));
        let again = Some(LibraryCommand::PhotoThumbnails {
            paths: vec![x.clone()],
            size: 512,
        });
        assert_eq!(asking.command(&folder, &[0, 1], 512), again);
        folder.alone(4, vec![x]);
        textures.sync(&ctx, &folder);
        assert_eq!(textures.len(), 0);
        assert!(textures.picture(&CellKey::Path(y), 512).is_none());
    }
}
