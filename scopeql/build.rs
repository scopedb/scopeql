// Copyright 2025 ScopeDB, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::env;
use std::fmt::Display;
use std::path::Path;
use std::process::Command;

fn configure_rustc_env(key: &'static str, value: impl Display) {
    println!("cargo:rustc-env={key}={value}");
}

fn configure_rustc_env_within_repo(repo: gix::Repository) -> Result<(), gix::Error> {
    let head = repo.head().map_err(gix::Error::from_error)?;
    configure_rustc_env(
        "SCOPEQL_GIT_BRANCH",
        match &head.kind {
            gix::head::Kind::Symbolic(r) => r.name.shorten(),
            gix::head::Kind::Unborn(n) => n.shorten(),
            gix::head::Kind::Detached { .. } => gix::bstr::BStr::new("(detached)"),
        },
    );

    let head_commit = repo.head_commit().map_err(gix::Error::from_error)?;
    configure_rustc_env("SCOPEQL_GIT_COMMIT_HASH", head_commit.id);

    fn is_dirty(repo: &gix::Repository) -> Result<bool, gix::Error> {
        let status_platform = repo
            .status(gix::progress::Discard)
            .map_err(gix::Error::from_error)?;
        let status_iter = status_platform
            .untracked_files(gix::status::UntrackedFiles::Collapsed)
            .into_iter(None)
            .map_err(gix::Error::from_error)?;
        for item in status_iter {
            let item = item.map_err(gix::Error::from_error)?;
            let dirty = match item {
                gix::status::Item::IndexWorktree(item) => match item {
                    gix::status::index_worktree::Item::Modification { .. } => true,
                    gix::status::index_worktree::Item::DirectoryContents { entry, .. } => {
                        matches!(entry.status, gix::dir::entry::Status::Untracked)
                    }
                    gix::status::index_worktree::Item::Rewrite { .. } => true,
                },
                gix::status::Item::TreeIndex { .. } => true,
            };
            if dirty {
                return Ok(true);
            }
        }

        Ok(false)
    }
    configure_rustc_env("SCOPEQL_GIT_DIRTY", is_dirty(&repo)?);

    let author = head_commit.author().map_err(gix::Error::from_error)?;
    let datetime = author.time().map_err(gix::Error::from_error)?;
    let epoch = datetime.seconds;
    let source_timestamp = jiff::Timestamp::from_second(epoch).map_err(gix::Error::from_error)?;
    configure_rustc_env("SCOPEQL_SOURCE_TIMESTAMP", source_timestamp);

    Ok(())
}

fn main() {
    // Ensure all required build environment variables are present with default values
    configure_rustc_env("SCOPEQL_SOURCE_TIMESTAMP", "");
    configure_rustc_env("SCOPEQL_BUILD_TIMESTAMP", "");
    configure_rustc_env("SCOPEQL_RUSTC_VERSION", "");
    configure_rustc_env("SCOPEQL_BUILD_TARGET", "");
    configure_rustc_env("SCOPEQL_GIT_BRANCH", "");
    configure_rustc_env("SCOPEQL_GIT_COMMIT_HASH", "");
    configure_rustc_env("SCOPEQL_GIT_DIRTY", "false");

    let now = jiff::Timestamp::now();
    // Truncate to seconds to align with SCOPEQL_SOURCE_TIMESTAMP
    configure_rustc_env(
        "SCOPEQL_BUILD_TIMESTAMP",
        now.strftime("%Y-%m-%dT%H:%M:%SZ"),
    );

    if let Ok(version) = Command::new("rustc").arg("-V").output() {
        let version = String::from_utf8_lossy(&version.stdout);
        configure_rustc_env("SCOPEQL_RUSTC_VERSION", version.trim());
    }

    if let Ok(target) = env::var("TARGET") {
        configure_rustc_env("SCOPEQL_BUILD_TARGET", target);
    }

    // Override Git-related build environment variables if within a Git repository
    if let Ok(repo) = gix::discover(Path::new(env!("CARGO_MANIFEST_DIR"))) {
        let git_refs_heads = repo.path().join("refs/heads");
        println!("cargo:rerun-if-changed={}", git_refs_heads.display());

        if let Err(err) = configure_rustc_env_within_repo(repo) {
            println!("cargo:warning=failed to configure environment within git repo: {err}");
        }
    }

    // Override SCOPEQL_SOURCE_TIMESTAMP with SOURCE_DATE_EPOCH if set
    // @see https://reproducible-builds.org/docs/source-date-epoch/
    println!("cargo:rerun-if-env-changed=SOURCE_DATE_EPOCH");
    if let Some(ts) = env::var_os("SOURCE_DATE_EPOCH") {
        let epoch = ts
            .into_string()
            .expect("SOURCE_DATE_EPOCH contains invalid Unicode")
            .parse::<i64>()
            .expect("SOURCE_DATE_EPOCH is not a valid integer");
        let source_timestamp = jiff::Timestamp::from_second(epoch).unwrap_or_else(|err| {
            panic!("SOURCE_DATE_EPOCH could not be cast to a timestamp: {err}")
        });
        configure_rustc_env("SCOPEQL_SOURCE_TIMESTAMP", source_timestamp);
    }
}
