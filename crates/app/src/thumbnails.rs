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

//! The grid panel's thumbnails on the app's side (`docs/gui.md`, The grid panel): which to
//! ask the session for, and the textures made of those the mirror holds.
//!
//! - **What is asked for:** the photos whose cells the grid panel drew this frame that have
//!   no thumbnail at the cell's size, each once while it stays in sight, in the panel's
//!   order, as one `LibraryCommand::Thumbnails`. A photo that leaves the panel's sight and
//!   comes back is asked for again if its thumbnail has not come, so one the session's stack
//!   of 30 dropped comes in the end, and one still queued moves to the stack's top.
//! - **The size:** the cell's side in physical pixels, rounded up to one of [`SIZES`], so
//!   a change of the interface's scale asks again only past a step.
//! - **The textures** are a cache derived from the mirror (`docs/gui.md`, Derived caches):
//!   brought in line whenever the folder's generation changes, a texture made for each
//!   thumbnail that has come and dropped with its folder.

use std::collections::{HashMap, HashSet};

use apertrust_ui::components::CellPicture;
use eframe::egui;

use crate::folder::{Folder, FolderState};

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
    asked: HashSet<(String, u32)>,
}

impl Asking {
    /// The photos among `shown`, the cells drawn in the panel's order, to ask for at `size`
    /// now, for the folder of the open numbered `open`: those `has` says have no thumbnail
    /// at it and that have not been asked for since they came into sight.
    pub fn ask<'a>(
        &mut self,
        open: u64,
        shown: impl IntoIterator<Item = &'a str>,
        size: u32,
        has: impl Fn(&str) -> bool,
    ) -> Vec<String> {
        if open != self.open {
            self.open = open;
            self.asked.clear();
        }
        let shown: Vec<&str> = shown.into_iter().collect();
        // Out of sight, at another size or answered: asked for again should it be wanted.
        self.asked
            .retain(|(photo, at)| *at == size && shown.contains(&photo.as_str()) && !has(photo));
        shown
            .into_iter()
            .filter(|photo| !has(photo))
            .filter(|photo| self.asked.insert(((*photo).to_owned(), size)))
            .map(str::to_owned)
            .collect()
    }
}

/// The textures of the thumbnails the mirror holds.
#[derive(Default)]
pub struct Textures {
    /// The folder's generation they were brought in line with.
    generation: Option<u64>,
    /// The open they belong to.
    open: u64,
    /// Each thumbnail's texture, with the generation the thumbnail came at.
    textures: HashMap<(String, u32), (u64, egui::TextureHandle)>,
}

impl Textures {
    /// Brings the textures in line with `folder`, once its generation has changed: a
    /// texture for each thumbnail that has come, and none for another folder's.
    pub fn sync(&mut self, ctx: &egui::Context, folder: &FolderState) {
        if self.generation == Some(folder.generation()) {
            return;
        }
        self.generation = Some(folder.generation());
        if self.open != folder.latest() {
            self.open = folder.latest();
            self.textures.clear();
        }
        let Folder::Open(listing) = &folder.folder else {
            self.textures.clear();
            return;
        };
        self.textures.retain(|key, (generation, _)| {
            listing
                .thumbnails
                .get(key)
                .is_some_and(|kept| kept.generation == *generation && kept.image.is_some())
        });
        for (key, kept) in &listing.thumbnails {
            let Some(image) = &kept.image else { continue };
            if self.textures.contains_key(key) {
                continue;
            }
            let size = [image.width as usize, image.height as usize];
            let pixels = egui::ColorImage::from_rgba_unmultiplied(size, &image.pixels);
            let name = format!("thumbnail {} {}", key.1, key.0);
            let texture = ctx.load_texture(name, pixels, egui::TextureOptions::LINEAR);
            self.textures
                .insert(key.clone(), (kept.generation, texture));
        }
    }

    /// The picture to draw for `photo` at `size`: its texture at that size, else one at
    /// another size while the right one comes.
    pub fn picture(&self, photo: &str, size: u32) -> Option<CellPicture> {
        let texture = self
            .textures
            .get(&(photo.to_owned(), size))
            .or_else(|| {
                SIZES
                    .iter()
                    .rev()
                    .find_map(|&other| self.textures.get(&(photo.to_owned(), other)))
            })
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
        let none = |_: &str| false;

        assert_eq!(asking.ask(1, ["a", "b", "c"], 256, none), ["a", "b", "c"]);
        // The next frame shows the same: nothing more.
        assert!(asking.ask(1, ["a", "b", "c"], 256, none).is_empty());
        // Scrolled on: only the new one; then back, and the one that left is asked again.
        assert_eq!(asking.ask(1, ["b", "c", "d"], 256, none), ["d"]);
        assert_eq!(asking.ask(1, ["a", "b", "c"], 256, none), ["a"]);
        // One that has come is not asked for.
        let has_b = |photo: &str| photo == "b";
        assert!(asking.ask(1, ["a", "b", "c"], 256, has_b).is_empty());
        // Another size asks for all again, and so does another folder.
        assert_eq!(asking.ask(1, ["a", "c"], 512, none), ["a", "c"]);
        assert_eq!(asking.ask(2, ["a", "c"], 512, none), ["a", "c"]);
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

        let a = textures.picture("a", 256).expect("a's texture");
        assert_eq!(a.size, egui::vec2(4.0, 3.0));
        // A photo with none has no texture; another size stands in while one comes.
        assert!(textures.picture("b", 256).is_none());
        assert_eq!(textures.picture("a", 128), Some(a));
        assert_eq!(textures.len(), 1);

        // The folder closes: its textures go.
        folder.closing(2);
        textures.sync(&ctx, &folder);
        assert!(textures.picture("a", 256).is_none());
        assert_eq!(textures.len(), 0);
    }
}
