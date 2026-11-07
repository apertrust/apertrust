#!/usr/bin/env bash
# Apertrust, a free, non-destructive, cross-platform raw photo editor
# Copyright (C) 2026  Apertrust developers
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program.  If not, see <http://www.gnu.org/licenses/>.

# Times darktable's pipeline against Apertrust's on one machine: darktable-cli
# with OpenCL off and on, Apertrust on the software adapter (lavapipe on Linux, WARP
# on Windows) and on the machine's own GPU, with the pipeline timings' method: the same
# modules on both sides, the times read from each program's -d perf lines, each figure
# the median of several runs, each run a new process. Prints the tables and writes them
# as Markdown, with the machine, its drivers and both programs' versions. Runs with
# the bash macOS ships (3.2).
#
# With --add, --size or --hq it times exports through apertrust-golden's `export` instead
# (the harness's mode): the exit test's history for darktable, Apertrust's crop, levels and
# parameters written into it, golden cases' modules added to both sides, an export at a size.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
# The reference release, from the one constant the harness uses (decision 3).
reference=$(sed -n 's/^pub const REFERENCE_VERSION: &str = "\(.*\)";$/\1/p' crates/golden/src/darktable.rs)

usage() {
  cat <<'EOF'
Usage: tools/compare-cpu.sh [OPTION]...

Times, on this machine, darktable-cli with OpenCL off and on against
apertrust-cli on the software adapter (lavapipe on Linux, WARP on Windows; none on
macOS) and on the GPU, for three raws. Every time but the whole run's comes from
the programs' -d perf lines; each figure is the median of --runs runs, each a new
process. Prints the tables and writes them as Markdown with the machine and the
versions. On Linux, lavapipe is the only Vulkan driver the loader is given
(VK_DRIVER_FILES, VK_ICD_FILENAMES).

Options:
  --raws LIST            raws, comma-separated: 40d (mire1.cr2, 10 MP), 7rv (Sony
                         7R V, 61 MP), xt5 (Fujifilm X-T5, 40 MP); default all
  --paths LIST           what to time, comma-separated: dt-cpu, dt-opencl,
                         software, gpu; default all
  --runs N               runs of each measurement; 3
  --renders N            renders of a warm export; 10
  --changes N            exposure changes of a preview on the GPU; 30
  --software-changes N   the same on the software adapter; 10
  --software-adapter NAME  the software adapter, part of its name: llvmpipe on
                         Linux, Microsoft Basic Render Driver on Windows; on macOS,
                         which has neither, skipped unless named
  --lavapipe-icd FILE    Mesa's lvp_icd JSON; default the first found in the
                         Vulkan loader's folders, /usr/share/vulkan/icd.d and others
  --gpu-adapter NAME     the GPU, part of its name; default the engine's choice
  --max-binding MIB      open Apertrust's adapters binding at most MIB at once,
                         as apertrust-cli --max-binding
  --add CASES            golden cases, comma-separated, whose modules both programs
                         run on top of their defaults, with the case's parameters:
                         grain, vignette, toneequal, colorbalancergb or bilat cases,
                         such as toneequal-eigf,colorbalancergb-ucs,bilat,vignette
                         (apertrust-golden list); times exports with the harness's mode
  --size PIXELS          export within PIXELS across and down, darktable-cli's --width
                         and --height, at the same size on both sides; harness mode
  --hq BOOL              high quality resampling at a size, darktable-cli's --hq and
                         apertrust-golden's: true (default) or false; harness mode
  --wait-quiet PERCENT   before each timed run, wait (up to a minute) until at least
                         PERCENT of the CPU is idle, and run again (up to twice) a run
                         after which it is not, as other programs were taking the machine;
                         the results say how many runs ended on a busy machine anyway
                         (macOS: top's CPU usage); 0, the default, never waits
  --harness              harness mode with the defaults alone: both programs' base
                         pipeline with Apertrust's crop, levels and parameters on both
                         sides, as the exit test has it (implied by --add, --size and
                         --hq). Takes each run of the programs in turn, so that other
                         load on the machine falls on all alike, and shows each time's
                         best of the runs beside its median. Times each program's cold
                         exports only, for darktable,
                         Apertrust on the GPU: no software adapter, warm export or preview;
                         apertrust-golden export writes the PNGs, 8 bits, as the CLI
  --darktable PATH       darktable-cli; default DARKTABLE_CLI, PATH, then the
                         default install location
  --test-images DIR     darktable's integration-test images, for mire1.cr2;
                         default APERTRUST_TEST_IMAGES
  --corpus DIR           the raw.pixls.us corpus; default APERTRUST_RAW_CORPUS,
                         else test-images/raw.pixls.us
  --out DIR              logs, images and results; default target/compare-cpu
  --no-build             use target/release/apertrust-cli as it is
  -h, --help             show this

Needs, for the paths and raws asked for:
  darktable-cli (the reference release, REFERENCE_VERSION in
                         crates/golden/src/darktable.rs)
                         https://www.darktable.org/install/, or DARKTABLE_CLI; a
                         Flatpak through a wrapper script running
                         flatpak run --command=darktable-cli org.darktable.Darktable "$@"
  lavapipe               Debian, Ubuntu, Fedora: mesa-vulkan-drivers; Arch:
                         vulkan-swrast
  mire1.cr2              the images folder of darktable's integration tests,
                         https://github.com/darktable-org/darktable-tests
                         (src/tests/integration in a darktable checkout with its
                         submodules), as APERTRUST_TEST_IMAGES
  the 7R V and the X-T5  tools/fetch-raw-corpus.sh
EOF
}

die() {
  echo "compare-cpu: $*" >&2
  exit 1
}

raws="40d,7rv,xt5"
paths="dt-cpu,dt-opencl,software,gpu"
runs=3
renders=10
changes=30
software_changes=10
software_adapter=""
lavapipe_icd=""
gpu_adapter=""
max_binding=""
adds=""
quiet=0
size=""
hq=""
harness=0
darktable="${DARKTABLE_CLI:-}"
test_images="${APERTRUST_TEST_IMAGES:-}"
corpus="${APERTRUST_RAW_CORPUS:-$root/test-images/raw.pixls.us}"
out="$root/target/compare-cpu"
build=1

# Fails unless option $1's value $2 is a whole number above 0.
whole() {
  [[ "$2" =~ ^[1-9][0-9]*$ ]] || die "$1 takes a whole number above 0, not '$2'"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --no-build) build=0 ;;
    --harness) harness=1 ;;
    -h | --help)
      usage
      exit 0
      ;;
    --raws | --paths | --runs | --renders | --changes | --software-changes | \
      --software-adapter | --lavapipe-icd | --gpu-adapter | --max-binding | \
      --add | --size | --hq | --wait-quiet | --darktable | --test-images | --corpus | --out)
      [[ $# -ge 2 && -n "$2" ]] || die "$1 needs a value (see --help)"
      case "$1" in
        --raws) raws=$2 ;;
        --paths) paths=$2 ;;
        --runs) whole "$1" "$2" && runs=$2 ;;
        --renders) whole "$1" "$2" && renders=$2 ;;
        --changes) whole "$1" "$2" && changes=$2 ;;
        --software-changes) whole "$1" "$2" && software_changes=$2 ;;
        --software-adapter) software_adapter=$2 ;;
        --lavapipe-icd) lavapipe_icd=$2 ;;
        --gpu-adapter) gpu_adapter=$2 ;;
        --max-binding) whole "$1" "$2" && max_binding=$2 ;;
        --add) adds=$2 ;;
        --wait-quiet)
          [[ "$2" =~ ^[0-9]+$ ]] || die "$1 takes a whole number, not '$2'"
          quiet=$2
          ;;
        --size) whole "$1" "$2" && size=$2 ;;
        --hq)
          [[ "$2" == true || "$2" == false ]] || die "--hq takes true or false, not '$2'"
          hq=$2
          ;;
        --darktable) darktable=$2 ;;
        --test-images) test_images=$2 ;;
        --corpus) corpus=$2 ;;
        --out) out=$2 ;;
      esac
      shift
      ;;
    *) die "unknown option $1 (see --help)" ;;
  esac
  shift
done

# Whether the comma-separated list $1 holds $2.
has() { [[ ",$1," == *",$2,"* ]]; }
for raw in ${raws//,/ }; do
  case "$raw" in
    40d | 7rv | xt5) ;;
    *) die "--raws: no raw '$raw'; 40d, 7rv or xt5" ;;
  esac
done
for path in ${paths//,/ }; do
  case "$path" in
    dt-cpu | dt-opencl | software | gpu) ;;
    *) die "--paths: no path '$path'; dt-cpu, dt-opencl, software or gpu" ;;
  esac
done

if [[ -n "$adds" || -n "$size" || -n "$hq" ]]; then
  harness=1
fi
if [[ -n "$hq" && -z "$size" ]]; then
  die "--hq asks for a resample, so it needs --size"
fi
if [[ $harness == 1 ]]; then
  if has "$paths" software; then
    if [[ "$paths" == "dt-cpu,dt-opencl,software,gpu" ]]; then
      paths="dt-cpu,dt-opencl,gpu"
    else
      die "the harness mode times no software adapter: leave software out of --paths"
    fi
  fi
  [[ -z "$gpu_adapter" ]] || die "the harness mode takes the engine's own adapter, not --gpu-adapter"
  [[ -z "$max_binding" ]] || die "the harness mode has no --max-binding"
  hq=${hq:-true}
fi

case "$(uname -s)" in
  Linux) os=linux ;;
  Darwin) os=macos ;;
  MINGW* | MSYS* | CYGWIN*) os=windows ;;
  *) os=other ;;
esac

# Microseconds since the epoch: bash 5's clock, else perl's, else GNU date's.
if [[ -n "${EPOCHREALTIME:-}" ]]; then
  now_us() { printf '%s\n' "${EPOCHREALTIME//[!0-9]/}"; }
elif command -v perl >/dev/null; then
  now_us() { perl -MTime::HiRes=time -e 'printf "%.0f\n", time * 1e6'; }
else
  now_us() { date +%s%6N; }
fi

# The median of the numbers in file $1, one a line, the upper one of an even count as
# apertrust-cli takes it; nothing when there are none.
median() {
  if [[ -s "$1" ]]; then
    sort -g "$1" | awk '{ v[NR] = $1 } END { if (NR) print v[int(NR / 2) + 1] }'
  fi
}

# Milliseconds as a person reads them, in ms or in s.
fmt_ms() {
  awk -v v="$1" 'BEGIN {
    if (v >= 100) printf "%.0f ms", v; else if (v >= 10) printf "%.1f ms", v; else printf "%.2f ms", v }'
}
fmt_s() {
  awk -v v="$1" 'BEGIN { s = v / 1000
    if (s >= 100) printf "%.0f s", s; else if (s >= 10) printf "%.1f s", s; else printf "%.2f s", s }'
}
# Milliseconds from seconds.
ms_of() { awk -v s="$1" 'BEGIN { print s * 1000 }'; }
# The first line of standard input; reads it all, so the writer never gets SIGPIPE.
first_line() { sed -n 1p; }
# The machine's load averages, over 1, 5 and 15 minutes, as `uptime` gives them.
load_averages() { uptime | sed 's/.*load averages*: *//'; }

# --- What there is to time ---------------------------------------------------------

problems=()
notes=()

raw_file() {
  case "$1" in
    40d) printf '%s' "$test_images/mire1.cr2" ;;
    7rv) printf '%s' "$corpus/Sony/ILCE-7RM5/7RM5-LosslessCompressedLarge.ARW" ;;
    xt5) printf '%s' "$corpus/Fujifilm/X-T5/DSCF0021.RAF" ;;
  esac
}
raw_label() {
  case "$1" in
    40d) printf '40D, 10 MP' ;;
    7rv) printf '7R V, 61 MP' ;;
    xt5) printf 'X-T5, 40 MP' ;;
  esac
}
for raw in ${raws//,/ }; do
  file=$(raw_file "$raw")
  if [[ $raw == 40d && -z "$test_images" ]]; then
    problems+=("mire1.cr2: set APERTRUST_TEST_IMAGES (or --test-images) to the images folder of darktable's integration tests, https://github.com/darktable-org/darktable-tests (src/tests/integration in a darktable checkout with its submodules)")
  elif [[ ! -f "$file" && $raw == 40d ]]; then
    problems+=("no $file: APERTRUST_TEST_IMAGES should be the images folder of https://github.com/darktable-org/darktable-tests")
  elif [[ ! -f "$file" ]]; then
    problems+=("no $file: run tools/fetch-raw-corpus.sh, or set APERTRUST_RAW_CORPUS (or --corpus) to the raw.pixls.us corpus")
  fi
done

dt_version=""
if has "$paths" dt-cpu || has "$paths" dt-opencl; then
  if [[ -z "$darktable" ]]; then
    darktable=$(command -v darktable-cli || true)
  fi
  if [[ -z "$darktable" ]]; then
    for candidate in /Applications/darktable.app/Contents/MacOS/darktable-cli \
      /opt/darktable/bin/darktable-cli "/c/Program Files/darktable/bin/darktable-cli.exe"; do
      if [[ -x "$candidate" ]]; then
        darktable=$candidate
        break
      fi
    done
  fi
  if [[ -z "$darktable" || ! -x "$darktable" ]]; then
    problems+=("darktable-cli not found: install darktable $reference (https://www.darktable.org/install/), or set DARKTABLE_CLI (or --darktable) to its darktable-cli; --help says how for a Flatpak")
  else
    dt_version=$("$darktable" --version 2>/dev/null | first_line || true)
    if [[ "$dt_version" != "darktable $reference" && "$dt_version" != "darktable $reference "* ]]; then
      problems+=("$darktable is '${dt_version:-not darktable-cli}', and the comparison needs darktable $reference, the reference release: https://www.darktable.org/install/, or DARKTABLE_CLI set to one")
    fi
  fi
fi

mkdir -p "$out"
rm -f "$out/idle.tsv"
# A user data folder of the comparison's own, empty, so a person's [engine] settings (the
# adapter, the memory budget) never change its timings; the CLI only reads it (decision
# 43), so it must exist.
export APERTRUST_USER_DATA_DIR="$out/user-data"
mkdir -p "$APERTRUST_USER_DATA_DIR"
cli="$root/target/release/apertrust-cli"
if [[ $build == 1 ]]; then
  echo "building apertrust-cli" >&2
  cargo build --release --quiet -p apertrust-cli
  if [[ $harness == 1 ]]; then
    cargo build --release --quiet -p apertrust-golden
  fi
fi
[[ -x "$cli" || -x "$cli.exe" ]] || die "no $cli: build it with cargo build --release -p apertrust-cli"
golden="$root/target/release/apertrust-golden"
if [[ $harness == 1 ]]; then
  [[ -x "$golden" || -x "$golden.exe" ]] || die "no $golden: build it with cargo build --release -p apertrust-golden"
  golden_version=$("$golden" --version | first_line)
fi
apertrust_version=$("$cli" --version | first_line)

# Mesa's lavapipe driver manifest, from the Vulkan loader's folders.
find_lavapipe_icd() {
  local dir file
  for dir in /usr/share/vulkan/icd.d /etc/vulkan/icd.d /usr/local/share/vulkan/icd.d \
    /usr/local/etc/vulkan/icd.d; do
    for file in "$dir/lvp_icd.$(uname -m).json" "$dir"/lvp_icd*.json; do
      if [[ -f "$file" ]]; then
        printf '%s' "$file"
        return 0
      fi
    done
  done
  return 1
}

# Runs apertrust-cli on path $1 (software or gpu) with the rest as its arguments: on the
# path's adapter, with --max-binding and the drivers' caches as asked.
apertrust() {
  local path=$1 adapter=""
  shift
  local environment=(env) options=()
  case "$path" in
    software)
      adapter=$software_adapter
      case "$os" in
        linux)
          environment+=(WGPU_BACKEND=vulkan)
          if [[ -n "$lavapipe_icd" ]]; then
            environment+=(VK_DRIVER_FILES="$lavapipe_icd" VK_ICD_FILENAMES="$lavapipe_icd")
          fi
          ;;
        windows) environment+=(WGPU_BACKEND=dx12) ;;
      esac
      ;;
    gpu) adapter=$gpu_adapter ;;
  esac
  if [[ -n "$adapter" ]]; then
    options+=(--adapter "$adapter")
  fi
  if [[ -n "$max_binding" ]]; then
    options+=(--max-binding "$max_binding")
  fi
  "${environment[@]}" "$cli" "$@" ${options[@]+"${options[@]}"}
}

# The adapter a run's -d gpu log, file $1, says it chose: `Apple M4 (Metal, integrated)`.
chosen() {
  sed -n 's/.*\[gpu\] adapter: \(.*\)  <- chosen$/\1/p' "$1" | first_line
}

# What `apertrust-cli -d gpu gpu` says of the adapter path $1 opens, into <path>_chosen
# (its name, backend and kind), <path>_name, <path>_label (the name before its own
# brackets, and the backend and kind), <path>_driver and <path>_limits.
adapter_info() {
  local path=$1 log="$out/$1-gpu.log" adapter
  apertrust "$path" -d gpu gpu >"$log" 2>&1 || return 1
  adapter=$(chosen "$log")
  [[ -n "$adapter" ]] || return 1
  printf -v "${path}_chosen" '%s' "$adapter"
  printf -v "${path}_name" '%s' "${adapter% (*}"
  printf -v "${path}_label" '%s (%s' "${adapter%% (*}" "${adapter##* (}"
  printf -v "${path}_driver" '%s' "$(sed -n 's/.*\[gpu\] driver: //p' "$log" | first_line)"
  printf -v "${path}_limits" '%s' "$(sed -n 's/.*\[gpu\] limits: //p' "$log" | first_line)"
}

software_chosen="" software_name="" software_label="" software_driver="" software_limits=""
gpu_chosen="" gpu_name="" gpu_label="" gpu_driver="" gpu_limits=""
want_software=0
want_gpu=0
if has "$paths" software; then
  if [[ -z "$software_adapter" ]]; then
    case "$os" in
      linux) software_adapter=llvmpipe ;;
      windows) software_adapter="Microsoft Basic Render Driver" ;;
    esac
  fi
  if [[ $os == linux && -z "$lavapipe_icd" ]]; then
    lavapipe_icd=$(find_lavapipe_icd || true)
  fi
  if [[ -z "$software_adapter" ]]; then
    notes+=("Software adapter: skipped, since wgpu has neither lavapipe nor WARP on macOS.")
  elif [[ $os == linux && ! -f "$lavapipe_icd" ]]; then
    problems+=("lavapipe not found (no lvp_icd JSON in /usr/share/vulkan/icd.d or /etc/vulkan/icd.d): install mesa-vulkan-drivers on Debian, Ubuntu and Fedora, vulkan-swrast on Arch; or name its JSON with --lavapipe-icd")
  elif ! adapter_info software; then
    problems+=("no software adapter named '$software_adapter': see $out/software-gpu.log${lavapipe_icd:+ (the Vulkan loader was given only $lavapipe_icd)}")
  elif [[ "$software_chosen" != *", software)" && $os != macos ]]; then
    problems+=("'$software_adapter' chose $software_chosen, not a software adapter: see $out/software-gpu.log")
  else
    want_software=1
  fi
fi
if has "$paths" gpu; then
  if ! adapter_info gpu; then
    problems+=("no GPU adapter${gpu_adapter:+ named '$gpu_adapter'}: see $out/gpu-gpu.log")
  elif [[ "$gpu_chosen" == *", software)" && -z "$gpu_adapter" ]]; then
    notes+=("GPU: skipped, since the engine's own choice is a software adapter, $gpu_chosen; --gpu-adapter names another.")
  else
    want_gpu=1
  fi
fi

if [[ ${#problems[@]} -gt 0 ]]; then
  echo "compare-cpu: cannot start:" >&2
  for problem in "${problems[@]}"; do
    echo "  - $problem" >&2
  done
  exit 1
fi

# --- darktable ----------------------------------------------------------------------

# darktable's history: test 0174's (crates/golden/cases/agx-default.xmp) less rawprepare
# and temperature, so that each raw gets darktable's own defaults for them, with
# highlight reconstruction and orientation off. Color calibration stays on, as Apertrust
# runs it on every colour raw (decision 27). Its demosaic asks for RCD, which darktable
# turns into Markesteijn 1-pass on X-Trans.
pipeline_xmp() {
  local source="$root/crates/golden/cases/agx-default.xmp"
  awk -v drop=rawprepare,temperature -v off=highlights,flip '
    function operation(entry) {
      if (match(entry, /darktable:operation="[^"]*"/)) return substr(entry, RSTART + 21, RLENGTH - 22)
      return ""
    }
    function number(entry) {
      if (match(entry, /darktable:num="[0-9]+"/)) return substr(entry, RSTART + 15, RLENGTH - 16) + 0
      return -1
    }
    BEGIN {
      n = split(drop, list, ","); for (i = 1; i <= n; i++) dropped[list[i]] = 1
      n = split(off, list, ","); for (i = 1; i <= n; i++) disabled[list[i]] = 1
    }
    FNR == 1 { pass++ }
    # The first pass counts the entries kept before the history end.
    pass == 1 {
      if (match($0, /darktable:history_end="[0-9]+"/)) end = substr($0, RSTART + 23, RLENGTH - 24) + 0
      if ($0 ~ /^[[:space:]]*<rdf:li[[:space:]]*$/) { inside = 1; entry = "" }
      if (inside) {
        entry = entry $0 "\n"
        if ($0 ~ /\/>[[:space:]]*$/) {
          inside = 0
          if (!(operation(entry) in dropped) && number(entry) < end) kept++
        }
      }
      next
    }
    !inside && /darktable:history_end="[0-9]+"/ {
      sub(/darktable:history_end="[0-9]+"/, "darktable:history_end=\"" kept + 0 "\"")
    }
    !inside && /^[[:space:]]*<rdf:li[[:space:]]*$/ { inside = 1; entry = "" }
    !inside { print; next }
    {
      entry = entry $0 "\n"
      if ($0 !~ /\/>[[:space:]]*$/) next
      inside = 0
      name = operation(entry)
      if (name in dropped) next
      if (name in disabled) sub(/darktable:enabled="1"/, "darktable:enabled=\"0\"", entry)
      sub(/darktable:num="[0-9]+"/, "darktable:num=\"" renumbered++ "\"", entry)
      printf "%s", entry
    }
  ' "$source" "$source"
}

# In the harness mode darktable's history is written per raw by apertrust-golden export.
xmp="$out/pipeline.xmp"
if [[ $harness == 0 ]]; then
  pipeline_xmp >"$xmp"
  for operation in rawprepare temperature; do
    if grep -q "darktable:operation=\"$operation\"" "$xmp"; then
      die "darktable's history still has $operation: $xmp"
    fi
  done
  if [[ $(grep -c 'darktable:enabled="0"' "$xmp") != 3 ]]; then
    die "darktable's history should have three modules off: $xmp"
  fi
fi

values="$out/values"
rm -rf "$values"
mkdir -p "$values" "$out/logs" "$out/images"

# Records a measurement: raw, path, metric, value.
record() { printf '%s\n' "$4" >>"$values/$1.$2.$3"; }

# Records each `NAME MS` line of standard input as raw $1, path $2, metric $3.NAME.
record_each() {
  local name ms
  while read -r name ms; do
    record "$1" "$2" "$3.$name" "$ms"
  done
}

# The share of the CPU that is idle, in whole percent, over a second (macOS: top's second
# sample); 100 where it cannot be told.
idle_percent() {
  local idle
  idle=$(top -l 2 -n 0 -s 1 2>/dev/null | sed -n 's/.*CPU usage:.* \([0-9]*\)\.[0-9]*% idle.*/\1/p' | tail -n 1)
  echo "${idle:-100}"
}

# With --wait-quiet, waits until the CPU is at least that idle, for a minute at most.
settle() {
  local waited=0
  [[ $quiet -gt 0 ]] || return 0
  while [[ $(idle_percent) -lt $quiet && $waited -lt 60 ]]; do
    sleep 3
    waited=$((waited + 4))
  done
}

# Runs the command after $1 with its output in log $1, and prints its wall time in ms. With
# --wait-quiet it runs it on a quiet machine, again (twice at most) when the machine was
# not quiet after it, which says so on standard error; the idle share after the run it
# keeps is appended to $out/idle.tsv with its log.
timed() {
  local log=$1 start end tries=0 idle=100
  shift
  while :; do
    settle
    start=$(now_us)
    "$@" >"$log" 2>&1 || return 1
    end=$(now_us)
    tries=$((tries + 1))
    [[ $quiet -gt 0 ]] || break
    idle=$(idle_percent)
    if [[ $idle -ge $quiet || $tries -ge 3 ]]; then
      printf '%s\t%s\n' "$idle" "$log" >>"$out/idle.tsv"
      break
    fi
    echo "    the machine was busy ($idle% idle); running again: $log" >&2
  done
  echo $(((end - start) / 1000))
}

# Each module's time in a darktable -d perf log, file $1, as `NAME MS` lines in the
# pipeline's order:
#   [dev_pixelpipe] took 0.137 secs (1.238 CPU) [export] processed `agx' on CPU
# and its Lab conversions, added up as `lab`:
#   [dt_ioppr_transform_image_colorspace] IOP_CS_LAB-->IOP_CS_RGB took 0.005 secs (…) [agx]
darktable_modules() {
  awk '
    function took() { match($0, /took [0-9.]+ secs/); return substr($0, RSTART + 5, RLENGTH - 10) * 1000 }
    /\[dev_pixelpipe\] took .* processed `/ {
      match($0, /processed `[a-z0-9_]+/)
      name = substr($0, RSTART + 11, RLENGTH - 11)
      if (!(name in ms)) order[++n] = name
      ms[name] += took()
    }
    /\[dt_ioppr_transform_image_colorspace/ && /took [0-9.]+ secs/ { lab += took(); labs++ }
    END {
      for (i = 1; i <= n; i++) print order[i], ms[order[i]]
      if (labs) print "lab", lab
    }' "$1"
}

dt_device=""
# The runs a call of time_darktable and time_apertrust makes, and whether it makes the
# warm-up first: all of them by default, one at a time in the harness mode.
first_run=1
last_run=$runs
warm=1
# The raws and paths whose warm-up failed or was skipped, as `RAW.PATH`, to leave out.
left_out=" "

# One darktable export of raw $1 with OpenCL $2 (cpu or opencl), run $3, 0 the warm-up.
# Returns 2 when OpenCL was asked for and darktable ran every module on the CPU.
darktable_run() {
  local raw=$1 mode=$2 run=$3 file log image ms switches sized
  file=$(raw_file "$raw")
  log="$out/logs/$raw.dt-$mode.$run.log"
  image="$out/images/$raw.dt-$mode.png"
  if [[ $mode == cpu ]]; then
    switches=(--disable-opencl)
  elif [[ $run == 0 ]]; then
    # The warm-up names the device it profiled, which darktable says only under -d
    # opencl; the counted runs leave its events unprofiled.
    switches=(--conf opencl=TRUE -d opencl)
  else
    switches=(--conf opencl=TRUE)
  fi
  # darktable-cli's own options come before --core, which starts darktable's.
  sized=()
  if [[ -n "$size" ]]; then
    sized=(--width "$size" --height "$size" --hq "$hq" --upscale false)
  fi
  rm -f "$image"
  if ! ms=$(timed "$log" "$darktable" "$file" "$xmp" "$image" ${sized[@]+"${sized[@]}"} --core \
    --configdir "$out/darktable-$mode" "${switches[@]}" -d perf) ||
    [[ ! -f "$image" ]]; then
    echo "    darktable failed: $log" >&2
    return 1
  fi
  local seconds loading on_gpu
  seconds=$(sed -n 's/.*\[dev_process_export\] pixel pipeline processing took \([0-9.]*\) secs.*/\1/p' "$log" | tail -n 1)
  loading=$(sed -n 's/.*\[dt_dev_load_raw\] loading the image\. took \([0-9.]*\) secs.*/\1/p' "$log" | tail -n 1)
  if [[ -z "$seconds" ]]; then
    echo "    no pipeline time in $log" >&2
    return 1
  fi
  on_gpu=$(grep -c "processed \`.*' on GPU" "$log" || true)
  if [[ $mode == opencl && $on_gpu == 0 ]]; then
    return 2
  elif [[ $mode == opencl && -z "$dt_device" ]]; then
    dt_device=$(sed -n "s/.*profiling device [0-9]* ('\(.*\)').*/\1/p" "$log" | first_line)
  elif [[ $mode == cpu && $on_gpu != 0 ]]; then
    echo "    darktable used OpenCL with it off: $log" >&2
    return 1
  fi
  if [[ $run != 0 ]]; then
    record "$raw" "dt-$mode" whole "$ms"
    record "$raw" "dt-$mode" pipeline "$(ms_of "$seconds")"
    if [[ -n "$loading" ]]; then
      record "$raw" "dt-$mode" decode "$(ms_of "$loading")"
    fi
    darktable_modules "$log" | record_each "$raw" "dt-$mode" op
    printf '    run %s: %s, pipeline %s\n' "$run" "$(fmt_s "$ms")" \
      "$(fmt_ms "$(ms_of "$seconds")")" >&2
  fi
}

time_darktable() {
  local raw=$1 mode=$2 run status=0 label
  label="darktable, $([[ $mode == cpu ]] && echo CPU || echo OpenCL)"
  [[ "$left_out" != *" $raw.dt-$mode "* ]] || return 0
  if [[ $warm == 1 ]]; then
    echo "  $label: warm-up" >&2
    darktable_run "$raw" "$mode" 0 || status=$?
    if [[ $status == 2 ]]; then
      notes+=("$label: skipped for $(raw_label "$raw"), since darktable ran every module on the CPU with OpenCL on: no OpenCL device, or none darktable could use (darktable-cltest says why; $out/logs/$raw.dt-opencl.0.log).")
      left_out="$left_out$raw.dt-$mode "
      return 0
    elif [[ $status != 0 ]]; then
      notes+=("$label: failed on $(raw_label "$raw"); see $out/logs/$raw.dt-$mode.0.log.")
      left_out="$left_out$raw.dt-$mode "
      return 0
    fi
  fi
  for ((run = first_run; run <= last_run; run++)); do
    if ! darktable_run "$raw" "$mode" "$run"; then
      notes+=("$label: run $run failed on $(raw_label "$raw"); see $out/logs/$raw.dt-$mode.$run.log.")
      left_out="$left_out$raw.dt-$mode "
      return 0
    fi
  done
}

# --- Apertrust ----------------------------------------------------------------------

# Each operation's GPU time in an apertrust-cli -d perf -d verbose log, file $1, as
# `NAME MS` lines: its passes added up, over the raw stage and every tile,
#   [perf] pass agx: 3.7 ms
#   [perf] pass demosaic rcd_step_1: 2.1 ms (20×)
apertrust_passes() {
  awk '
    /\[perf\] pass [a-z0-9_]+( [a-z0-9_]+)?: [0-9.]+ ms/ {
      line = $0
      sub(/.*\[perf\] pass /, "", line)
      name = line
      sub(/[ :].*/, "", name)
      match(line, /: [0-9.]+ ms/)
      if (!(name in ms)) order[++n] = name
      ms[name] += substr(line, RSTART + 2, RLENGTH - 5)
    }
    END { for (i = 1; i <= n; i++) print order[i], ms[order[i]] }' "$1"
}

# Each operation's median GPU time in an `apertrust-cli -d perf export --runs` log, file
# $1, as `NAME MS` lines: `[perf] median agx: 26.5 ms GPU`.
apertrust_medians() {
  sed -n 's/.*\[perf\] median \([a-z0-9_]*\): \([0-9.]*\) ms GPU$/\1 \2/p' "$1"
}

# Fails, with a note, unless the -d gpu log $3 of a run on path $2 chose the adapter
# apertrust-cli -d gpu gpu chose there; $1 names the run.
check_adapter() {
  local what=$1 log=$3 expected="${2}_chosen" got
  expected=${!expected}
  got=$(chosen "$log")
  if [[ "$got" != "$expected" ]]; then
    echo "    $what ran on ${got:-no adapter its log names}, not $expected: $log" >&2
    notes+=("Apertrust: $what ran on ${got:-an adapter its log does not name}, not $expected; see $log.")
    return 1
  fi
}

# The flags apertrust-golden export takes after its history file, for the harness mode:
# the cases' modules and the size.
harness_flags=()
if [[ -n "$adds" ]]; then
  for case_name in ${adds//,/ }; do
    harness_flags+=(--add "$case_name")
  done
fi
if [[ -n "$size" ]]; then
  harness_flags+=(--width "$size" --height "$size" --hq "$hq")
fi
# The history file apertrust-golden export writes for the next export, once, in the
# harness mode: darktable's history for the raw, which the export only writes.
export_history=()

# Exports raw file $2 to image $3 on path $1, with -d perf and -d gpu and the debug domains
# after the three: apertrust-cli export with the highlights out (darktable's history has
# them off), or in the harness mode apertrust-golden export, the exit test's modules.
apertrust_export() {
  local path=$1 file=$2 image=$3
  shift 3
  if [[ $harness == 1 ]]; then
    "$golden" -d perf -d gpu "$@" export "$file" -o "$image" \
      ${export_history[@]+"${export_history[@]}"} ${harness_flags[@]+"${harness_flags[@]}"}
  else
    apertrust "$path" -d perf -d gpu --conf export.disk.on_conflict=overwrite "$@" export "$file" -o "$image" --skip highlights
  fi
}

# Times raw $1 through apertrust-cli on path $2 (software or gpu).
time_apertrust() {
  local raw=$1 path=$2 file run log ms changes_here=$changes image
  local label="${2}_label" name="${2}_name"
  label=${!label}
  name=${!name}
  file=$(raw_file "$raw")
  if [[ $path == software ]]; then
    changes_here=$software_changes
  fi
  image="$out/images/$raw.$path.png"

  [[ "$left_out" != *" $raw.$path "* ]] || return 0
  if [[ $warm == 1 ]]; then
    echo "  Apertrust, $label: warm-up export" >&2
    log="$out/logs/$raw.$path.export.0.log"
    if [[ $harness == 1 ]]; then
      export_history=(--darktable-xmp "$xmp")
    fi
    if ! timed "$log" apertrust_export "$path" "$file" "$image" >/dev/null; then
      echo "    failed: $log" >&2
      notes+=("Apertrust, $label: failed on $(raw_label "$raw"); see $log.")
      left_out="$left_out$raw.$path "
      return 0
    fi
    check_adapter "the warm-up export of $(raw_label "$raw")" "$path" "$log" || {
      left_out="$left_out$raw.$path "
      return 0
    }
    export_history=()
  fi

  # A cold export:
  #   [perf] decoded 7RM5-LosslessCompressedLarge.ARW in 160 ms
  #   [perf] compiled 17 kernels in 35 ms, passes: 17
  #   [perf] rendered 17 passes from rawprepare in 725.3 ms, GPU: 274.3 ms
  # or, where it tiles, after the raw stage's `rendered` line,
  #   [perf] exported 20 tiles in 1195 ms, rendering: 632.1 ms, GPU: 610.3 ms
  local decoded compiled tiled wall gpu tiles
  for ((run = first_run; run <= last_run; run++)); do
    log="$out/logs/$raw.$path.export.$run.log"
    if ! ms=$(timed "$log" apertrust_export "$path" "$file" "$image" -d verbose); then
      echo "    cold export failed: $log" >&2
      notes+=("Apertrust, $label: a cold export failed on $(raw_label "$raw"); see $log.")
      return 0
    fi
    check_adapter "cold export $run of $(raw_label "$raw")" "$path" "$log" || return 0
    decoded=$(sed -n 's/.*\[perf\] decoded .* in \([0-9]*\) ms$/\1/p' "$log" | tail -n 1)
    compiled=$(sed -n 's/.*\[perf\] compiled [0-9]* kernels* in \([0-9]*\) ms.*/\1/p' "$log" |
      awk '{ s += $1 } END { print s + 0 }')
    tiled=$(sed -n 's/.*\[perf\] exported \([0-9]*\) tiles* in [0-9]* ms, rendering: \([0-9.]*\) ms\(, GPU: \([0-9.]*\) ms\)\{0,1\}$/\1 \2 \4/p' "$log" | tail -n 1)
    wall="" gpu="" tiles=""
    if [[ -n "$tiled" ]]; then
      read -r tiles wall gpu <<<"$tiled"
    else
      read -r wall gpu <<<"$(sed -n 's/.*\[perf\] rendered [0-9]* passes* from [a-z0-9_]* in \([0-9.]*\) ms\(, GPU: \([0-9.]*\) ms\)\{0,1\}$/\1 \3/p' "$log" | tail -n 1)"
    fi
    if [[ -z "$wall" ]]; then
      echo "    no render time in $log" >&2
      notes+=("Apertrust, $label: no render time in $log.")
      return 0
    fi
    record "$raw" "$path" whole "$ms"
    record "$raw" "$path" compiled "$compiled"
    record "$raw" "$path" pipeline "$wall"
    if [[ -n "$decoded" ]]; then
      record "$raw" "$path" decode "$decoded"
    fi
    if [[ -n "$gpu" ]]; then
      record "$raw" "$path" pipeline_gpu "$gpu"
    fi
    if [[ -n "$tiles" ]]; then
      record "$raw" "$path" tiles "$tiles"
    fi
    apertrust_passes "$log" | record_each "$raw" "$path" op
    printf '    cold export %s: %s, compiling %s, rendering %s\n' "$run" "$(fmt_s "$ms")" \
      "$(fmt_ms "$compiled")" "$(fmt_ms "$wall")" >&2
  done

  if [[ $harness == 1 ]]; then
    return 0
  fi

  # A warm export, which renders the image whole, so only on an adapter that binds it:
  #   [perf] median of 10 renders: 265.5 ms, GPU: 274.3 ms
  #   [perf] median agx: 22.4 ms GPU
  local medians refused
  for ((run = 1; run <= runs; run++)); do
    log="$out/logs/$raw.$path.warm.$run.log"
    if ! timed "$log" apertrust "$path" -d perf -d gpu --conf export.disk.on_conflict=overwrite export "$file" -o "$image" \
      --skip highlights --runs "$renders" >/dev/null; then
      refused=$(sed -n 's/^Error: \(.*more than the GPU binds.*\)/\1/p' "$log" | first_line)
      if [[ -n "$refused" ]]; then
        echo "    no warm export: $refused" >&2
        notes+=("Apertrust, $label: no warm export of $(raw_label "$raw"), since \`export --runs\` renders the image whole and here $refused.")
      else
        echo "    warm export failed: $log" >&2
        notes+=("Apertrust, $label: a warm export failed on $(raw_label "$raw"); see $log.")
      fi
      break
    fi
    check_adapter "warm export $run of $(raw_label "$raw")" "$path" "$log" || break
    medians=$(sed -n 's/.*\[perf\] median of [0-9]* renders: \([0-9.]*\) ms, GPU: \([0-9.NaN]*\) ms.*/\1 \2/p' "$log" | tail -n 1)
    wall="" gpu=""
    read -r wall gpu <<<"$medians"
    if [[ -z "$wall" ]]; then
      echo "    no median render in $log" >&2
      notes+=("Apertrust, $label: no median render in $log.")
      break
    fi
    record "$raw" "$path" warm "$wall"
    if [[ -n "$gpu" && "$gpu" != NaN ]]; then
      record "$raw" "$path" warm_gpu "$gpu"
    fi
    apertrust_medians "$log" | record_each "$raw" "$path" warmop
    printf '    warm export %s: median of %s renders %s\n' "$run" "$renders" "$(fmt_ms "$wall")" >&2
  done

  # The viewport, from what preview prints:
  #   GPU: Apple M4
  #   Decoded in 111 ms. First render, 2400×1600, the raw stage and demosaic at full
  #   resolution: 89.9 ms (rawprepare 1.68 ms GPU, …)
  #   Per exposure change, median of 30: 5.73 ms submit to done (min 5.60)
  local first change size used
  for ((run = 1; run <= runs; run++)); do
    log="$out/logs/$raw.$path.preview.$run.log"
    if ! timed "$log" apertrust "$path" preview "$file" --changes "$changes_here" >/dev/null; then
      echo "    preview failed: $log" >&2
      notes+=("Apertrust, $label: a preview failed on $(raw_label "$raw"); see $log.")
      return 0
    fi
    used=$(sed -n 's/^GPU: //p' "$log" | first_line)
    if [[ "$used" != "$name" ]]; then
      echo "    preview $run ran on '$used': $log" >&2
      notes+=("Apertrust: preview $run of $(raw_label "$raw") ran on '$used', not $label; see $log.")
      return 0
    fi
    first=$(sed -n 's/.*First render, [^:]*: \([0-9.]*\) ms.*/\1/p' "$log")
    change=$(sed -n 's/^Per exposure change, median of [0-9]*: \([0-9.]*\) ms.*/\1/p' "$log")
    size=$(sed -n 's/.*First render, \([0-9]*\)[^0-9,]*\([0-9]*\),.*/\1 × \2/p' "$log")
    if [[ -z "$first" || -z "$change" ]]; then
      echo "    no times in $log" >&2
      notes+=("Apertrust, $label: no viewport times in $log.")
      return 0
    fi
    record "$raw" "$path" first "$first"
    record "$raw" "$path" change "$change"
    printf '%s\n' "$size" >"$values/$raw.$path.size"
    printf '    preview %s: first render %s, exposure change %s\n' "$run" "$(fmt_ms "$first")" \
      "$(fmt_ms "$change")" >&2
  done
}

# --- Run ----------------------------------------------------------------------------

load_before=$(load_averages)
echo "logs in $out/logs" >&2
for raw in ${raws//,/ }; do
  echo "$(raw_label "$raw"): $(raw_file "$raw")" >&2
  if [[ $harness == 1 ]]; then
    # darktable's history is Apertrust's for this raw, written by a first export, which
    # is also the one Apertrust's warm-up. The history must exist before darktable runs.
    xmp="$out/history-$raw.xmp"
    # The warm-ups in turn, then each run of every path in turn.
    first_run=1 last_run=0 warm=1
    if [[ $want_gpu == 1 ]]; then
      time_apertrust "$raw" gpu
    else
      "$golden" export "$(raw_file "$raw")" -o "$out/images/$raw.history.png" \
        --darktable-xmp "$xmp" ${harness_flags[@]+"${harness_flags[@]}"} >/dev/null 2>&1 ||
        die "apertrust-golden export failed for $(raw_label "$raw")"
    fi
    has "$paths" dt-cpu && time_darktable "$raw" cpu
    has "$paths" dt-opencl && time_darktable "$raw" opencl
    warm=0
    for ((turn = 1; turn <= runs; turn++)); do
      echo "  run $turn of $runs" >&2
      first_run=$turn last_run=$turn
      has "$paths" dt-cpu && time_darktable "$raw" cpu
      has "$paths" dt-opencl && time_darktable "$raw" opencl
      [[ $want_gpu == 0 ]] || time_apertrust "$raw" gpu
    done
    first_run=1 last_run=$runs warm=1
    continue
  fi
  if has "$paths" dt-cpu; then
    time_darktable "$raw" cpu
  fi
  if has "$paths" dt-opencl; then
    time_darktable "$raw" opencl
  fi
  if [[ $want_software == 1 ]]; then
    time_apertrust "$raw" software
  fi
  if [[ $want_gpu == 1 && $harness == 0 ]]; then
    time_apertrust "$raw" gpu
  fi
done

# --- The machine --------------------------------------------------------------------

cpu="" cores="" threads="" memory="" system=""
case "$os" in
  linux)
    cpu=$(sed -n 's/^model name[[:space:]]*: //p' /proc/cpuinfo | first_line)
    if [[ -z "$cpu" ]] && command -v lscpu >/dev/null; then
      cpu=$(lscpu | sed -n 's/^Model name:[[:space:]]*//p' | first_line)
    fi
    threads=$(getconf _NPROCESSORS_ONLN 2>/dev/null || true)
    if command -v lscpu >/dev/null; then
      cores=$( (lscpu -p=core,socket 2>/dev/null || true) | (grep -v '^#' || true) | sort -u | wc -l | tr -d ' ')
      [[ "$cores" != 0 ]] || cores=""
    fi
    memory=$(awk '/^MemTotal:/ { printf "%.0f GB", $2 / 1048576 }' /proc/meminfo)
    if [[ -r /etc/os-release ]]; then
      system=$(. /etc/os-release && printf '%s' "${PRETTY_NAME:-${NAME:-}}")
    fi
    system="${system:-Linux}, kernel $(uname -r)"
    ;;
  macos)
    cpu=$(sysctl -n machdep.cpu.brand_string)
    cores=$(sysctl -n hw.physicalcpu)
    threads=$(sysctl -n hw.logicalcpu)
    memory=$(awk -v b="$(sysctl -n hw.memsize)" 'BEGIN { printf "%.0f GB", b / 1073741824 }')
    system="macOS $(sw_vers -productVersion), Darwin $(uname -r)"
    ;;
  *)
    threads=${NUMBER_OF_PROCESSORS:-}
    system="$(uname -s) $(uname -r)"
    ;;
esac
cpu=$(printf '%s' "${cpu:-an unknown CPU}" | sed 's/  */ /g; s/ *$//')
counts=""
[[ -z "$cores" ]] || counts="$cores cores"
[[ -z "$threads" ]] || counts="${counts:+$counts, }$threads threads"
machine="$cpu${counts:+ ($counts)}${memory:+, $memory}, $system"

# --- The tables ---------------------------------------------------------------------

path_label() {
  case "$1" in
    dt-cpu) printf 'darktable, CPU' ;;
    dt-opencl) printf 'darktable, OpenCL' ;;
    software) printf 'Apertrust, %s' "$software_label" ;;
    gpu) printf 'Apertrust, %s' "$gpu_label" ;;
  esac
}
# The median of metric $3 of raw $1 on path $2, formatted by $4; "-" for none.
cell() {
  local value
  value=$(median "$values/$1.$2.$3")
  if [[ -n "$value" ]]; then "$4" "$value"; else printf -- '-'; fi
}
# The smallest number in file $1, one a line; nothing when there are none.
best() {
  if [[ -s "$1" ]]; then
    sort -g "$1" | sed -n 1p
  fi
}
# The best of metric $3 of raw $1 on path $2, formatted by $4; "-" for none.
cell_best() {
  local value
  value=$(best "$values/$1.$2.$3")
  if [[ -n "$value" ]]; then "$4" "$value"; else printf -- '-'; fi
}
# A time with its GPU time beside it: metric $3 and $3_gpu of raw $1 on path $2.
with_gpu() {
  local text gpu
  text=$(cell "$1" "$2" "$3" fmt_ms)
  gpu=$(median "$values/$1.$2.$3_gpu")
  [[ -z "$gpu" ]] || text="$text (GPU $(fmt_ms "$gpu"))"
  printf '%s' "$text"
}

# Prints tab-separated rows, the first the header, as aligned columns.
aligned() {
  awk -F '\t' '
    {
      for (i = 1; i <= NF; i++) { cell[NR, i] = $i; if (length($i) > width[i]) width[i] = length($i) }
      if (NF > columns) columns = NF
    }
    END {
      for (r = 1; r <= NR; r++) {
        line = ""
        for (i = 1; i < columns; i++) line = line sprintf("%-" width[i] "s  ", cell[r, i])
        print line cell[r, columns]
      }
    }'
}
# Prints tab-separated rows, the first the header, as a Markdown table, "-" as "—".
markdown_table() {
  awk -F '\t' '
    {
      line = "|"
      for (i = 1; i <= NF; i++) line = line " " ($i == "-" ? "—" : $i) " |"
      print line
      if (NR == 1) {
        line = "|"
        for (i = 1; i <= NF; i++) line = line " --- |"
        print line
      }
    }'
}
tab() {
  local IFS='	'
  printf '%s\n' "$*"
}

# Whole runs and pipelines: a row for each raw and path with figures.
summary="$out/summary.tsv"
if [[ $harness == 1 ]]; then
  tab Raw Path "Whole run" "Whole run, best" Decoding "Kernels compiled" Pipeline \
    "Pipeline, best" >"$summary"
else
  tab Raw Path "Whole run" Decoding "Kernels compiled" Pipeline "Pipeline, warm" \
    "Viewport, first render" "Exposure change" >"$summary"
fi
for raw in ${raws//,/ }; do
  for path in dt-cpu dt-opencl software gpu; do
    [[ -s "$values/$raw.$path.whole" ]] || continue
    pipeline=$(with_gpu "$raw" "$path" pipeline)
    tiles=$(median "$values/$raw.$path.tiles")
    [[ -z "$tiles" ]] || pipeline="$pipeline, $tiles tiles"
    if [[ $harness == 1 ]]; then
      tab "$(raw_label "$raw")" "$(path_label "$path")" "$(cell "$raw" "$path" whole fmt_s)" \
        "$(cell_best "$raw" "$path" whole fmt_s)" \
        "$(cell "$raw" "$path" decode fmt_ms)" "$(cell "$raw" "$path" compiled fmt_ms)" \
        "$pipeline" "$(cell_best "$raw" "$path" pipeline fmt_ms)" >>"$summary"
      continue
    fi
    tab "$(raw_label "$raw")" "$(path_label "$path")" "$(cell "$raw" "$path" whole fmt_s)" \
      "$(cell "$raw" "$path" decode fmt_ms)" "$(cell "$raw" "$path" compiled fmt_ms)" \
      "$pipeline" "$(with_gpu "$raw" "$path" warm)" "$(cell "$raw" "$path" first fmt_ms)" \
      "$(cell "$raw" "$path" change fmt_ms)" >>"$summary"
  done
done

# Each module of raw $1, a row each, a column for each path: darktable's modules, and
# Apertrust's operations' GPU time, the warm export's medians where it ran, else the
# cold export's.
modules_of() {
  local raw=$1 path paths_here=() header name names known row warm
  for path in dt-cpu dt-opencl software gpu; do
    if [[ -s "$values/$raw.$path.whole" ]]; then
      paths_here+=("$path")
    fi
  done
  [[ ${#paths_here[@]} -gt 0 ]] || return 0
  # The metrics each path's column reads, the warm export's where it ran.
  local metric=()
  header=(Module)
  for path in "${paths_here[@]}"; do
    warm=$(ls "$values" | grep "^$raw\.$path\.warmop\." || true)
    if [[ $path == dt-* ]]; then
      header+=("$(path_label "$path")")
      metric+=(op)
    elif [[ -n "$warm" ]]; then
      header+=("$(path_label "$path"), warm")
      metric+=(warmop)
    else
      header+=("$(path_label "$path"), cold")
      metric+=(op)
    fi
  done
  tab "${header[@]}"
  # darktable's order, then whatever else either side ran.
  known="rawprepare temperature highlights region demosaic exposure colorin channelmixerrgb agx colorout finalscale gamma lab"
  names=$(
    {
      for name in $known; do echo "$name"; done
      ls "$values" | sed -n -e "s/^$raw\.[a-z-]*\.warmop\.\(.*\)$/\1/p" \
        -e "s/^$raw\.[a-z-]*\.op\.\(.*\)$/\1/p" | sort -u
    } | awk '!seen[$0]++'
  )
  local i shown
  for name in $names; do
    if [[ $name == lab ]]; then
      row=("darktable's Lab conversions")
    else
      row=("$name")
    fi
    shown=0
    for ((i = 0; i < ${#paths_here[@]}; i++)); do
      row+=("$(cell "$raw" "${paths_here[i]}" "${metric[i]}.$name" fmt_ms)")
      if [[ -s "$values/$raw.${paths_here[i]}.${metric[i]}.$name" ]]; then
        shown=1
      fi
    done
    if [[ $shown == 1 ]]; then
      tab "${row[@]}"
    fi
  done
}

echo
aligned <"$summary"
for raw in ${raws//,/ }; do
  modules_of "$raw" >"$out/modules.$raw.tsv"
  if [[ -s "$out/modules.$raw.tsv" ]]; then
    echo
    echo "$(raw_label "$raw"), each module:"
    aligned <"$out/modules.$raw.tsv"
  fi
done

viewport=""
for raw in ${raws//,/ }; do
  for path in software gpu; do
    if [[ -z "$viewport" && -s "$values/$raw.$path.size" ]]; then
      viewport=$(cat "$values/$raw.$path.size")
    fi
  done
done

markdown="$out/results.md"
{
  echo "Measured on $(date +%Y-%m-%d) with \`tools/compare-cpu.sh\`, each figure the median of $runs run$([[ $runs == 1 ]] || echo s), each a new process."
  echo
  echo "- **Machine:** $machine."
  if has "$paths" dt-cpu || has "$paths" dt-opencl; then
    echo "- **darktable:** $dt_version${dt_device:+, OpenCL on $dt_device}."
  fi
  if [[ $quiet -gt 0 && -s "$out/idle.tsv" ]]; then
    busy=$(awk -F '\t' -v q="$quiet" '$1 < q { n++ } END { print n + 0 }' "$out/idle.tsv")
    echo "- **Other load:** each run waited for a CPU at least $quiet% idle and was repeated when the machine was busier after it; $busy of $(wc -l <"$out/idle.tsv" | tr -d ' ') runs counted still ended with it busier, the CPU $(awk -F '\t' '{ s += $1 } END { printf "%.0f", s / NR }' "$out/idle.tsv")% idle on average after them."
  fi
  if [[ $harness == 1 ]]; then
    echo "- **Apertrust:** $golden_version, \`apertrust-golden export\`, the engine's export as \`apertrust-cli export\` runs it."
    with_cases=""
    [[ -z "$adds" ]] || with_cases=" and the modules of the golden cases $adds"
    at_size=""
    [[ -z "$size" ]] || at_size=", within $size × $size pixels, high quality $hq"
    echo "- **The export:** the default modules$with_cases$at_size, each program's PNG at 8 bits; Apertrust's crop (the declared image), levels and parameters written into darktable's history by \`apertrust-golden export --darktable-xmp\`, as the exit test has them."
    echo "- **Load:** load averages (1, 5 and 15 minutes) when the runs began, $load_before; when they ended, $(load_averages)."
  else
    echo "- **Apertrust:** $apertrust_version."
  fi
  if [[ $want_software == 1 ]]; then
    echo "- **Software adapter:** $software_chosen, driver ${software_driver:-not named}; ${software_limits:-limits not logged}${lavapipe_icd:+; the Vulkan loader given only \`$lavapipe_icd\`}."
  fi
  if [[ $want_gpu == 1 ]]; then
    echo "- **GPU:** $gpu_chosen, driver ${gpu_driver:-not named}; ${gpu_limits:-limits not logged}."
  fi
  if [[ -n "$max_binding" ]]; then
    echo "- Apertrust bound at most $max_binding MiB at once (\`--max-binding\`)."
  fi
  echo
  markdown_table <"$summary"
  for raw in ${raws//,/ }; do
    if [[ -s "$out/modules.$raw.tsv" ]]; then
      echo
      echo "$(raw_label "$raw"), each module:"
      echo
      markdown_table <"$out/modules.$raw.tsv"
    fi
  done
  echo
  echo "- **Whole run:** \`darktable-cli\` or \`apertrust-cli export\` from start to exit, the PNG written; the only time measured from outside."
  echo "- **Decoding:** darktable's \`[dt_dev_load_raw] loading the image\`; Apertrust's \`decoded\`. **Kernels compiled:** Apertrust's \`compiled\` lines added up."
  if [[ $harness == 1 ]]; then
    echo "- **Pipeline:** darktable's \`pixel pipeline processing took\`; Apertrust's \`rendered\`, from submission to the GPU finishing, with its passes' GPU time, or where the export tiles its \`exported\` line's renders, added up over the raw stage and the tiles. **Best:** the smallest of the runs, which other load on the machine can only raise; the runs of the programs were taken in turn."
  else
    echo "- **Pipeline:** darktable's \`pixel pipeline processing took\`; Apertrust's \`rendered\`, from submission to the GPU finishing, with its passes' GPU time, or where the export tiles its \`exported\` line's renders, added up over the raw stage and the tiles. **Pipeline, warm:** Apertrust's median of $renders renders on one pipeline, the first included (\`export --runs\`), which renders the image whole, so only where the adapter binds it at once."
  fi
  echo "- **Each module:** darktable's \`[dev_pixelpipe] took … processed\` lines, and its colour-space conversions to and from Lab; Apertrust's GPU time, its passes added up: the warm export's medians (\`median agx\`), else the cold export's \`pass\` lines (\`-d verbose\`) over the raw stage and the tiles. Apertrust's \`region\` copies a tile's input."
  if [[ $harness == 0 ]]; then
    echo "- **Viewport:** \`apertrust-cli preview\`${viewport:+ at $viewport}, through Apertrust's default pipeline: its first render, and its median of $changes exposure changes on the GPU and $software_changes on the software adapter."
  fi
  for note in ${notes[@]+"${notes[@]}"}; do
    echo "- $note"
  done
} >"$markdown"

if [[ ${#notes[@]} -gt 0 ]]; then
  echo
  for note in "${notes[@]}"; do
    echo "$note"
  done
fi
echo
echo "Markdown: $markdown"
