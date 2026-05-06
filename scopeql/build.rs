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

use std::collections::BTreeSet;
use std::env;
use std::path::Path;
use std::str::FromStr;

use shadow_rs::CARGO_METADATA;
use shadow_rs::CARGO_TREE;
use shadow_rs::ShadowBuilder;

fn configure_rerun_if_head_commit_changed() {
    let mut current = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();

    // skip if no valid-looking git repository could be found
    while let Ok((dir, _)) = gix_discover::upwards(current.as_path()) {
        match dir {
            gix_discover::repository::Path::Repository(git_dir) => {
                unreachable!(
                    "build.rs should never be placed in a git bare repository: {}",
                    git_dir.display()
                );
            }
            gix_discover::repository::Path::WorkTree(work_dir) => {
                let git_refs_heads = work_dir.join(".git/refs/heads");
                println!("cargo::rerun-if-changed={}", git_refs_heads.display());
                break;
            }
            gix_discover::repository::Path::LinkedWorkTree { work_dir, .. } => {
                current = work_dir
                    .parent()
                    .expect("submodule's work_dir must have parent")
                    .to_path_buf();
                continue;
            }
        };
    }
}

fn main() -> shadow_rs::SdResult<()> {
    let now = jiff::Timestamp::now();

    configure_rerun_if_head_commit_changed();

    // The "CARGO_WORKSPACE_DIR" is set manually (not by Rust itself) in Cargo config file, to
    // solve the problem where the "CARGO_MANIFEST_DIR" is not what we want when this repo is
    // made as a submodule in another repo.
    let src_path = env::var("CARGO_WORKSPACE_DIR").or_else(|_| env::var("CARGO_MANIFEST_DIR"))?;
    let out_path = env::var("OUT_DIR")?;
    let shadow = ShadowBuilder::builder()
        .src_path(src_path)
        .out_path(out_path)
        // exclude these two large constants that we don't need
        .deny_const(BTreeSet::from([CARGO_METADATA, CARGO_TREE]))
        .build()?;

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
        println!("cargo:rustc-env=SOURCE_TIMESTAMP={source_timestamp}");
    } else if let Some(commit) = shadow.map.get(shadow_rs::COMMIT_DATE_3339) {
        let ts = commit.v.as_str();
        let source_timestamp = jiff::Timestamp::from_str(ts).unwrap_or_else(|err| {
            panic!("COMMIT_DATE_3339 {ts} could not be cast to a timestamp: {err}")
        });
        println!("cargo:rustc-env=SOURCE_TIMESTAMP={source_timestamp}");
    } else {
        println!("cargo:warning=SOURCE_TIMESTAMP is set to empty");
        println!("cargo:rustc-env=SOURCE_TIMESTAMP=");
    };

    let build_timestamp = now;
    println!("cargo:rustc-env=BUILD_TIMESTAMP={build_timestamp}");

    Ok(())
}
