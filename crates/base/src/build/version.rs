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

//! Apertrust's version, from git as Gumka has it (decision 44): the newest
//! `release-X.Y.Z` tag at or before the commit built. `Cargo.toml`'s version is not the
//! app's. The `build` module reads it with this in a binary's build script.

use std::path::Path;
use std::process::Command;

/// The version a build reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    /// `X.Y.Z` on a commit tagged `release-X.Y.Z`, `X.Y.Z+N~gHASH` N commits after it,
    /// `0.0.0+N~gHASH` before any, N then every commit; `-dirty` after it when tracked
    /// files have changes. Or a packager's, as given.
    pub full: String,
    /// How many commits lead to this one: a number that only grows along main, for what
    /// takes numbers only. `None` when a packager gives the version.
    pub build: Option<u32>,
}

impl Version {
    /// Four numbers for what takes numbers only, as Windows' version resource does: the
    /// release's three, those `full` starts with (0 where it starts with none, as a
    /// packager's may), and the count of commits, 0 when unknown; each at most 65535.
    pub fn numbers(&self) -> [u16; 4] {
        let release = self.full.split(['+', '-', '~']).next().unwrap_or_default();
        let mut numbers = [0; 4];
        let mut parts = release.split('.').map(|part| part.parse::<u32>());
        if let (Some(Ok(x)), Some(Ok(y)), Some(Ok(z)), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        {
            numbers[..3].copy_from_slice(&[x, y, z].map(clamp));
        }
        numbers[3] = clamp(self.build.unwrap_or(0));
        numbers
    }
}

fn clamp(number: u32) -> u16 {
    u16::try_from(number).unwrap_or(u16::MAX)
}

/// The version of the checkout at `root`, the repository's top folder, unless `given`,
/// a packager's `APERTRUST_VERSION`, is set and not empty: that is used as it is. A
/// folder without git, as a source archive, a shallow clone, which has neither the tags
/// nor the history, a git that fails and a release tag that is not `release-X.Y.Z` are
/// errors saying why: no version is guessed.
pub fn read(root: &Path, given: Option<&str>) -> Result<Version, String> {
    if let Some(given) = given.filter(|given| !given.is_empty()) {
        return Ok(Version {
            full: given.to_owned(),
            build: None,
        });
    }
    // Its own `.git` only, a folder in a worktree: a source archive unpacked inside
    // another checkout is not that checkout's commit.
    if !root.join(".git").exists() {
        return Err("no git checkout, as in a source archive; a packager sets \
                    APERTRUST_VERSION"
            .into());
    }
    from_git(|args| git(root, args))
}

/// The version from what `git` prints when run with the given arguments.
fn from_git(git: impl Fn(&[&str]) -> Result<String, String>) -> Result<Version, String> {
    if git(&["rev-parse", "--is-shallow-repository"])? == "true" {
        return Err(
            "this clone is shallow, without the tags and history the version \
                    comes from; fetch them with `git fetch --unshallow --tags`"
                .into(),
        );
    }
    let build = git(&["rev-list", "--count", "HEAD"])?
        .parse()
        .map_err(|e| format!("git's count of commits: {e}"))?;
    // Tags such as `release-notes` are not versions.
    let described = git(&[
        "describe",
        "--tags",
        "--long",
        "--abbrev=7",
        "--match",
        "release-[0-9]*",
    ]);
    let release = match described {
        Ok(described) => from_describe(&described)?,
        // No release yet, as if one were tagged before the first commit. Only then: the
        // release tags tell why describe failed, not git's message, which is translated.
        Err(failed) => {
            if !git(&["tag", "--list", "--merged", "HEAD", "release-[0-9]*"])?.is_empty() {
                return Err(failed);
            }
            format!(
                "0.0.0+{build}~g{}",
                git(&["rev-parse", "--short=7", "HEAD"])?
            )
        }
    };
    // Not `describe --dirty`, which refreshes the index and writes it back, taking its
    // lock while the person may be committing: `status` without the optional locks
    // writes nothing. Untracked files do not count, as for `describe --dirty`.
    let changes = git(&["status", "--porcelain", "--untracked-files=no"])?;
    let dirty = if changes.is_empty() { "" } else { "-dirty" };
    Ok(Version {
        full: format!("{release}{dirty}"),
        build: Some(build),
    })
}

/// The version from `git describe --tags --long`'s `release-X.Y.Z-N-gHASH`.
fn from_describe(described: &str) -> Result<String, String> {
    let mut fields = described.rsplitn(3, '-');
    let (Some(hash), Some(commits), Some(tag)) = (fields.next(), fields.next(), fields.next())
    else {
        return Err(format!("cannot read the version from {described:?}"));
    };
    let release = tag.strip_prefix("release-").unwrap_or(tag);
    let numbers: Vec<&str> = release.split('.').collect();
    let number = |n: &&str| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit());
    if numbers.len() != 3 || !numbers.iter().all(number) {
        return Err(format!(
            "the tag {tag} is not release-X.Y.Z; delete it, or tag the commit as one"
        ));
    }
    Ok(match commits {
        "0" => release.to_owned(),
        _ => format!("{release}+{commits}~{hash}"),
    })
}

/// A git command's trimmed output in the checkout at `root`, or why it failed. Git takes
/// none of its optional locks, so it never writes the index, and ignores the variables
/// a git hook running cargo would set, which point at another checkout.
pub fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env_remove("GIT_DIR")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_WORK_TREE")
        .output()
        .map_err(|e| format!("cannot run git: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    } else {
        Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime};

    use super::*;

    #[test]
    fn the_version_comes_from_the_newest_release_tag() {
        let version = |described| from_describe(described).unwrap();
        assert_eq!(version("release-0.2.0-0-gabc1234"), "0.2.0", "on the tag");
        assert_eq!(
            version("release-0.2.0-12-gabc1234"),
            "0.2.0+12~gabc1234",
            "12 commits later"
        );
        assert_eq!(version("release-10.20.30-3-g0f"), "10.20.30+3~g0f");
    }

    #[test]
    fn the_numbers_are_the_release_and_the_count_of_commits() {
        let numbers = |full: &str, build| {
            Version {
                full: full.to_owned(),
                build,
            }
            .numbers()
        };
        assert_eq!(numbers("0.2.0", Some(615)), [0, 2, 0, 615]);
        assert_eq!(
            numbers("0.2.0+12~gabc1234-dirty", Some(615)),
            [0, 2, 0, 615]
        );
        assert_eq!(numbers("0.0.0+615~gabc1234", Some(615)), [0, 0, 0, 615]);
        assert_eq!(numbers("1.4.2-beta.1", None), [1, 4, 2, 0], "a packager's");
        assert_eq!(numbers("2026.11", None), [0, 0, 0, 0], "not three numbers");
        assert_eq!(numbers("1.70000.0", Some(70_000)), [1, 65535, 0, 65535]);
    }

    #[test]
    fn a_malformed_release_tag_is_named_never_guessed() {
        for (described, tag) in [
            ("release-0.2-0-gabc1234", "release-0.2"),
            ("release-0.2.0.1-0-gabc1234", "release-0.2.0.1"),
            ("release-0.2.0-beta.1-0-gabc1234", "release-0.2.0-beta.1"),
            ("release-v0.2.0-0-gabc1234", "release-v0.2.0"),
            ("release-0..2-0-gabc1234", "release-0..2"),
        ] {
            let refused = from_describe(described).unwrap_err();
            assert!(refused.contains(&format!("the tag {tag} ")), "{refused}");
        }
        assert!(from_describe("abc1234").is_err());
    }

    #[test]
    fn only_a_missing_release_tag_is_0_0_0() {
        // A git whose describe fails, with these release tags at or before the commit.
        let git = |tags: &'static str| {
            move |args: &[&str]| match args {
                ["rev-parse", "--is-shallow-repository"] => Ok("false".to_owned()),
                ["rev-list", "--count", "HEAD"] => Ok("5".to_owned()),
                ["describe", ..] => Err("git describe failed".to_owned()),
                ["tag", "--list", "--merged", "HEAD", _] => Ok(tags.to_owned()),
                ["rev-parse", "--short=7", "HEAD"] => Ok("abc1234".to_owned()),
                ["status", ..] => Ok(String::new()),
                _ => panic!("git {args:?}"),
            }
        };
        let version = from_git(git("")).unwrap();
        assert_eq!(version.full, "0.0.0+5~gabc1234");
        assert_eq!(version.build, Some(5));
        assert_eq!(
            from_git(git("release-0.1.0")).err().as_deref(),
            Some("git describe failed"),
            "no version is guessed"
        );
    }

    /// Runs git in `dir` with a fixed identity, no signing and repositories in the files
    /// format, whatever the person's settings, and none of a git hook's variables.
    fn git_in(dir: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .current_dir(dir)
            .env_remove("GIT_DEFAULT_REF_FORMAT")
            .env_remove("GIT_DIR")
            .env_remove("GIT_INDEX_FILE")
            .env_remove("GIT_WORK_TREE")
            .args(["-c", "user.name=Apertrust"])
            .args(["-c", "user.email=apertrust@example.invalid"])
            .args(["-c", "commit.gpgSign=false", "-c", "tag.gpgSign=false"])
            .args(["-c", "init.defaultRefFormat=files"])
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    }

    /// A new, empty folder for the test `name`.
    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("apertrust-version-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A repository in `dir` with one commit of one tracked file, `tracked`.
    fn repository(dir: &Path) {
        fs::create_dir_all(dir).unwrap();
        git_in(dir, &["init", "-q"]);
        fs::write(dir.join("tracked"), "one\n").unwrap();
        git_in(dir, &["add", "tracked"]);
        git_in(dir, &["commit", "-q", "-m", "one"]);
    }

    fn commit(dir: &Path, message: &str) -> String {
        git_in(dir, &["commit", "-q", "--allow-empty", "-m", message]);
        git_in(dir, &["rev-parse", "--short=7", "HEAD"])
    }

    fn version(dir: &Path) -> (String, Option<u32>) {
        let version = read(dir, None).unwrap();
        (version.full, version.build)
    }

    #[test]
    fn the_version_follows_the_repository() {
        let dir = scratch("follows");
        let refused = read(&dir, None).unwrap_err();
        assert!(refused.contains("no git checkout"), "{refused}");

        repository(&dir);
        git_in(&dir, &["tag", "release-notes"]);
        git_in(&dir, &["tag", "v9.9.9"]);
        let hash = git_in(&dir, &["rev-parse", "--short=7", "HEAD"]);
        assert_eq!(
            version(&dir),
            (format!("0.0.0+1~g{hash}"), Some(1)),
            "no release yet: tags that are no release are ignored"
        );
        git_in(&dir, &["checkout", "-q", "-b", "other"]);
        commit(&dir, "elsewhere");
        git_in(&dir, &["tag", "release-9.0.0"]);
        git_in(&dir, &["checkout", "-q", "-"]);
        assert_eq!(
            version(&dir),
            (format!("0.0.0+1~g{hash}"), Some(1)),
            "a release on another branch is not this commit's"
        );

        git_in(&dir, &["tag", "release-0.1.0"]);
        assert_eq!(version(&dir), ("0.1.0".into(), Some(1)), "on the tag");
        commit(&dir, "two");
        let hash = commit(&dir, "three");
        assert_eq!(
            version(&dir),
            (format!("0.1.0+2~g{hash}"), Some(3)),
            "two commits after it"
        );

        git_in(&dir, &["tag", "release-0.2"]);
        let refused = read(&dir, None).unwrap_err();
        assert!(refused.contains("the tag release-0.2 "), "{refused}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn tracked_changes_mark_the_version_dirty_without_writing_the_index() {
        let dir = scratch("dirty");
        repository(&dir);
        git_in(&dir, &["tag", "release-0.1.0"]);
        let index = dir.join(".git/index");
        let before = fs::read(&index).unwrap();
        // The same text, written later: a plain `git status` would refresh the index's
        // record of it and write the index back.
        let tracked = fs::File::options()
            .write(true)
            .open(dir.join("tracked"))
            .unwrap();
        tracked
            .set_modified(SystemTime::now() + Duration::from_secs(60))
            .unwrap();
        fs::write(dir.join("untracked"), "new\n").unwrap();
        assert_eq!(version(&dir).0, "0.1.0", "an untracked file does not count");
        assert_eq!(
            fs::read(&index).unwrap(),
            before,
            "the index is not written"
        );

        fs::write(dir.join("tracked"), "two\n").unwrap();
        assert_eq!(version(&dir).0, "0.1.0-dirty");
        let hash = commit(&dir, "two");
        assert_eq!(version(&dir).0, format!("0.1.0+1~g{hash}-dirty"));
        git_in(&dir, &["add", "tracked"]);
        assert_eq!(version(&dir).0, format!("0.1.0+1~g{hash}-dirty"), "staged");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_packager_s_version_is_used_as_given() {
        let dir = scratch("given");
        let given = read(&dir, Some("1.2.3-1.fc43")).unwrap();
        assert_eq!(given.full, "1.2.3-1.fc43", "without git");
        assert_eq!(given.build, None);
        assert!(read(&dir, Some("")).is_err(), "empty counts as unset");

        repository(&dir);
        assert_eq!(read(&dir, Some("2.0.0")).unwrap().full, "2.0.0");
        assert_eq!(read(&dir, Some("")).unwrap(), read(&dir, None).unwrap());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_worktree_reads_the_repository_s_tags() {
        let dir = scratch("worktree");
        let repo = dir.join("repo");
        repository(&repo);
        git_in(&repo, &["tag", "release-0.1.0"]);
        git_in(&repo, &["worktree", "add", "-q", "-b", "side", "../side"]);
        let side = dir.join("side");
        assert!(side.join(".git").is_file());
        assert_eq!(version(&side), ("0.1.0".into(), Some(1)));
        let hash = commit(&side, "two");
        assert_eq!(version(&side), (format!("0.1.0+1~g{hash}"), Some(2)));
        assert_eq!(
            version(&repo),
            ("0.1.0".into(), Some(1)),
            "the main checkout's"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_shallow_clone_is_refused() {
        let dir = scratch("shallow");
        let repo = dir.join("repo");
        repository(&repo);
        git_in(&repo, &["tag", "release-0.1.0"]);
        commit(&repo, "two");
        // `--no-local`, as a local clone copies everything and ignores `--depth`; a
        // relative path rather than a file URL, which Windows spells differently.
        git_in(
            &dir,
            &["clone", "-q", "--no-local", "--depth", "1", "repo", "clone"],
        );
        let clone = dir.join("clone");
        let refused = read(&clone, None).unwrap_err();
        assert!(refused.contains("shallow"), "{refused}");

        git_in(&clone, &["fetch", "-q", "--unshallow", "--tags"]);
        assert_eq!(version(&clone), version(&repo));
        let _ = fs::remove_dir_all(&dir);
    }
}
