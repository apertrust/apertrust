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

//! The engine's settings and the export settings as `apertrust-cli` reads them (decisions
//! 43 and 56): from its user data folder's `settings.toml`, chosen by `--user-data-dir`,
//! its aliases or `APERTRUST_USER_DATA_DIR`, a flag of its own first, and never written.
//! Every run here names a folder of its own, so none reads a person's.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A folder of a test's own, removed when the test ends: never the user's.
struct Folder(PathBuf);

impl Folder {
    fn new(name: &str) -> Folder {
        let path = std::env::temp_dir().join(format!(
            "apertrust-cli-settings-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("a temporary folder");
        Folder(path)
    }

    /// A folder whose settings file holds `text`.
    fn holding(name: &str, text: &str) -> Folder {
        let folder = Folder::new(name);
        std::fs::write(folder.settings(), text).expect("written");
        folder
    }

    fn settings(&self) -> PathBuf {
        self.0.join("settings.toml")
    }

    /// Each file in it with its bytes and when it was last changed, sorted by name.
    fn snapshot(&self) -> Vec<(String, Vec<u8>, std::time::SystemTime)> {
        let mut files: Vec<_> = std::fs::read_dir(&self.0)
            .expect("the folder")
            .map(|entry| {
                let entry = entry.expect("an entry");
                let modified = entry.metadata().and_then(|m| m.modified());
                let bytes = std::fs::read(entry.path()).expect("a file");
                let name = entry.file_name().to_string_lossy().into_owned();
                (name, bytes, modified.expect("a time"))
            })
            .collect();
        files.sort_by(|a, b| a.0.cmp(&b.0));
        files
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The CLI with `args` and `-d settings`, with `APERTRUST_USER_DATA_DIR` naming `variable`
/// and the home folder `home`, where given; its stderr and whether it succeeded.
fn run(args: &[&str], variable: &Path, home: Option<&Path>) -> (String, bool) {
    run_in(args, Some(variable), home)
}

/// [`run`], without `APERTRUST_USER_DATA_DIR` where `variable` is None: then the platform's
/// folder, which `home` moves on Unix, XDG's own variable left out.
fn run_in(args: &[&str], variable: Option<&Path>, home: Option<&Path>) -> (String, bool) {
    let (stderr, status) = run_status(args, variable, home);
    (stderr, status == Some(0))
}

/// [`run_in`], with the CLI's exit status: None where a signal ended it.
fn run_status(
    args: &[&str],
    variable: Option<&Path>,
    home: Option<&Path>,
) -> (String, Option<i32>) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_apertrust-cli"));
    command
        .args(["-d", "settings"])
        .args(args)
        .env_remove("RUST_LOG")
        .env_remove("APERTRUST_USER_DATA_DIR")
        .env_remove("APERTRUST_CACHE_DIR");
    if let Some(variable) = variable {
        command.env("APERTRUST_USER_DATA_DIR", variable);
    }
    if let Some(home) = home {
        command.env("HOME", home).env_remove("XDG_CONFIG_HOME");
    }
    let Output { status, stderr, .. } = command.output().expect("the CLI runs");
    (String::from_utf8_lossy(&stderr).into_owned(), status.code())
}

/// An export of a raw that is not there: it reads the engine's settings and the export
/// settings, then fails before it needs a GPU.
fn export(out: &Folder) -> Vec<String> {
    export_to(out, "out.png")
}

/// [`export`], to the file `name` in `out`, whose extension chooses the format.
fn export_to(out: &Folder, name: &str) -> Vec<String> {
    let file = out.0.join(name).display().to_string();
    vec!["export".into(), "no-such-raw.CR2".into(), "-o".into(), file]
}

/// Whether `stderr` has a line of the settings domain ending in `line`.
fn logged(stderr: &str, line: &str) -> bool {
    stderr
        .lines()
        .any(|l| l.contains("[settings] ") && l.ends_with(line))
}

const ENGINE: &str = "version = 1\ntheme = \"light\"\nfuture = 3\n\n[engine]\n\
                      adapter = \"from-file\"\nmemory_budget = 4096\n";

#[test]
fn a_flag_of_its_own_comes_before_the_file_and_the_file_before_the_default() {
    let folder = Folder::holding("precedence", ENGINE);
    let elsewhere = Folder::new("precedence-elsewhere");
    let dir = folder.0.display().to_string();
    let export = export(&elsewhere);
    let export: Vec<&str> = export.iter().map(String::as_str).collect();

    let from_file = [&["--user-data-dir", &dir], export.as_slice()].concat();
    let (stderr, _) = run(&from_file, &elsewhere.0, None);
    assert!(
        logged(&stderr, "engine.adapter: \"from-file\", from: file"),
        "{stderr}"
    );
    assert!(
        logged(&stderr, "engine.memory_budget: 4096 MiB, from: file"),
        "{stderr}"
    );
    assert!(logged(&stderr, "settings.toml, never written"), "{stderr}");

    let from_flag = [
        export.as_slice(),
        &["--adapter", "from-flag", "--configdir", &dir],
    ]
    .concat();
    let (stderr, _) = run(&from_flag, &elsewhere.0, None);
    assert!(
        logged(&stderr, "engine.adapter: \"from-flag\", from: --adapter"),
        "{stderr}"
    );
    assert!(
        logged(&stderr, "engine.memory_budget: 4096 MiB, from: file"),
        "{stderr}"
    );
    // The flag that named the folder, as the folders' line names it.
    let named = format!("user data folder: {dir}, from: --configdir");
    assert!(logged(&stderr, &named), "{stderr}");

    // A folder without a settings file: the defaults. By the variable, without a flag.
    let (stderr, _) = run(&export, &elsewhere.0, None);
    let named = format!(
        "user data folder: {}, from: APERTRUST_USER_DATA_DIR",
        elsewhere.0.display()
    );
    assert!(logged(&stderr, &named), "{stderr}");
    assert!(
        logged(&stderr, "engine.adapter: wgpu's choice, from: default"),
        "{stderr}"
    );
    assert!(
        logged(&stderr, "engine.memory_budget: 2048 MiB, from: default"),
        "{stderr}"
    );
}

/// `--conf KEY=VALUE`, before the command or after it: after a flag of its own, before the
/// file, the last for one key counting (decision 43).
#[test]
fn a_conf_comes_after_a_flag_of_its_own_and_before_the_file() {
    let folder = Folder::holding("conf", ENGINE);
    let elsewhere = Folder::new("conf-elsewhere");
    let dir = folder.0.display().to_string();
    let export = export(&elsewhere);
    let export: Vec<&str> = export.iter().map(String::as_str).collect();
    let budget = ["--conf", "engine.memory_budget=1024"];

    let before = [&["--user-data-dir", &dir], &budget[..], export.as_slice()].concat();
    let (stderr, _) = run(&before, &elsewhere.0, None);
    assert!(
        logged(&stderr, "engine.memory_budget: 1024 MiB, from: --conf"),
        "{stderr}"
    );
    assert!(
        logged(&stderr, "engine.adapter: \"from-file\", from: file"),
        "{stderr}"
    );

    let after = [
        &["--user-data-dir", &dir][..],
        export.as_slice(),
        &[
            "--conf=engine.adapter=from-conf",
            "--conf",
            "engine.memory_budget=512",
        ],
        &budget,
    ]
    .concat();
    let (stderr, _) = run(&after, &elsewhere.0, None);
    assert!(
        logged(&stderr, "engine.adapter: \"from-conf\", from: --conf"),
        "{stderr}"
    );
    assert!(
        logged(&stderr, "engine.memory_budget: 1024 MiB, from: --conf"),
        "{stderr}"
    );

    let flagged = [after.as_slice(), &["--adapter", "from-flag"]].concat();
    let (stderr, _) = run(&flagged, &elsewhere.0, None);
    assert!(
        logged(&stderr, "engine.adapter: \"from-flag\", from: --adapter"),
        "{stderr}"
    );
}

const EXPORT: &str = "version = 1\n[export]\nformat = \"jpeg\"\n\n\
                      [export.jpeg]\nquality = 80\nsubsample = \"420\"\n\n\
                      [export.png]\nbit_depth = \"16\"\n\n\
                      [export.size]\nmode = \"cm\"\ndpi = 600\nwidth = 1000\n";

/// `export` takes its format's options from the export settings, as darktable-cli takes
/// them from darktablerc: the file's, `--conf`'s over them for the run, darktable's
/// defaults where neither gives one (decision 56).
#[test]
fn export_takes_its_format_s_options_from_the_file_and_conf_over_it() {
    let folder = Folder::holding("export", EXPORT);
    let elsewhere = Folder::new("export-elsewhere");
    let dir = folder.0.display().to_string();
    let jpeg = export_to(&elsewhere, "out.jpg");
    let jpeg: Vec<&str> = jpeg.iter().map(String::as_str).collect();

    let from_file = [&["--user-data-dir", &dir][..], &jpeg].concat();
    let (stderr, _) = run(&from_file, &elsewhere.0, None);
    for line in [
        "export.jpeg.quality: 80, from: file",
        "export.jpeg.subsample: \"420\", from: file",
        "export.size.mode: \"cm\", from: file, density: 600 dpi",
        "export.size.dpi: 600, from: file",
        "export.disk.on_conflict: \"create_unique_filename\", from: default",
    ] {
        assert!(logged(&stderr, line), "{line}: {stderr}");
    }
    // `-o` is the pattern, so export.disk.path is read only where `-o` is left out.
    assert!(!stderr.contains("export.disk.path"), "{stderr}");
    let no_output = [
        &["--user-data-dir", &dir][..],
        &["export", "no-such-raw.CR2"],
    ]
    .concat();
    let (stderr, _) = run(&no_output, &elsewhere.0, None);
    for line in [
        "export.disk.path: \"$(FILE_FOLDER)/apertrust_exported/$(FILE_NAME)\", from: default",
        "export.format: \"jpeg\", from: file",
        "export.jpeg.quality: 80, from: file",
    ] {
        assert!(logged(&stderr, line), "{line}: {stderr}");
    }

    let conf = [
        "--conf",
        "export.jpeg.quality=70",
        "--conf=export.size.dpi=1200",
        "--conf=export.disk.on_conflict=skip",
    ];
    let from_conf = [&["--user-data-dir", &dir][..], &jpeg, &conf].concat();
    let (stderr, _) = run(&from_conf, &elsewhere.0, None);
    for line in [
        "export.jpeg.quality: 70, from: --conf",
        "export.jpeg.subsample: \"420\", from: file",
        "export.size.mode: \"cm\", from: file, density: 1200 dpi",
        "export.size.dpi: 1200, from: --conf",
        "export.disk.on_conflict: \"skip\", from: --conf",
    ] {
        assert!(logged(&stderr, line), "{line}: {stderr}");
    }

    // In pixels mode the dpi gives no density, and is not logged.
    let pixels = [
        &["--user-data-dir", &dir][..],
        &jpeg,
        &["--conf=export.size.mode=pixels"],
    ];
    let (stderr, _) = run(&pixels.concat(), &elsewhere.0, None);
    let line = "export.size.mode: \"pixels\", from: --conf, density: 300 dpi";
    assert!(logged(&stderr, line), "{line}: {stderr}");
    assert!(!stderr.contains("export.size.dpi"), "{stderr}");

    // A folder without a settings file: darktable's.
    let (stderr, _) = run(&jpeg, &elsewhere.0, None);
    for line in [
        "export.jpeg.quality: 95, from: default",
        "export.jpeg.subsample: \"auto\", from: default",
        "export.size.mode: \"pixels\", from: default, density: 300 dpi",
    ] {
        assert!(logged(&stderr, line), "{line}: {stderr}");
    }
}

/// A flag of `export`'s own comes over the file and `--conf` both: `-o`'s extension
/// chooses the format whatever `export.format` says, as darktable-cli's does, and that
/// format's options are the ones read.
#[test]
fn o_chooses_the_format_over_the_file_and_conf() {
    let folder = Folder::holding("export-flag", EXPORT);
    let elsewhere = Folder::new("export-flag-elsewhere");
    let dir = folder.0.display().to_string();
    let png = export_to(&elsewhere, "out.png");
    let png: Vec<&str> = png.iter().map(String::as_str).collect();
    let args = [
        &["--user-data-dir", &dir][..],
        &png,
        &["--conf", "export.format=jpeg"],
    ]
    .concat();
    let (stderr, _) = run(&args, &elsewhere.0, None);
    for line in [
        "export.format: \"png\", from: -o",
        "export.png.bit_depth: \"16\", from: file",
        "export.png.compression: 5, from: default",
    ] {
        assert!(logged(&stderr, line), "{line}: {stderr}");
    }
    assert!(!stderr.contains("export.jpeg."), "{stderr}");
}

/// A `--conf` of no engine setting, the app's included, or of a value its kind cannot
/// read, stops every command before its user data folder is even looked for, naming it,
/// with a usage error's status, 2, as clap's and the app's.
#[test]
fn a_conf_the_cli_cannot_take_stops_it_naming_it() {
    let folder = Folder::new("conf-refused");
    let missing = folder.0.join("missing").display().to_string();
    for (given, error) in [
        ("scale=1.5", "--conf scale: no setting this program reads"),
        (
            "engine.memory_budget=big",
            "--conf engine.memory_budget: cannot be \"big\"",
        ),
        (
            "export.jpeg.quality=high",
            "--conf export.jpeg.quality: cannot be \"high\"",
        ),
        (
            "engine.memory_budget",
            "--conf \"engine.memory_budget\" is not KEY=VALUE",
        ),
    ] {
        for command in [&["gpu"][..], &["info", "no-such-raw.CR2"]] {
            let args = [&["--user-data-dir", &missing, "--conf", given][..], command].concat();
            let (stderr, status) = run_status(&args, Some(&folder.0), None);
            assert_eq!(status, Some(2), "{given}: {stderr}");
            let line = format!("apertrust-cli: {error}\n");
            assert!(stderr.ends_with(&line), "{stderr}");
            assert!(!stderr.contains("user data folder"), "{stderr}");
        }
    }
}

/// The CLI reads settings and never writes them: what it sets, a damaged file and a
/// write another process left all stay as they are, and a missing file stays missing.
#[test]
fn the_cli_never_writes_its_settings() {
    let elsewhere = Folder::new("never-elsewhere");
    let export = export(&elsewhere);
    let export: Vec<&str> = export.iter().map(String::as_str).collect();
    for (name, text) in [
        ("never-engine", ENGINE),
        ("never-damaged", "version = 1\n[engine\nadapter = 3\n"),
        (
            "never-newer",
            "version = 9\n[engine]\nmemory_budget = \"big\"\n",
        ),
    ] {
        let folder = Folder::holding(name, text);
        let stale = folder.0.join("settings.toml.123-4.tmp");
        std::fs::write(&stale, "x").expect("written");
        let file = std::fs::File::options().write(true).open(&stale);
        let old = std::time::SystemTime::now() - std::time::Duration::from_secs(120);
        file.and_then(|f| f.set_modified(old)).expect("dated");
        let before = folder.snapshot();
        let dir = folder.0.display().to_string();
        let args = [
            &["--user-data-dir", &dir],
            export.as_slice(),
            &["--adapter", "x", "--conf", "engine.memory_budget=1024"],
        ]
        .concat();
        let (stderr, succeeded) = run(&args, &elsewhere.0, None);
        assert!(
            !succeeded && stderr.contains("[settings] "),
            "{name}: {stderr}"
        );
        assert_eq!(folder.snapshot(), before, "{name}: {stderr}");
    }
    let folder = Folder::new("never-none");
    let dir = folder.0.display().to_string();
    let args = [&["--user-data-dir", &dir], export.as_slice()].concat();
    run(&args, &elsewhere.0, None);
    assert!(folder.snapshot().is_empty(), "{:?}", folder.snapshot());
}

/// `--user-data-dir ~/…` and `--user-data-dir=~/…`, which no shell expands here, and its
/// aliases, are folders in the home folder, as `apertrust-base` takes them.
#[cfg(unix)]
#[test]
fn a_leading_tilde_is_the_home_folder_in_both_forms() {
    let home = Folder::new("home");
    let elsewhere = Folder::new("home-elsewhere");
    std::fs::create_dir_all(home.0.join("udd")).expect("a folder");
    std::fs::write(home.0.join("udd/settings.toml"), ENGINE).expect("written");
    let export = export(&elsewhere);
    let export: Vec<&str> = export.iter().map(String::as_str).collect();
    for (given, flag) in [
        (vec!["--user-data-dir", "~/udd"], "--user-data-dir"),
        (vec!["--user-data-dir=~/udd"], "--user-data-dir"),
        (vec!["--config-dir", "~/udd"], "--config-dir"),
        (vec!["--configdir=~/udd"], "--configdir"),
    ] {
        let args = [given.as_slice(), export.as_slice()].concat();
        let (stderr, _) = run(&args, &elsewhere.0, Some(&home.0));
        let named = format!(
            "user data folder: {}, from: {flag}",
            home.0.join("udd").display()
        );
        assert!(logged(&stderr, &named), "{given:?}: {stderr}");
        assert!(
            logged(&stderr, "engine.adapter: \"from-file\", from: file"),
            "{given:?}: {stderr}"
        );
    }
    // A missing one is an error naming it in the home folder, and nothing is created,
    // neither there nor a folder named `~`.
    let args = [&["--user-data-dir=~/new/udd"], export.as_slice()].concat();
    let (stderr, succeeded) = run(&args, &elsewhere.0, Some(&home.0));
    let missing = home.0.join("new/udd");
    let named = format!("cannot find the user data folder {}", missing.display());
    assert!(!succeeded && stderr.contains(&named), "{stderr}");
    assert!(!home.0.join("new").exists());
    assert!(!Path::new(env!("CARGO_MANIFEST_DIR")).join("~").exists());
}

/// A folder given that is missing is an error naming it, its cause said once, and nothing
/// is created; only the commands that run the engine read its settings.
#[test]
fn a_missing_folder_is_refused_and_nothing_is_created() {
    let folder = Folder::new("missing");
    let typo = folder.0.join("typo/deep/folder");
    let dir = typo.display().to_string();
    let (stderr, succeeded) = run(&["--user-data-dir", &dir, "gpu"], &folder.0, None);
    let error = format!(
        "Error: cannot find the user data folder {dir} given by --user-data-dir: no such folder\n"
    );
    assert!(!succeeded && stderr.ends_with(&error), "{stderr}");
    assert!(!stderr.contains("Caused by"), "{stderr}");
    assert!(!folder.0.join("typo").exists());
    // `info` reads no engine settings.
    let (stderr, _) = run(&["info", "no-such-raw.CR2"], &folder.0, None);
    assert!(!stderr.contains("engine."), "{stderr}");
    assert!(stderr.contains("[settings] user data folder: "), "{stderr}");
}

/// A read-only folder, as one shared from another machine, is read: the CLI writes nothing
/// there, so it needs nothing but to read it.
#[cfg(unix)]
#[test]
fn a_read_only_folder_is_read() {
    use std::os::unix::fs::PermissionsExt;
    let folder = Folder::holding("read-only", ENGINE);
    let elsewhere = Folder::new("read-only-elsewhere");
    let mode = |mode| std::fs::set_permissions(&folder.0, std::fs::Permissions::from_mode(mode));
    mode(0o555).expect("read-only");
    let dir = folder.0.display().to_string();
    let export = export(&elsewhere);
    let export: Vec<&str> = export.iter().map(String::as_str).collect();
    let args = [&["--user-data-dir", &dir], export.as_slice()].concat();
    let (stderr, _) = run(&args, &elsewhere.0, None);
    mode(0o755).expect("writable again, to be removed");
    assert!(
        logged(&stderr, "engine.adapter: \"from-file\", from: file"),
        "{stderr}"
    );
    assert!(!stderr.contains("cannot write"), "{stderr}");
}

/// With no folder named, the platform's is read where it is, and created nowhere when it
/// is missing: its settings are the defaults.
#[cfg(unix)]
#[test]
fn a_missing_platform_folder_is_not_created() {
    let home = Folder::new("platform-home");
    let elsewhere = Folder::new("platform-elsewhere");
    let export = export(&elsewhere);
    let export: Vec<&str> = export.iter().map(String::as_str).collect();
    let (stderr, _) = run_in(&export, None, Some(&home.0));
    assert!(logged(&stderr, ", from: the platform, missing"), "{stderr}");
    assert!(
        logged(&stderr, "engine.memory_budget: 2048 MiB, from: default"),
        "{stderr}"
    );
    // Neither `Library` nor `.config`, nor anything else of Apertrust's. The GPU driver
    // may keep its shader cache in `.cache` (Mesa's, on Linux), but Apertrust's own cache
    // folder is not there either.
    let created: Vec<_> = std::fs::read_dir(&home.0)
        .expect("there")
        .map(|entry| entry.expect("read").file_name())
        .filter(|name| name != ".cache")
        .collect();
    assert!(created.is_empty(), "{created:?}");
    assert!(!home.0.join(".cache/apertrust").exists());
}

/// An adapter the settings name that is not there falls back to wgpu's choice with a
/// warning; one the flag names that is not there is an error (decision 43). Needs a GPU
/// adapter, hardware or software.
#[test]
fn a_setting_that_names_no_adapter_falls_back_and_a_flag_is_refused() {
    let folder = Folder::holding(
        "adapter",
        "version = 1\n[engine]\nadapter = \"no adapter is called this\"\n",
    );
    let dir = folder.0.display().to_string();
    let (stderr, succeeded) = run(&["--user-data-dir", &dir, "gpu"], &folder.0, None);
    assert!(succeeded, "{stderr}");
    let warned =
        "engine.adapter \"no adapter is called this\" names no adapter: wgpu's choice used";
    assert!(stderr.contains(warned), "{stderr}");

    let args = ["--user-data-dir", &dir, "gpu", "--adapter", "nor this"];
    let (stderr, succeeded) = run(&args, &folder.0, None);
    assert!(!succeeded, "{stderr}");
    assert!(
        stderr.contains("no GPU adapter name contains \"nor this\""),
        "{stderr}"
    );
}

/// The CLI with `args` and `-d settings`, reading no user data folder's settings but
/// `folder`'s, with `WGPU_BACKEND` unset and the variables `envs` set; its stderr and exit
/// status.
fn run_backend(args: &[&str], folder: &Folder, envs: &[(&str, &str)]) -> (String, Option<i32>) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_apertrust-cli"));
    command
        .args(["-d", "settings", "--user-data-dir"])
        .arg(&folder.0)
        .args(args)
        .env_remove("RUST_LOG")
        .env_remove("APERTRUST_USER_DATA_DIR")
        .env_remove("APERTRUST_CACHE_DIR")
        .env_remove("WGPU_BACKEND")
        .envs(envs.iter().copied());
    let Output { status, stderr, .. } = command.output().expect("the CLI runs");
    (String::from_utf8_lossy(&stderr).into_owned(), status.code())
}

/// The Vulkan loader's variable, naming a driver that is not there: Vulkan then lists no
/// adapter, as on a machine without a Vulkan driver.
#[cfg(windows)]
fn no_vulkan_driver(folder: &Folder) -> (&'static str, String) {
    let missing = folder.0.join("no-such-driver.json");
    ("VK_DRIVER_FILES", missing.display().to_string())
}

/// The graphics API, `engine.backend` (decision 82): `WGPU_BACKEND`, then `--backend`, then
/// `--conf`, then the file, then Automatic, each logged with where it came from; a name
/// `--backend` does not know is a usage error.
#[test]
fn a_backend_comes_from_wgpu_backend_then_the_flag_then_conf_then_the_file() {
    let folder = Folder::holding("backend", "version = 1\n[engine]\nbackend = \"vulkan\"\n");
    let elsewhere = Folder::new("backend-elsewhere");
    let export = export(&elsewhere);
    let export: Vec<&str> = export.iter().map(String::as_str).collect();
    let (stderr, _) = run_backend(&export, &folder, &[]);
    // macOS's wgpu has no Vulkan, and a machine may have no Vulkan driver: the file's is
    // then warned of and Automatic taken.
    let used = if cfg!(target_os = "macos") {
        let warned = "engine.backend vulkan in the settings file: the graphics API vulkan \
                      is not built for macOS; Automatic used";
        assert!(stderr.contains(warned), "{stderr}");
        "auto"
    } else if stderr.contains("the graphics API vulkan finds no adapter; Automatic used") {
        "auto"
    } else {
        "vulkan"
    };
    let from_file = format!("engine.backend: {used}, from: file");
    assert!(logged(&stderr, &from_file), "{stderr}");
    let conf = [export.as_slice(), &["--conf", "engine.backend=dx12"]].concat();
    let (stderr, _) = run_backend(&conf, &folder, &[]);
    assert!(
        logged(&stderr, "engine.backend: dx12, from: --conf"),
        "{stderr}"
    );
    let flag = [conf.as_slice(), &["--backend", "vulkan"]].concat();
    let (stderr, _) = run_backend(&flag, &folder, &[]);
    assert!(
        logged(&stderr, "engine.backend: vulkan, from: --backend"),
        "{stderr}"
    );
    let (stderr, _) = run_backend(&flag, &folder, &[("WGPU_BACKEND", "vulkan")]);
    let variable = "engine.backend: vulkan, from: WGPU_BACKEND";
    assert!(logged(&stderr, variable), "{stderr}");

    // The setting's names alone: Metal is Automatic on macOS, and no name of its own.
    for name in ["opengl", "metal"] {
        for command in [&["gpu"][..], &["preview", "no-such-raw.CR2"], &export] {
            let args = [command, &["--backend", name]].concat();
            let (stderr, status) = run_backend(&args, &folder, &[]);
            assert_eq!(status, Some(2), "{stderr}");
            let refused = format!("{name} is none of auto, dx12, vulkan");
            assert!(stderr.contains(&refused), "{stderr}");
        }
    }
}

/// A backend this build lacks, typed for the run, is an error, unless `WGPU_BACKEND`
/// chooses in its place; one in the file is Automatic (above). Windows builds every one.
/// Needs a GPU adapter, hardware or software.
#[cfg(not(windows))]
#[test]
fn a_backend_typed_for_the_run_that_this_build_lacks_is_refused() {
    let folder = Folder::new("backend-lacking");
    let (system, primary) = if cfg!(target_os = "macos") {
        ("macOS", "metal")
    } else {
        ("Linux", "vulkan")
    };
    let refused = format!("the graphics API dx12 is not built for {system}");
    for args in [
        &["gpu", "--backend", "dx12"][..],
        &["--conf", "engine.backend=dx12", "gpu"],
    ] {
        let (stderr, status) = run_backend(args, &folder, &[]);
        assert_eq!(status, Some(1), "{stderr}");
        assert!(stderr.contains(&refused), "{stderr}");
    }
    let args = ["gpu", "--backend", "dx12"];
    let (stderr, status) = run_backend(&args, &folder, &[("WGPU_BACKEND", primary)]);
    assert_eq!(status, Some(0), "{stderr}");
}

/// A backend the file names that finds no adapter on this machine, as Vulkan without a
/// Vulkan driver, is warned of and Automatic taken, so the engine still opens; the same
/// typed for the run is an error. On Windows, where Direct3D 12 is there without Vulkan
/// (WARP at least). Needs a GPU adapter, hardware or software.
#[cfg(windows)]
#[test]
fn a_file_s_backend_that_finds_no_adapter_falls_back_and_a_flag_s_is_refused() {
    let folder = Folder::holding(
        "backend-no-adapter",
        "version = 1\n[engine]\nbackend = \"vulkan\"\n",
    );
    let (name, missing) = no_vulkan_driver(&folder);
    let no_vulkan = [(name, missing.as_str())];
    let (stderr, status) = run_backend(&["gpu"], &folder, &no_vulkan);
    assert_eq!(status, Some(0), "{stderr}");
    let warned = "engine.backend vulkan in the settings file: the graphics API vulkan finds \
                  no adapter; Automatic used";
    assert!(stderr.contains(warned), "{stderr}");
    assert!(
        logged(&stderr, "engine.backend: auto, from: file"),
        "{stderr}"
    );
    let (stderr, status) = run_backend(&["gpu", "--backend", "vulkan"], &folder, &no_vulkan);
    assert_eq!(status, Some(1), "{stderr}");
    assert!(stderr.contains("no usable GPU adapter"), "{stderr}");
}

/// The cache folder `-d settings` names: `--cache-dir`'s or darktable's `--cachedir`'s, the
/// last given, before or after the command, then `APERTRUST_CACHE_DIR`'s, then the user
/// data folder's when that is given; created nowhere, a relative one not in the current
/// folder either.
#[test]
fn the_cache_folder_is_named_and_never_created() {
    let folder = Folder::new("cache");
    let current = Folder::new("cache-current");
    let dir = folder.0.display().to_string();
    let cache = folder.0.join("cache");
    let given = cache.display().to_string();
    // Each run in a current folder of the test's own, where a relative folder would land.
    let run_here = |args: &[&str], variable: Option<&Path>| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_apertrust-cli"));
        command
            .current_dir(&current.0)
            .args(["-d", "settings"])
            .args(args)
            .env_remove("RUST_LOG")
            .env("APERTRUST_USER_DATA_DIR", &folder.0)
            .env_remove("APERTRUST_CACHE_DIR");
        if let Some(variable) = variable {
            command.env("APERTRUST_CACHE_DIR", variable);
        }
        let stderr = command.output().expect("the CLI runs").stderr;
        String::from_utf8_lossy(&stderr).into_owned()
    };
    let named = |stderr: &str, from: &str| {
        let line = format!("cache folder: {}, from: {from}", cache.display());
        logged(stderr, &line)
    };
    let joined = format!("--cachedir={given}");
    let last_apart = [
        "--cachedir",
        "elsewhere",
        "--cache-dir",
        &given,
        "info",
        "x.CR2",
    ];
    let last_joined = ["--cache-dir", "elsewhere", "info", &joined, "x.CR2"];
    for (args, flag) in [
        (last_apart.as_slice(), "--cache-dir"),
        (&last_joined, "--cachedir"),
    ] {
        let stderr = run_here(args, None);
        assert!(named(&stderr, flag), "{args:?}: {stderr}");
    }
    let stderr = run_here(&["--user-data-dir", &dir, "info", "x.CR2"], None);
    assert!(named(&stderr, "the user data folder"), "{stderr}");
    let stderr = run_here(&["info", "x.CR2"], Some(&cache));
    assert!(named(&stderr, "APERTRUST_CACHE_DIR"), "{stderr}");
    // `elsewhere` alone is the current folder's, and is not made there either.
    let stderr = run_here(&["--cachedir", "elsewhere", "info", "x.CR2"], None);
    // As the CLI finds its current folder: with links resolved, as macOS's temporary one.
    let here = std::fs::canonicalize(&current.0).expect("the current folder");
    let elsewhere = here.join("elsewhere");
    let line = format!("cache folder: {}, from: --cachedir", elsewhere.display());
    assert!(logged(&stderr, &line), "{stderr}");
    assert!(!cache.exists() && !elsewhere.exists());
    assert_eq!(std::fs::read_dir(&current.0).expect("there").count(), 0);
}
