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

use apertrust_ui::Locale;
use wgpu::DeviceType::{Cpu, DiscreteGpu, IntegratedGpu};
use wgpu::PowerPreference::{HighPerformance, LowPower};

use super::*;

/// The laptop's names, as wgpu reports them.
pub(crate) const RTX: &str = "NVIDIA GeForce RTX 5060 Laptop GPU";
pub(crate) const RADEON: &str = "AMD Radeon 780M Graphics";
pub(crate) const WARP: &str = "Microsoft Basic Render Driver";
pub(crate) const M4: &str = "Apple M4";

/// An adapter as the helper lists it.
pub(crate) fn adapter(name: &str, kind: wgpu::DeviceType, backend: wgpu::Backend) -> Adapter {
    Adapter {
        name: name.to_owned(),
        kind,
        backend,
    }
}

/// What a computer like the test laptop lists with `backend`, on `os`: on Windows the
/// integrated GPU first, as wgpu lists them there, and WARP with Direct3D 12; on the Mac
/// an Apple GPU alone; on Linux the same GPUs with Vulkan, and no lavapipe.
pub(crate) fn listed(os: OperatingSystem, backend: Backend) -> Vec<Adapter> {
    use wgpu::Backend::{Dx12, Metal, Vulkan};
    let vulkan = [
        adapter(RADEON, IntegratedGpu, Vulkan),
        adapter(RTX, DiscreteGpu, Vulkan),
    ];
    let dx12 = [
        adapter(RADEON, IntegratedGpu, Dx12),
        adapter(RTX, DiscreteGpu, Dx12),
        adapter(WARP, Cpu, Dx12),
    ];
    match (os, backend) {
        (OperatingSystem::Mac, _) => vec![adapter(M4, IntegratedGpu, Metal)],
        (OperatingSystem::Windows, Backend::Auto) => vulkan.into_iter().chain(dx12).collect(),
        (OperatingSystem::Windows, Backend::Dx12) => dx12.into(),
        _ => vulkan.into(),
    }
}

/// What the app hands Settings on such a computer, the RTX in use with Vulkan, listing
/// as [`listed`] does, built as for `os`, whatever this machine's build has.
pub(crate) fn running(os: OperatingSystem) -> Running {
    let backend = if os == OperatingSystem::Mac {
        wgpu::Backend::Metal
    } else {
        wgpu::Backend::Vulkan
    };
    let name = if os == OperatingSystem::Mac { M4 } else { RTX };
    let built = match os {
        OperatingSystem::Windows => wgpu::Backends::DX12 | wgpu::Backends::VULKAN,
        OperatingSystem::Mac => wgpu::Backends::METAL,
        _ => wgpu::Backends::VULKAN,
    };
    Running {
        in_use: Some(InUse {
            name: name.to_owned(),
            backend,
            software: false,
            chosen: false,
        }),
        built,
        lister: Some(Lister::new(move |api| Ok(listed(os, api)))),
        started: Started::default(),
        restart_failed: None,
    }
}

fn words() -> Translator {
    Translator::new(Locale::EnGb)
}

/// The card's list on `os` with `stored`, for `shown`, the API's words `api`.
fn list(os: OperatingSystem, stored: &str, shown: Shown<'_>, api: &str) -> Card {
    card(
        &words(),
        kept(stored),
        shown,
        Some(api),
        os,
        HighPerformance,
    )
}

/// `text` as the card's value, told software by its kind where it can be, as the dialog
/// tells it with no listing yet.
fn kept(text: &str) -> Stored<'_> {
    let software = Hardware::default().software(text);
    Stored { text, software }
}

fn info(name: &str, kind: wgpu::DeviceType, backend: wgpu::Backend) -> wgpu::AdapterInfo {
    wgpu::AdapterInfo {
        name: name.to_owned(),
        ..wgpu::AdapterInfo::new(kind, backend)
    }
}

/// What the helper prints reads back as what it listed, a name with a tab or a line break
/// in it on its one line; what a driver prints before the list, or anything after it, is
/// not the helper's.
#[test]
fn the_helper_s_lines_read_back_as_its_adapters() {
    use wgpu::Backend::{Dx12, Vulkan};
    let infos = [
        info(RADEON, IntegratedGpu, Vulkan),
        info("Odd\tname\r\n", DiscreteGpu, Vulkan),
        info(WARP, Cpu, Dx12),
    ];
    let text = lines(&infos);
    assert_eq!(
        text,
        "apertrust adapters 1\n\
         integrated\tvulkan\tAMD Radeon 780M Graphics\n\
         discrete\tvulkan\tOdd name\n\
         cpu\tdx12\tMicrosoft Basic Render Driver\n\
         end\n"
    );
    let read = vec![
        adapter(RADEON, IntegratedGpu, Vulkan),
        adapter("Odd name", DiscreteGpu, Vulkan),
        adapter(WARP, Cpu, Dx12),
    ];
    assert_eq!(parse(&text), Ok(read.clone()));
    let windows = text.replace('\n', "\r\n");
    assert_eq!(parse(&windows), Ok(read.clone()));
    let wrapped = format!("a driver's own line\n{text}test result: ok\n");
    assert_eq!(parse(&wrapped), Ok(read));
    assert_eq!(parse(&lines(&[])), Ok(Vec::new()));
}

/// Nothing, no list, a line that is no adapter, an unknown kind or a list cut short, as by
/// a crash, are failures, never a list of fewer cards.
#[test]
fn what_is_not_a_whole_list_is_a_failure() {
    assert_eq!(parse(""), Err(Failure::Nothing));
    assert_eq!(parse(" \n"), Err(Failure::Nothing));
    assert_eq!(parse("Segmentation fault\n"), Err(Failure::Garbled));
    let cut = "apertrust adapters 1\ndiscrete\tvulkan\tNVIDIA\n";
    assert_eq!(parse(cut), Err(Failure::Garbled));
    let bad = "apertrust adapters 1\nnot an adapter\nend\n";
    assert_eq!(parse(bad), Err(Failure::Garbled));
    let kind = "apertrust adapters 1\nquantum\tvulkan\tQPU\nend\n";
    assert_eq!(parse(kind), Err(Failure::Garbled));
    let unnamed = "apertrust adapters 1\ndiscrete\tvulkan\t \nend\n";
    assert_eq!(parse(unnamed), Err(Failure::Garbled));
    // A newer helper's list is not this one's.
    let newer = "apertrust adapters 2\nend\n";
    assert_eq!(parse(newer), Err(Failure::Garbled));
}

/// The hidden argument, with the API to list; any other command line is the app's.
#[test]
fn the_helper_is_asked_for_by_its_argument_alone() {
    let asked = |args: &[&str]| helper_asked(args.iter().map(std::ffi::OsString::from));
    assert_eq!(asked(&["--list-adapters"]), Some(Backend::Auto));
    assert_eq!(asked(&["--list-adapters", "dx12"]), Some(Backend::Dx12));
    assert_eq!(asked(&["--list-adapters", "vulkan"]), Some(Backend::Vulkan));
    assert_eq!(asked(&["--list-adapters", "metal"]), Some(Backend::Auto));
    assert_eq!(asked(&[]), None);
    assert_eq!(asked(&["photo.CR3", "--list-adapters"]), None);
}

/// What the fake helper does, `FAKE` saying it.
const FAKE: &str = "APERTRUST_FAKE_ADAPTER_HELPER";

/// The helper's stand-in, run by the tests below as a process of its own: this test binary
/// running this test alone, which does what [`FAKE`] says. Run with the rest, it does
/// nothing. The test harness writes its own lines round the test's, as a driver might.
#[test]
fn fake_helper() {
    let Ok(what) = std::env::var(FAKE) else {
        return;
    };
    // The harness's own words end without a line break before the test's.
    println!();
    match what.as_str() {
        "list" => print!(
            "{}",
            lines(&[info(RTX, DiscreteGpu, wgpu::Backend::Vulkan)])
        ),
        "crash" => std::process::exit(3),
        // A whole list, then a driver that crashes as the helper ends.
        "listed-then-crash" => {
            print!(
                "{}",
                lines(&[info(RTX, DiscreteGpu, wgpu::Backend::Vulkan)])
            );
            std::process::exit(3)
        }
        "hang" => std::thread::sleep(Duration::from_secs(60)),
        _ => println!("Segmentation fault"),
    }
}

/// This test binary as the helper, doing `what`.
fn fake(what: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().expect("the test binary"));
    command
        .args(["--exact", "hardware::tests::fake_helper", "--nocapture"])
        .args(["--test-threads=1"])
        .env(FAKE, what);
    command
}

/// The helper's list read from its process; one that crashes, hangs past its limit or
/// prints no list is a failure the row can say, the app going on.
#[test]
fn a_helper_that_crashes_hangs_or_prints_no_list_is_a_failure() {
    let listed = run(fake("list"), Duration::from_secs(60));
    let rtx = adapter(RTX, DiscreteGpu, wgpu::Backend::Vulkan);
    assert_eq!(listed, Ok(vec![rtx]));
    let crashed = run(fake("crash"), Duration::from_secs(60));
    assert!(matches!(crashed, Err(Failure::Crashed(_))), "{crashed:?}");
    // A whole list counts whatever the helper's end.
    let listed = run(fake("listed-then-crash"), Duration::from_secs(60));
    let rtx = adapter(RTX, DiscreteGpu, wgpu::Backend::Vulkan);
    assert_eq!(listed, Ok(vec![rtx]));
    let started = Instant::now();
    let hung = run(fake("hang"), Duration::from_millis(500));
    assert_eq!(hung, Err(Failure::TimedOut));
    assert!(
        started.elapsed() < Duration::from_secs(30),
        "ended at its limit"
    );
    assert_eq!(
        run(fake("garbage"), Duration::from_secs(60)),
        Err(Failure::Garbled)
    );
    let missing = Command::new("apertrust-no-such-program");
    assert!(matches!(
        run(missing, Duration::from_secs(1)),
        Err(Failure::Start(_))
    ));
}

/// Automatic first, plain; the cards in Automatic's order, each name once
/// whichever API lists it; then a separator and CPU, never the software adapter's own name.
#[test]
fn the_list_is_automatic_then_the_cards_as_ranked_then_cpu() {
    let windows = OperatingSystem::Windows;
    let auto = listed(windows, Backend::Auto);
    let card = list(windows, "", Shown::Listed(&auto), "Automatic");
    let automatic = "Automatic";
    let cpu = "CPU (no graphics card), slow";
    assert_eq!(card.words, [automatic, RTX, RADEON, cpu]);
    assert_eq!(card.selected, Some(0));
    assert_eq!(card.separator, Some(3));
    assert_eq!((card.missing.clone(), card.unlisted.clone()), (None, None));
    assert_eq!(card.note, None);
    assert!(!card.listing);
    let stores: Vec<Option<&str>> = (0..4).map(|item| card.stores(item)).collect();
    assert_eq!(stores, [None, Some(RTX), Some(RADEON), Some("cpu")]);
    assert!(card.words.iter().all(|words| !words.contains(WARP)));
    // For low power, the integrated GPU first.
    let low = card_for(&words(), "", Shown::Listed(&auto), windows, LowPower);
    assert_eq!(low.words[..3], ["Automatic", RADEON, RTX]);
    // Two identical GPUs are one item.
    let twins = [
        adapter(RTX, DiscreteGpu, wgpu::Backend::Vulkan),
        adapter(RTX, DiscreteGpu, wgpu::Backend::Vulkan),
    ];
    let card = list(windows, "", Shown::Listed(&twins), "Vulkan");
    assert_eq!(card.words.iter().filter(|w| *w == RTX).count(), 1);
    // With nothing but the software adapter, Automatic, plain, then CPU.
    let alone = [adapter(WARP, Cpu, wgpu::Backend::Dx12)];
    let card = list(windows, "", Shown::Listed(&alone), "Direct3D 12");
    assert_eq!(card.words, ["Automatic", cpu]);
}

/// [`card`] for `preference`.
fn card_for(
    words: &Translator,
    stored: &str,
    shown: Shown<'_>,
    os: OperatingSystem,
    preference: wgpu::PowerPreference,
) -> Card {
    card(
        words,
        kept(stored),
        shown,
        Some("Automatic"),
        os,
        preference,
    )
}

/// CPU is disabled, saying why, where the API has no software adapter, as Vulkan on
/// Windows or Linux without lavapipe; the Mac, whose Metal has none, never lists it.
#[test]
fn cpu_is_disabled_where_the_api_has_none_and_not_listed_on_the_mac() {
    for os in [OperatingSystem::Windows, OperatingSystem::Nix] {
        let vulkan = listed(os, Backend::Vulkan);
        let card = list(os, "", Shown::Listed(&vulkan), "Vulkan");
        assert_eq!(card.words.len(), 4, "{os:?}");
        let missing = Some((3, "Not found with Vulkan".to_owned()));
        assert_eq!(card.missing, missing, "{os:?}");
        assert_eq!(card.separator, Some(3), "{os:?}");
        assert_eq!(card.note, None, "{os:?}");
    }
    let mac = OperatingSystem::Mac;
    let metal = listed(mac, Backend::Auto);
    let card = list(mac, "", Shown::Listed(&metal), "Metal");
    assert_eq!(card.words, ["Automatic", M4]);
    assert_eq!((card.separator, card.missing), (None, None));
}

/// The item checked is the one the stored value opens, by the engine's rule: a name typed
/// by hand checks the card that holds it, and `cpu` checks CPU.
#[test]
fn the_item_checked_is_the_one_the_stored_value_opens() {
    let windows = OperatingSystem::Windows;
    let dx12 = listed(windows, Backend::Dx12);
    let checked = |stored| list(windows, stored, Shown::Listed(&dx12), "Direct3D 12").selected;
    assert_eq!(checked(""), Some(0));
    assert_eq!(checked(RTX), Some(1));
    assert_eq!(checked(RADEON), Some(2));
    assert_eq!(checked("radeon"), Some(2));
    assert_eq!(checked("cpu"), Some(3));
    // The software adapter by its own name, as the text field may have kept it, is CPU.
    assert_eq!(checked(WARP), Some(3));
    let card = list(windows, "radeon", Shown::Listed(&dx12), "Direct3D 12");
    assert_eq!((card.unlisted, card.note), (None, None));
}

/// A stored card the API does not find is kept: listed after the cards, disabled and
/// checked, saying it is not on this computer, and the row says Automatic is used; `cpu`
/// where the API has no software adapter is CPU, disabled and checked.
#[test]
fn a_card_not_found_is_kept_disabled_and_checked() {
    let windows = OperatingSystem::Windows;
    let dx12 = listed(windows, Backend::Dx12);
    let gone = "NVIDIA GeForce RTX 4090";
    let card = list(windows, gone, Shown::Listed(&dx12), "Direct3D 12");
    let cpu = "CPU (no graphics card), slow";
    assert_eq!(card.words[1..], [RTX, RADEON, gone, cpu]);
    assert_eq!(card.selected, Some(3));
    let unlisted = Some((3, "Not found on this computer".to_owned()));
    assert_eq!(card.unlisted, unlisted);
    assert_eq!(card.stores(3), Some(gone));
    assert_eq!(card.separator, Some(4));
    let note = card.note.expect("a note");
    assert_eq!(note.text, "Graphics card not found: Automatic used");
    assert!(note.warning && !note.live);

    let vulkan = listed(windows, Backend::Vulkan);
    let card = list(windows, "cpu", Shown::Listed(&vulkan), "Vulkan");
    assert_eq!(card.selected, Some(3));
    assert_eq!(card.unlisted, Some((3, "Not found with Vulkan".to_owned())));
    assert_eq!(card.missing, None);
    let note = card.note.expect("a note").text;
    assert_eq!(note, "Graphics card not found: Automatic used");
    // On the Mac too, where CPU is listed only so.
    let mac = OperatingSystem::Mac;
    let metal = listed(mac, Backend::Auto);
    let card = list(mac, "cpu", Shown::Listed(&metal), "Metal");
    assert_eq!(card.words[2], cpu);
    assert_eq!(card.selected, Some(2));
}

/// While the cards are listed the box shows the stored choice without brackets and is
/// disabled, saying it is looking only once it has been a moment; a listing that failed
/// says the cards could not be listed and keeps what is stored.
#[test]
fn listing_and_a_listing_that_failed() {
    let windows = OperatingSystem::Windows;
    let quick = Shown::Listing { looking: false };
    let card = list(windows, "", quick, "Vulkan");
    assert_eq!(
        (card.words, card.listing, card.note),
        (vec!["Automatic".into()], true, None)
    );
    let slow = Shown::Listing { looking: true };
    let card = list(windows, RADEON, slow, "Vulkan");
    assert_eq!(card.words, [RADEON]);
    let note = card.note.expect("a note");
    assert_eq!(note.text, "Looking for graphics cards…");
    assert!(!note.warning);
    // The software adapter, by `cpu` or by its own name as the text field kept it.
    for software in ["cpu", WARP, "llvmpipe (LLVM 17.0.6, 256 bits)"] {
        let card = list(windows, software, slow, "Vulkan");
        assert_eq!(card.words, ["CPU (no graphics card), slow"], "{software}");
    }

    let failed = list(windows, RADEON, Shown::Failed, "Vulkan");
    let cpu = "CPU (no graphics card), slow";
    assert_eq!(failed.words, ["Automatic", RADEON, cpu]);
    assert_eq!(failed.selected, Some(1));
    let said = "The graphics cards could not be listed".to_owned();
    assert_eq!(failed.unlisted, Some((1, said.clone())));
    assert_eq!(failed.missing, Some((2, said.clone())));
    assert_eq!(failed.note.expect("a note"), Note::warning(said));
}

/// The adapter in use, as the window opened it, CPU where it is a software one.
#[test]
fn what_is_in_use_is_the_window_s_adapter() {
    use gpu::AdapterChoice::{Any, Named, Preferred};
    let rtx = InUse::of(&info(RTX, DiscreteGpu, wgpu::Backend::Vulkan), &Any);
    assert_eq!(rtx.name, RTX);
    assert_eq!((rtx.backend, rtx.software), (wgpu::Backend::Vulkan, false));
    // The software adapter is chosen where the choice names it, by `cpu` or by its own
    // name as the text field kept it; Automatic, or a card not found, fell to it.
    let warp = info(WARP, Cpu, wgpu::Backend::Dx12);
    let chosen = |choice| InUse::of(&warp, &choice).chosen;
    assert!(InUse::of(&warp, &Any).software);
    assert!(chosen(Preferred("cpu".into())));
    assert!(chosen(Named("CPU".into())));
    assert!(chosen(Preferred(WARP.into())));
    assert!(!chosen(Any));
    assert!(!chosen(Preferred("NVIDIA GeForce RTX 4090".into())));
    let words = words();
    let names: Vec<String> = [
        wgpu::Backend::Dx12,
        wgpu::Backend::Vulkan,
        wgpu::Backend::Metal,
        wgpu::Backend::Gl,
    ]
    .into_iter()
    .map(|backend| in_use_api(&words, backend))
    .collect();
    assert_eq!(names, ["Direct3D 12", "Vulkan", "Metal", "OpenGL"]);
    let api = |backend, os| api_words(&words, backend, os);
    assert_eq!(api(Backend::Auto, OperatingSystem::Mac), "Metal");
    assert_eq!(api(Backend::Auto, OperatingSystem::Nix), "Vulkan");
    assert_eq!(api(Backend::Dx12, OperatingSystem::Windows), "Direct3D 12");
}

/// The software adapter stored by its own name reads as CPU: by its kind where a listing
/// or the adapter in use has it, else by WARP's or lavapipe's name; a card's never.
#[test]
fn a_software_adapter_s_own_name_reads_as_cpu() {
    let none = Hardware::default();
    assert!(none.software("cpu") && none.software("CPU"));
    assert!(none.software(WARP));
    assert!(none.software("llvmpipe (LLVM 17.0.6, 256 bits)"));
    assert!(!none.software(RADEON) && !none.software(""));
    // By kind, from what is in use: a software adapter of another name.
    let mut running = running(OperatingSystem::Windows);
    running.in_use = Some(InUse {
        name: "SwiftShader Device".into(),
        backend: wgpu::Backend::Vulkan,
        software: true,
        chosen: true,
    });
    let hardware = Hardware::new(running);
    assert!(hardware.software("SwiftShader Device"));
    assert!(!hardware.software(RADEON));
    // Under an API whose list holds it, the item checked is CPU.
    let dx12 = listed(OperatingSystem::Windows, Backend::Dx12);
    let card = list(
        OperatingSystem::Windows,
        WARP,
        Shown::Listed(&dx12),
        "Direct3D 12",
    );
    assert_eq!(card.selected, Some(3));
    assert_eq!(card.note, None);
    // Under one without it, CPU, checked and disabled, as `cpu` is, never its name.
    let vulkan = listed(OperatingSystem::Windows, Backend::Vulkan);
    let card = list(
        OperatingSystem::Windows,
        WARP,
        Shown::Listed(&vulkan),
        "Vulkan",
    );
    assert_eq!(card.selected, Some(3));
    assert!(card.words.iter().all(|words| words != WARP));
}

/// Where the API Automatic stands for is not known, the words name none.
#[test]
fn an_api_not_known_is_not_named() {
    let windows = OperatingSystem::Windows;
    let vulkan = listed(windows, Backend::Vulkan);
    let shown = Shown::Listed(&vulkan);
    let card = card(&words(), kept(""), shown, None, windows, HighPerformance);
    let tip = "Not found with this graphics API".to_owned();
    assert_eq!(card.missing, Some((3, tip)));
    let said = with_api(&words(), "settings-gpu-no-adapter", None, None);
    assert_eq!(said, "No graphics card found: Automatic used");
    let said = with_api(&words(), "settings-gpu-dropped", None, Some("CPU"));
    assert_eq!(
        said,
        "CPU isn’t available with this graphics API: set to Automatic"
    );
}

/// What a run started with: the settings that apply after a restart, the three of
/// Hardware acceleration, with their values once the file and `--conf` are read. A value
/// tells a change by itself, never by a guess at what would open; a setting it does not
/// hold never waits; Keep takes a value as the run's.
#[test]
fn a_run_starts_with_the_values_of_the_settings_that_apply_after_a_restart() {
    let (mut keeper, settings) = crate::settings::Keeper::nowhere();
    keeper.apply(&settings, "engine.adapter", Some(&RADEON.into()));
    let mut started = Started::of(crate::settings::shown(), |s| keeper.value(s));
    let ids: Vec<&str> = started.0.iter().map(|(id, _)| *id).collect();
    assert_eq!(
        ids,
        ["engine.adapter", "engine.backend", "engine.memory_budget"]
    );
    let radeon = toml::Value::from(RADEON);
    let automatic = toml::Value::from("");
    assert!(!started.differs("engine.adapter", Some(&radeon)));
    assert!(started.differs("engine.adapter", Some(&automatic)));
    assert!(started.differs("engine.memory_budget", Some(&4096.into())));
    assert!(!started.differs("theme", Some(&"light".into())));
    started.keep("engine.adapter", Some(automatic.clone()));
    assert!(!started.differs("engine.adapter", Some(&automatic)));
}
