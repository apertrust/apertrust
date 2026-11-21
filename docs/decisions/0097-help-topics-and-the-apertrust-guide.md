# 0097. Help topics and the Apertrust Guide

## Context

Apertrust needs help a person can reach from where they are: every tool, and the other places that need explaining, such as the Browser, the workspaces and Settings. The help is a website of its own, built in its own repository, versioned with the app's releases and translated into the interface's languages. The app must link into it without breaking when its pages move, and old releases must keep working. darktable's help links each widget to a page path held in a table compiled into darktable (`src/common/usermanual_url.c`), its manual's languages in a list compiled in too, so a page that moves breaks the link in every release already shipped.

## Decision

**The Apertrust Guide.** The help is a website called the Apertrust Guide, at `guide.apertrust.org`, built from the repository `apertrust/apertrust-guide`. In the app, every item that opens it says Help; the Help menu's item says Apertrust Guide. The Guide is written as how-to's, not a technical reference (Content, below).

**Topics are the Guide's page paths.** The app knows only topics, and a topic is the path of the page in the Guide that answers it: `tools/processing/exposure`, `browser`, `settings/storage`. A topic, once in a release, is never renamed or removed. When a page moves, merges or splits, the Guide keeps its old path answering with a redirect to the page that now answers it (The Guide's repository, below), so the app needs no change and every release already shipped still lands.

- **A tool's topic** comes from its kind ([0093](0093-one-tool-model-processing-tools-and-utility-tools.md)) and its id, with nothing declared: `tools/processing/<id>` for a processing tool, as `tools/processing/exposure`; `tools/utility/<id>` for a utility tool, as `tools/utility/export`, `tools/utility/history` and `tools/utility/collections`. `ToolDefinition::help: Option<&'static str>` overrides it where a tool's help is elsewhere, which should be rare (planned).
- **Every other place offering help names its topic:** `browser`, `viewer`, `workspaces` (the Tools panel's ⋮), `settings/<page>` for each page of Settings, `dialogs/<id>` for a dialog with a Help button. The Guide's start page needs none.

**The URL.** The app joins four parts and opens the system's browser, through egui's `open_url` (eframe's `links`, already built in): `{base}/{version}/{language}/{topic}/` for a topic, for example `https://guide.apertrust.org/1.0/uk/tools/processing/exposure/`, answered by the page at that path, and `{base}/{version}/{language}/` for the Guide's start page, `https://guide.apertrust.org/1.0/uk/`.

- **`base`** is the setting `help.url`, `https://guide.apertrust.org` by default, so a person can point it at a mirror or a copy of their own (planned).
- **`version`** is the release's major and minor, `1.0`, for a build of a `release-X.Y.Z` tag; `development` for any other build, a commit after a tag and a nightly among them (decision 44's versions).
- **`language`** is the interface's: `uk`, `pl`, and `en` for both British and American English, the Guide written in British English as the interface's source is; the pseudo-locale takes `en`.
- The app holds no list of the Guide's pages or languages.

**Asking first.** The first time a person chooses Help, the app asks whether to open the Apertrust Guide online, naming the address, and remembers the answer for that address; a new `help.url` asks again. Declined, Help does nothing but say where the Guide is (planned).

**Where Help is** (planned):

| Place | How | Topic |
| --- | --- | --- |
| A tool's ⋯ in the Tools panel | Help, its last item, after the layout's, a line before it: the registry action `section-help`, acting on its own section as the `section-*` items do | `tools/processing/<id>` or `tools/utility/<id>`, or its override |
| The Help menu | Apertrust Guide, its first item after macOS's search: the action `guide` | None: the start page, `{base}/{version}/{language}/` |
| The browser bar's options menu | Help, last | `browser` |
| The Tools panel's ⋮ | Help, last | `workspaces` |
| Settings, and a dialog that needs explaining | A Help button, an icon at the footer's start, "Help" its tooltip and its name for screen readers | `settings/<page>`, `dialogs/<id>` |
| What has the focus | The action `help`, no key by default: F1 to F5 are the colour labels, as darktable has them ([the default keyboard map](../gui.md#the-default-keyboard-map)), so a person binds one | Its own topic, a Tools panel tool's header its tool's, `tools/processing/<id>` or `tools/utility/<id>` or its override, as its ⋯'s Help; an Import sheet tool's header none, as its ⋯ |

The photo's and a thumbnail's right-click menus hold no Help: they act on photos, and decision 91 bounds them to 16 items. The Import sheet's Help is a button at its footer's start, as Settings' is, opening the topic `import` ([0086](0086-import-as-a-full-window-sheet.md)); its Import tool's ⋯ holds no Help.

**The Guide's repository** (planned):

- **Astro with Starlight,** Astro's own theme for documentation, builds it, as Astro builds apertrust.org, and GitHub Pages serves it. Starlight brings what the Guide needs: languages with an English fallback, a search that runs in the browser (Pagefind), dark and light, and accessible navigation. A page's path is where readers find it and is its own topic; its front matter lists the other topics it answers, its own path implied (`topics: [tools/exposure]` on `tools/processing/exposure`, had it moved from there). An entry may name a heading on the page after a `#`: `workspaces/hidden-tools#hidden-tools` sends the topic `workspaces/hidden-tools` to the page's Hidden tools heading. The `#` is the front matter's alone; a topic never holds one, so the app's URL never carries a fragment, and the redirect adds the heading.
- **A topic other than a page's own path becomes a redirect page** at that path, `/{version}/{language}/{topic}/`, to the page that claims it, generated from the pages' `topics` as the site is built: a page that moved keeps its old path, and a topic for a heading lands on it. A path the site does not have, a topic no page claims among them, reaches the site's 404 page, which GitHub Pages serves for every such path: for any path under a version and a language it says the page is not written yet, in that version and language, with the search and a link to the start page. A page not translated is the English page with Starlight's line saying so; a language the Guide does not have reaches English through the same 404 page, so the app's URL always lands.
- **Versions are branches:** `main` builds `/development/`; a `release-X.Y` branch, cut at the release of X.Y.0 and kept for its patch releases, builds `/X.Y/`, which shows the Guide of the newest X.Y release; `/latest/` redirects to the newest. One workflow builds every branch with its own base path into one site; every minor release's Guide stays published, its redirects its own.
- **Translations** go through Weblate with po4a, as darktable's manual does, once the English pages settle: a changed English paragraph marks its translations for review instead of leaving them stale.

**Checks in both directions** (planned):

- In the app, a test holds every tool and every place that offers Help to a topic; places may share one.
- `apertrust --list-help-topics` prints every topic the app uses, one a line, and opens no window, as `--list-adapters` does.
- The Guide's CI builds the app at `main`, and a release branch at its release's tag, runs that, and fails on a topic no English page claims, its own path or its front matter; on a topic two pages claim; and on a topic in a page's front matter that is another page's own path.

**Content.** The Guide is for people editing photos. It follows [Diátaxis](https://diataxis.fr/)'s four kinds, each page one kind, how-to's first:

- **Getting started,** Diátaxis's tutorial: one walk from opening a raw to exporting it.
- **How-to's,** named by what a person wants, not by a tool: "Fix a colour cast", "Recover blown highlights", "Straighten the horizon", "Export for the web"; a few numbered steps with pictures, each linking the tools it uses.
- **Tool pages,** the reference, where a tool's Help lands, the same on every one: what it is for in a line, when to reach for it, links to the how-to's that use it, then each control and what it does.
- **Concepts,** Diátaxis's explanation, short and optional: scene-referred editing, why the processing order is fixed, workspaces.

## Options considered

- **Page paths in the app with nothing to keep them,** as darktable's table: every page that moves breaks the releases already shipped, and the app would need a release to fix a link.
- **A `/topic/` namespace of redirects,** `{base}/{version}/{language}/topic/{topic}/`, every topic a redirect page apart from the pages: the `topic/` part reads as noise in an address people see and share, and topics that are the pages' own paths stay as stable through the redirects the Guide generates when a page moves.
- **A table of topics to pages in the app,** shipped with each release: the same breakage a step removed; the Guide changes more often than the app.
- **darktable's help mode,** a Toolbar button turning the pointer into a question mark for the next click: a button and a mode of its own, out of reach of the keyboard; ⋯'s item, the menus and the `help` action reach the same pages.
- **F1 for help:** F1 to F5 are darktable's colour-label keys, which Apertrust keeps.
- **Hugo,** as darktable's manual: a second site toolchain beside apertrust.org's Astro, for what Starlight already does.
- **starlight-versions,** a Starlight plugin keeping old versions as copies in `main`: still in early development by its own account, and a release's Guide is better built from that release's branch, as the app is from its tag.
- **mdBook,** in Rust: its translations are weak, and the rule of Rust first (decision 5) covers what ships with the app, not the website's tools.
- **Docusaurus,** with versions and translations built in: a React toolchain beside apertrust.org's Astro.
- **"Manual" and `manual.apertrust.org`:** a manual reads as the technical reference the Guide is not. **`help.apertrust.org`:** the site's own name is the Guide, and help.* is where people look for support, which Apertrust may want for that later.
- **Only the latest release's manual online,** as darktable's: a release's Help would land on a newer Guide's pages, describing controls it does not have.
- **A reference first,** a page per tool listing its controls, as darktable's manual is: it answers what a control does, not how to get the photo a person wants.
- **Opening the browser without asking:** Help is the person's own choice, but it is the app's first reach to the Internet from the desktop, so it asks once.

## Consequences

- `apertrust-tools`: `ToolDefinition::help`, and a tool's topic from its kind and id (planned).
- The app: a `help` module building the URL and asking first; the actions `section-help`, `guide` and `help`; Help in ⋯, the Help menu, the browser bar's options, the Tools panel's ⋮, Settings and the dialogs that need one; the settings `help.url` and the answer kept for its address; `--list-help-topics`; their messages in every language (planned).
- The repository `apertrust/apertrust-guide`: Astro with Starlight, the topic redirects and the 404 page, the versioned build and its deployment to `guide.apertrust.org` on GitHub Pages, the CI against the app's topics, a stub page for every tool's topic, then the how-to's (planned).
- gui.md's Help menu, ⋯, the browser bar's options and the Tools panel's ⋮ list the items; the roadmap lists the work.
