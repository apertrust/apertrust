# 0071. A self-hosted server with sync

## Context

Many photographers keep their photos on a NAS. A share over SMB lists slowly (21.6 s for 2,825 files with a call per file on macOS, [0053](0053-finding-folders-and-photos.md)), pays the network's latency on every call, needs a VPN away from home, and makes nothing beside the files: no index, no thumbnails, nothing a tablet or a browser can connect to. Sidecars must stay the truth ([0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md)), and no lock may decide between copies ([0052](0052-comparing-two-copies-of-a-sidecar.md)). How does Apertrust use a home server for the photos, and later for the engine?

## Decision

**A self-hosted server, planned, that the desktop app optionally connects to.** It comes first as a Docker container on a NAS, for amd64 and arm64. The app works fully without it. Its accounts are the person's own, on their own server, never an account with Apertrust ([principles.md](../principles.md#what-we-will-not-build)).

**The originals and their sidecars live in the server's folders.**

- The desktop keeps local copies to edit and sends back what changed. Raws never change, so only sidecars and `.apdata` files go back: kilobytes a photo.
- A raw comes down once, into a local copy in the cache keyed by its fingerprint, as an offline drive's local copies do ([0053](0053-finding-folders-and-photos.md)).
- The desktop keeps in its cache the sidecars of a server root that it has read or written, by root and path, with the changes the server has not taken yet. So a photo can be rated and edited while the server is out of reach, and the changes go back once it answers.

**A server root** is a new kind of root ([0053](0053-finding-folders-and-photos.md)). It is found by the server's URL and an account instead of a path, and its id is kept on the server at the root's top, as a folder's is in `.apertrust/root.toml`. Each machine's root map holds the URL and the account where a folder root holds a path. A library may mix server roots and folders.

**Conflicts are decision 52's.** The server is one more copy. A sidecar sent or received is compared by its clock vectors and merged as copies from several computers are, before every write on either side. There is no lock and no check-out.

**Sidecars stay the truth** ([0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md)): the truth is the sidecar files in the server's folders. The server's index, of its folders' listings, sidecars and EXIF, is a disposable cache, as a library's is ([0055](0055-the-librarys-cache.md)): rebuilt from the files, never backed up and never the truth. No database ever is.

**The library's lowest layer goes through a storage interface** with two backends, local files and the server ([0054](0054-four-layers-for-the-librarys-code.md)). It is to be in place before Phase 5 builds on the file system.

- The server lists and reads its own folders through the same library code, with the local-files backend, and serves what that code finds.
- Sync is the storage backend's protocol: listing, reading, writing and fetching by root and path. It is not the session protocol, since the desktop's engine and library stay in its own process.

**One server,** `apertrust-cli serve`, running the `apertrust-server` library crate ([0006](0006-two-programs-on-three-desktop-platforms.md), [0058](0058-a-native-engine-every-client-through-the-session-protocol.md)). It serves sync first, and later the engine for browser and tablet clients through the session protocol. Sync and listing need no GPU. Authentication and encrypted transport are as [0058](0058-a-native-engine-every-client-through-the-session-protocol.md) requires: TLS and an account, with a VPN still the recommended default in front of a server reached over the Internet.

**What it gives over a NAS share:**

- away from home, one protocol made for the network's latency, a few requests carrying kilobytes, where SMB over the Internet needs a VPN and pays the latency on every call;
- listing from the server's index, not a share's call per file;
- thumbnails made on the server, beside the files;
- a place for tablets and browsers to connect ([0058](0058-a-native-engine-every-client-through-the-session-protocol.md)).

**When:** after Phase 5's library, beside Phase 7's server work ([roadmap.md](../roadmap.md#phase-7-remote-hdr-and-packaging)).

**Open questions:**

- **The server's licence.** AGPL-3.0 is common for self-hosted servers, and GPL-3.0's section 13 lets GPL-3.0-or-later code be combined with it. Apertrust is GPL-3.0-or-later ([0064](0064-licences-gplv3-darktables-provenance-and-bundled-fonts.md)).
- **Accounts and sharing between people:** one person's server, or a household's, with photos shared between its accounts.
- **How much of the engine runs on a NAS without a GPU.** Thumbnails from the embedded pictures need no GPU, and renders run on lavapipe ([0007](0007-a-software-gpu-instead-of-a-cpu-path.md)).
- **What the container ships beside the binary:** Mesa's lavapipe, the Vulkan loader and a TLS stack. Apertrust would ship the image, so [0005](0005-rust-and-no-c-or-c-unless-no-rust-option-exists.md) weighs each.
- **Where a library whose roots are all on the server keeps its own files.**
- **A hosted service later.** The principles leave out accounts, subscriptions and cloud processing, so it would need [0001](0001-what-apertrust-is-the-principles.md) to change.

## Options considered

- **A NAS share alone,** as [0053](0053-finding-folders-and-photos.md) handles it: slow listing, no reach away from home without a VPN, nothing made beside the files, nothing for tablets.
- **A sync service or a cloud folder between machines:** already handled ([0053](0053-finding-folders-and-photos.md)), but it brings conflict copies and placeholders, and no index, thumbnails or engine beside the files.
- **A database on the server as the truth,** as photo servers usually keep: against [0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md), and one more truth to back up.
- **Check-out and check-in, with a lock on the server:** against [0052](0052-comparing-two-copies-of-a-sidecar.md). A laptop away with a lock held would block every other machine.
- **Raws synced both ways:** they never change, so only sidecars need to go back.
- **Sync as session-protocol commands to a remote library:** it would move the desktop's library out of its process, and its edits would wait on the network.
- **A server program of its own:** [0006](0006-two-programs-on-three-desktop-platforms.md) keeps the server a library inside `apertrust-cli`.
- **A hosted service first:** an account with Apertrust, which the principles leave out. It is kept as an open question.

## Consequences

- [0053](0053-finding-folders-and-photos.md) gains server roots, and [0054](0054-four-layers-for-the-librarys-code.md) the storage interface, as the library's step 21b, before Phase 5 builds on the file system. [0058](0058-a-native-engine-every-client-through-the-session-protocol.md)'s server is this one. [0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md) and [0052](0052-comparing-two-copies-of-a-sidecar.md) hold as they are, and the server is one more copy to them.
- [library.md](../library.md#a-server) describes the server, and its open questions are listed with the library's.
- The roadmap lists it under Phase 7.
