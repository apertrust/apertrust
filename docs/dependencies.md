# Dependencies

How a dependency is chosen and tracked, the main libraries Apertrust builds on and why, and its licence.

## Choosing one

- **Rust first.** No C or C++ library is compiled into Apertrust or shipped with it unless no Rust option exists (architecture decision 5). Libraries the platform provides, loaded at run time, are outside the rule; AGENTS.md lists them.
- **A compatible licence.** Every licence must be compatible with GPLv3. `cargo deny check` checks it against `deny.toml`; run it before you add a dependency.
- **Maintained.** Prefer a crate with a history of releases and an active maintainer.
- **Real.** Check that the crate exists on [crates.io](https://crates.io) under that exact name before you add it. AI tools sometimes suggest crates that do not exist, or names close to real ones.
- **Explained.** Say in the pull request why the dependency is needed and which others you considered.
- **One version.** Shared versions live in the root `Cargo.toml`, under `[workspace.dependencies]`.

## Tracking them

- Dependabot proposes updates to the crates and to the GitHub Actions each week (`.github/dependabot.yml`).
- `CREDITS.md` lists every component from `Cargo.lock`. Run `tools/credits.sh` after a dependency changes; CI runs `tools/credits.sh --check`.

## Build tools

Used to build Apertrust, never compiled into it or shipped with it, so decision 5 does not reach them; their licences are still GPLv3-compatible.

- **cargo-packager** 0.11.8 (MIT or Apache-2.0), installed with `cargo install --locked cargo-packager --version 0.11.8`, which `tools/package.sh` checks: it builds every package from `packaging/Packager.toml` (decision 68; [packaging.md](packaging.md)). Rejected: a script per platform, cargo-dist, cargo-bundle and Tauri's bundler (the decision's options).
- **cargo-generate-rpm** 0.21.1 (MIT), installed on Linux with `cargo install --locked cargo-generate-rpm --version 0.21.1`, which `tools/package.sh` checks: it builds the `.rpm` from `packaging/linux/rpm.toml`, which cargo-packager does not make, in Rust with the `rpm` crate, so packaging needs neither rpmbuild nor a spec file, and finds the libraries the binaries link with `ldd` (decision 68; [packaging.md](packaging.md#linux)). Rejected: rpmbuild with a spec file (rpm's own build tools on the machine that packages, and another description of the app).
- **ds_store** 1.3.3 and **mac_alias** 2.2.3 (both MIT), Python packages that `packaging/macos/dmg/generate.py` uses to write the disk image's `layout.DS_Store` from nothing, run by hand only when the window's layout changes; the file is committed, so packaging never needs Python, and nothing of them is shipped ([packaging.md](packaging.md)).
- **winresource** 0.1.34 (MIT), a build dependency of the app on Windows only, without its `toml` feature: it writes the icon and version resource script and runs the Windows SDK's own `rc.exe`, so nothing but MSVC's toolchain is needed. Build dependencies are not shipped, so `CREDITS.md` does not list it.
- **resvg** and **png** render the app icon's files in the app's `icons` example (`tools/build-icons.sh`) and the disk image's background in its `dmg_background` example (`tools/build-dmg-background.sh`), both already the interface's and the export's dependencies.

## The main libraries

The choices behind the libraries that shape the image, the system clipboard's and the session protocol's serialization, each under decision 5's rule.

### Raw decoding: rawler

Raw decoding uses rawler, a decoder in Rust from the dnglab project (decision 23).

- **Against RawSpeed,** darktable's decoder, on 40 raw.pixls.us samples: rawler decoded 36 and RawSpeed 29. Wherever both decode a mosaic, the raw values are identical, apart from Sony's lossy ARWs, where both dither Sony's tone curve but seed the dither from different bits of each row.
- **Canon CR3:** rawler reads it; RawSpeed cannot, and darktable uses LibRaw for it.
- **Golden tests** of ports take darktable's own renders as input, so the decoder adds nothing to their ΔE.
- **Licence and build:** rawler is LGPL-2.1, compatible with GPLv3, and pure Rust, so it builds wherever Rust does, Windows with MSVC included.
- **Metadata:** it also reads EXIF, ratings and lenses from maker notes; the metadata spike checks how far that goes.
- **A corpus test** pins its output on the 40 samples, so each rawler update is deliberate.
- Rejected: RawSpeed through FFI (a C++ build with CMake, pugixml and OpenMP, no MSVC, no CR3) and LibRaw (C++).

### Lens correction: lensfun's database and models

Distortion, chromatic aberration and vignetting corrections use lensfun's lens database and correction models. Under decision 5 they are not called through FFI: lensfun's lookup and interpolation are ported to Rust, and its models run in a Slang kernel, in Phase 3.

- **It is what darktable's lens correction module uses,** so corrections match darktable in the golden tests and the same database applies.
- **The database updates on its own,** so new lenses need no app release. Its licence, CC BY-SA, and the library's, LGPL, are both compatible with GPLv3.
- **Embedded corrections come first.** Correction data some cameras store in their raws is supported alongside lensfun, as in darktable's lens module, including how it chooses between the two sources; the tool lets the user pick the source (`docs/ports/lens.md`).
- **Where the embedded data comes from:** Sony's raw IFD, Fujifilm's FujiIFD and maker note, OM System's maker note, and a DNG's OpcodeList3. `apertrust-raw` reads them over rawler's public TIFF and maker-note parsers, with darktable's checks, into the image context's `Camera::lens_correction`, as darktable stores them; the operation computes everything from them (decision 25).
- **Why not lensfun itself:** it needs GLib, and darktable's GPU path asks it for a corrected coordinate for every pixel on the CPU, then uploads the map, about 1.4 GB per render at 60 MP. Porting the lookup keeps the database, and the models on the GPU keep the pipeline GPU-resident and differentiable.

### Colour management: moxcms, validated against lcms2

The pure-Rust moxcms is used behind a small colour-management interface in the engine. lcms2 is the reference it is validated against in CI, never shipped (decision 5).

- In a GPU-resident pipeline, per-pixel conversion runs in the engine's own kernels. The library parses ICC profiles (camera input, display, export) and builds transforms, so correctness on real-world profiles matters more than speed.
- Pure Rust means no C dependency on three platforms or in Flatpak.
- moxcms is pre-1.0 with breaking releases; lcms2 has decades of production use, darktable's included.
- **The validation gate:** CI runs both libraries on a corpus of real profiles (camera, calibrated display, printer, ICC v4 LUT-based) and compares the transforms. Golden tests catch any difference against darktable, which uses lcms2. Profiles where moxcms fails are fixed upstream.
- Both licences (BSD or Apache, and MIT) are compatible with GPLv3.

### Metadata: XMP by Apertrust's own layer, EXIF by rawler

The app reads and writes XMP and tags (decision 50). It needs EXIF reading, maker-note lens identification for lensfun, embedded raw previews, XMP sidecars read and written (its own edits, ratings, labels and keywords, including those in darktable's XMP files) and metadata in exported files. darktable does all of it through exiv2, a C++ library that decision 5 keeps out.

**XMP** is a conformant layer of Apertrust's own, `apertrust-library`'s `xmp` module, next to `sidecar`, on quick-xml. Its tree, its reader and its writer are [library.md](library.md#the-xmp-layer)'s; here, what it takes on and leaves out.

- **It refuses,** naming the line, what lies outside XMP's subset: typed RDF nodes, `parseType="Literal"` and `"Collection"`, and an unqualified element or attribute.
- **It writes exiv2's compact form:** the declarations and the simple, unqualified properties as attributes, the rest as elements, a struct as a nested `rdf:Description`, laid out as the committed `.apxmp` fixtures are. A sidecar has no packet wrapper, as darktable's has none. It writes through the safe write, and not at all when nothing changed.
- **Checked** against darktable's own `.xmp` files and packets other tools wrote, cross-checked locally with ExifTool's and exiv2's command-line tools, as the golden harness runs darktable-cli; neither is a dependency.
- **Planned:** the standard schemas' typed handling (`dc:subject`, `xmp:Rating`, `xmp:Label`, `lr:hierarchicalSubject`) and darktable's namespace with its history, with ratings and keywords in Phase 3 (the layer reads and keeps them as any property meanwhile); embedding in exports (JPEG's APP1 with Extended XMP, PNG's iTXt, DNG's tag 700) with Export's metadata.

**EXIF** is read by `apertrust-raw` from rawler's metadata (decisions 24 and 56), which also gives the camera's identification ([plugins.md](plugins.md#the-cameras-names)), ratings and lens names. A pure-Rust writer for exports is chosen with Export's metadata; exiv2 would need an exception to decision 5.

**The metadata spike** round-trips the XMP layer over darktable's `.xmp` files and packets other tools wrote, and reads its output back with ExifTool and exiv2. It passes when every property and unknown namespace is kept; ratings, labels and keywords read back the same in ExifTool, exiv2, darktable and commercial raw editors; and lens identification and orientation from rawler match exiv2's. Its fallback is exiv2, which would need an exception to decision 5.

**Not used:** exiftool-rs, ExifTool's Perl run by an interpreter in Rust that compiles in PCRE2 and zlib; Adobe's XMP Toolkit and rexiv2, both C++; xmpkit, young, with one maintainer and unproven conformance; typst's `xmp_writer`, which only writes; RDF/XML crates such as oxrdfxml, which give triples without XMP's order, languages or canonical form.

### Images other than raws, planned

Decision 111: Apertrust opens the images darktable reads, each through a Rust decoder in `apertrust-raw`, added with its tier and checked with `cargo deny check` then. The versions are those current on crates.io when the decision was recorded.

| Tier | Format | Crate | Licence | Note |
| --- | --- | --- | --- | --- |
| 1 | JPEG | jpeg-decoder 0.3.2 or zune-jpeg 0.5.15 | MIT OR Apache-2.0; MIT OR Apache-2.0 OR Zlib | Both in the tree already (the export's tests; `image` through rawler and eframe); the one nearer libjpeg-turbo's decode, darktable's, is taken. EXIF and XMP through rawler's `formats::jfif` |
| 1 | ICC profiles | moxcms | BSD-3-Clause OR Apache-2.0 | Decision 26's, already in the tree through `image`; added directly with the display colour's first part |
| 2 | TIFF | tiff 0.11.3 | MIT | In the tree through `image` |
| 2 | PNG | png 0.18.1 | MIT OR Apache-2.0 | Already a dependency |
| 3 | WebP | image-webp 0.2.4 | MIT OR Apache-2.0 | |
| 3 | JPEG XL | jxl-oxide 0.12.6 | MIT OR Apache-2.0 | In the tree through rawler |
| 3 | AVIF | rav1d 1.1.0, avif-parse 2.1.0 | BSD-2-Clause; MPL-2.0 | rav1d with its `asm` feature off, so no assembler or C toolchain builds it; libavif's YUV to RGB conversion ported |
| 4 | OpenEXR | exr 1.74.3 | BSD-3-Clause | No unsafe code |
| 5 | JPEG 2000 | hayro-jpeg2000 0.5.0 | Apache-2.0 OR MIT | |
| 5 | GIF | gif 0.14.2 | MIT OR Apache-2.0 | |

Radiance HDR, PFM, binary PNM, QOI and BMP are read by Apertrust's own code, ports of darktable's loaders where it has one.

- **HEIF and HEIC,** tier 6, choose their decoder after a spike comparing the pure-Rust `heic` crate with libheif on phones' photos: `heic`, AGPL-3.0-only (or a commercial licence), which `deny.toml` does not allow yet; the platform's decoders, which differ by platform, with none on Linux; libheif with libde265, which would need an exception to decision 5; or none (decision 111).
- **Not used:** GraphicsMagick, ImageMagick and libvips (C, darktable's catch-all for formats no photographer's workflow needs); the `image` crate as the one decoder (its AVIF is dav1d, C); libjpeg-turbo, libtiff, libpng, libwebp, libjxl, libavif, OpenJPEG and OpenEXR, darktable's, all C or C++.

### The volumes and their free space: sysinfo

Import's Source lists the cards and drives mounted, and its preview the destination's free space (decision 85), through `sysinfo` (MIT), a dependency of `apertrust-library` alone, its default features off and only `disk` on, checked with `cargo deny check` when it is added with Import's volumes.

- **What it gives,** on macOS, Linux and Windows alike: each volume's name, mount point, removable flag and free and total space, so the library holds no `unsafe` of its own and the CLI has the free space too.
- **Rust over the system's own calls:** on macOS through the objc2 bindings to IOKit and Core Foundation, frameworks the OS provides; on Linux `/proc/mounts` and `statvfs`; on Windows the `windows` crate. Nothing in C is compiled in.
- **Where it stops:** it reads the volumes when asked, not as they come and go; those notifications (20c) and Eject stay in `apertrust-display`'s platform code.
- **Not used:** the volumes through `apertrust-display`'s own FFI on three systems, for what `sysinfo` gives in Rust; `fs4` and `fs2`, which give a path's free space but no list of the volumes.

### The system clipboard: arboard and smithay-clipboard

Copy and paste reach the system clipboard through eframe's `clipboard` feature, which brings egui-winit's arboard and, on Wayland, smithay-clipboard, both Rust, over the platform's own clipboard and, on Linux, libwayland and the X11 libraries loaded at run time.

### The session protocol on the wire: serde and serde_json

The session protocol's types derive serde's `Serialize` and `Deserialize` (decision 108; [session-protocol.md](session-protocol.md#where-it-lives)), so serde, with its `derive` and `rc` features, is a plain dependency, behind no feature, of the seven crates whose types the protocol carries: the engine, base, plugin, raw, tools, library and export; the design system had it already. `rc` serializes the `Arc` fields, a thumbnail's pixels, the vectorscope's colours and the history's steps, each read back as an `Arc` of its own. serde_json (MIT OR Apache-2.0) is a dev-dependency of the engine and base alone, for the round-trip tests, shipped in no build; the MCP adapter is to make it a dependency of its own (decision 108).

## Licences

Decision 64. Porting darktable code makes the project GPLv3, chosen deliberately. The ported files keep darktable's copyright lines and provenance after the migration too: the code was written from darktable's source, whatever it later becomes. darktable's data files ship verbatim in `data/` under the same licence, each one's provenance in `data/README.md` (decision 45).

- **Dependencies** must pass `cargo deny check`, every licence compatible with GPLv3 ([Choosing one](#choosing-one)).
- **Bundled fonts** are the one other licence: OFL-1.1, the files unmodified and shipped with their licence text; the icons, derived from Lucide, keep its ISC and MIT notices.
- **Every source file** starts with the GPLv3 header, and a file that ports darktable code keeps darktable's copyright line (AGENTS.md, Licence headers).
