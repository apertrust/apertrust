# 0071. A self-hosted server with sync

## Context

Many photographers keep their photos on a NAS. A share over SMB lists slowly (21.6 s for 2,825 files with a call per file on macOS, [0053](0053-finding-folders-and-photos.md)), pays the network's latency on every call, needs a VPN away from home, and keeps nothing beside the files: no index of them, nothing a laptop away from home can sync with. Sidecars must stay the truth ([0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md)), and no lock may decide between copies ([0052](0052-comparing-two-copies-of-a-sidecar.md)). How does Apertrust use a home server for the photos?

## Decision

**A self-hosted server, planned, that the desktop app optionally connects to.** It comes first as a Docker container on a NAS, for amd64 and arm64. The app works fully without it. Its accounts are the person's own, on their own server, never an account with Apertrust ([principles.md](../principles.md#what-we-will-not-build)).

**No processing on the server,** in this option: it stores, lists and syncs. It runs no engine, needs no GPU and makes no renders or thumbnails; the desktop makes a server root's thumbnails as it makes any folder's. It is written in Rust, as the app is.

**The originals and their sidecars live in the server's folders.**

- The desktop keeps local copies to edit and sends back what changed. Raws never change, so only sidecars and `.apdata` files go back: kilobytes a photo.
- A raw comes down once, into a local copy in the cache keyed by its fingerprint, as an offline drive's local copies do ([0053](0053-finding-folders-and-photos.md)).
- The desktop keeps the sidecars of a server root that it has read or written, by root and path, with the changes the server has not taken yet, in its user data folder, not the cache, since the changes are nowhere else ([0076](0076-the-cache-kinds-limits-and-control.md)). So a photo can be rated and edited while the server is out of reach, and the changes go back once it answers.

**A server root** is a new kind of root ([0053](0053-finding-folders-and-photos.md)). It is found by the server's URL and an account instead of a path, and its id is kept on the server at the root's top, as a folder's is in `.apertrust/root.toml`. Each machine's root map holds the URL and the account where a folder root holds a path. A library may mix server roots and folders.

**Conflicts are decision 52's.** The server is one more copy. A sidecar sent or received is compared by its clock vectors and merged as copies from several computers are, before every write on either side. There is no lock and no check-out.

**Sidecars stay the truth** ([0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md)): the truth is the sidecar files in the server's folders. The server's index, of its folders' listings, sidecars and EXIF, is a disposable cache, as a library's is ([0055](0055-the-librarys-cache.md)): rebuilt from the files, never backed up and never the truth. No database ever is.

**The library's lowest layer goes through a storage interface** with two backends, local files and the server ([0054](0054-four-layers-for-the-librarys-code.md)). It is to be in place before Phase 5 builds on the file system.

- The server lists and reads its own folders through the same library code, with the local-files backend, and serves what that code finds.
- Sync is the storage backend's protocol: listing, reading, writing and fetching by root and path. It is not the session protocol, since the desktop's engine and library stay in its own process.

**One server,** `apertrust-cli serve`, running the `apertrust-server` library crate ([0006](0006-two-programs-on-three-desktop-platforms.md), [0058](0058-a-native-engine-every-client-through-the-session-protocol.md)). It serves sync. The engine for browser and tablet clients through the session protocol ([0058](0058-a-native-engine-every-client-through-the-session-protocol.md)) is a separate, later option for the same program, not part of this one. Authentication and encrypted transport are as [0058](0058-a-native-engine-every-client-through-the-session-protocol.md) requires: an account, and TLS by rustls with a crypto provider written in Rust, so nothing compiled in needs an exception to [0005](0005-rust-and-no-c-or-c-unless-no-rust-option-exists.md); a reverse proxy in front (the NAS's own, Traefik, Caddy) may terminate TLS instead, the server then listening only on the container's own network. A VPN is still the recommended default in front of a server reached over the Internet.

**The container's base system counts as the platform,** as a Linux distribution's own libraries do: the C library, the certificates and what else the image's base gives. Only what Apertrust itself compiles into the server follows [0005](0005-rust-and-no-c-or-c-unless-no-rust-option-exists.md).

**What it gives over a NAS share:**

- away from home, one protocol made for the network's latency, a few requests carrying kilobytes, where SMB over the Internet needs a VPN and pays the latency on every call;
- listing from the server's index, not a share's call per file;
- a place every machine, and later a mobile app, syncs its ratings and edits with.

**When:** after Phase 5's library, beside Phase 7's server work ([roadmap.md](../roadmap.md#phase-7-remote-hdr-and-packaging)).

**Later, Shared Project, for a studio shoot,** as commercial raw editors offer: several people in one project at once, on their own computers, each change on every machine within seconds. File › Share Project… starts it and Join Shared Project… joins one; a library may be shared the same way after it. ("Session" stays the engine's word, [0058](0058-a-native-engine-every-client-through-the-session-protocol.md).)

- **The host** is the NAS's server, or a desktop app running the same server code: the tethering computer, sharing its project with no NAS.
- **Changes are pushed:** the server tells every connected machine of a change as it is made, within about 2 s on a good local network, instead of at the next exchange. The local network first; over the Internet later.
- **Roles per account:** the owner manages the project's structure (import, captures, folders, roles, albums, moves and renames) and the tethering; a collaborator edits, rates, labels, keywords and makes virtual copies.
- **Merged, never locked:** decision 52's comparison, with its module-by-module merge, planned as a setting, coming in for this, so two people editing one photo's different modules both land and nobody waits on a lock ([0052](0052-comparing-two-copies-of-a-sidecar.md)).
- **Follow mode** shows the newest capture as it arrives; a sync panel shows who is connected and how far behind each machine is.

**Open questions:**

- **The server's licence.** AGPL-3.0 is common for self-hosted servers, and GPL-3.0's section 13 lets GPL-3.0-or-later code be combined with it. Apertrust is GPL-3.0-or-later ([0064](0064-licences-gplv3-darktables-provenance-and-bundled-fonts.md)).
- **Accounts and sharing between people:** one person's server, or a household's, with photos shared between its accounts.
- **Where a library whose roots are all on the server keeps its own files.**
- **A hosted service later.** The principles leave out accounts, subscriptions and cloud processing, so it would need [0001](0001-what-apertrust-is-the-principles.md) to change.

## Options considered

- **A NAS share alone,** as [0053](0053-finding-folders-and-photos.md) handles it: slow listing, no reach away from home without a VPN, nothing made beside the files, nothing for tablets.
- **A sync service or a cloud folder between machines:** already handled ([0053](0053-finding-folders-and-photos.md)), but it brings conflict copies and placeholders, and no index, thumbnails or engine beside the files.
- **A database on the server as the truth,** as photo servers usually keep: against [0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md), and one more truth to back up.
- **Check-out and check-in, with a lock on the server:** against [0052](0052-comparing-two-copies-of-a-sidecar.md). A laptop away with a lock held would block every other machine.
- **Raws synced both ways:** they never change, so only sidecars need to go back.
- **Sync as session-protocol commands to a remote library:** it would move the desktop's library out of its process, and its edits would wait on the network.
- **Processing on the server** (renders, thumbnails, the engine): it needs a GPU, or lavapipe's slowness on a NAS, and more of the engine in the container. Left to [0058](0058-a-native-engine-every-client-through-the-session-protocol.md)'s later option.
- **A server program of its own:** [0006](0006-two-programs-on-three-desktop-platforms.md) keeps the server a library inside `apertrust-cli`.
- **A hosted service first:** an account with Apertrust, which the principles leave out. It is kept as an open question.

## Consequences

- [0053](0053-finding-folders-and-photos.md) gains server roots, and [0054](0054-four-layers-for-the-librarys-code.md) the storage interface, as the library's step 21b, before Phase 5 builds on the file system. [0058](0058-a-native-engine-every-client-through-the-session-protocol.md)'s server for remote clients, later, is the same program. [0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md) and [0052](0052-comparing-two-copies-of-a-sidecar.md) hold as they are, and the server is one more copy to them.
- [library.md](../library.md#a-server) describes the server, and its open questions are listed with the library's.
- The roadmap lists it under Phase 7.
