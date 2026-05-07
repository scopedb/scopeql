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

use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildInfo {
    pub branch: &'static str,
    pub commit: &'static str,
    pub commit_short: &'static str,
    pub clean: bool,
    pub source_time: &'static str,
    pub build_time: &'static str,
    pub rustc: &'static str,
    pub target: &'static str,
    pub version: &'static str,
}

pub const fn build_info() -> BuildInfo {
    let dirty = env!("SCOPEQL_GIT_DIRTY").eq_ignore_ascii_case("true");
    let clean = !dirty;

    let commit = env!("SCOPEQL_GIT_COMMIT_HASH");
    let (commit_short, _) = commit.split_at(8);

    BuildInfo {
        branch: env!("SCOPEQL_GIT_BRANCH"),
        commit,
        commit_short,
        clean,
        source_time: env!("SCOPEQL_SOURCE_TIMESTAMP"),
        build_time: env!("SCOPEQL_BUILD_TIMESTAMP"),
        rustc: env!("SCOPEQL_RUSTC_VERSION"),
        target: env!("SCOPEQL_BUILD_TARGET"),
        version: env!("CARGO_PKG_VERSION"),
    }
}

pub const fn version() -> &'static str {
    const BUILD_INFO: BuildInfo = build_info();

    const_format::formatcp!(
        "\nversion: {}\nbranch: {}\ncommit: {}\nclean: {}\nsource_time: {}\nbuild_time: {}\nrustc: {}\ntarget: {}",
        BUILD_INFO.version,
        BUILD_INFO.branch,
        BUILD_INFO.commit,
        BUILD_INFO.clean,
        BUILD_INFO.source_time,
        BUILD_INFO.build_time,
        BUILD_INFO.rustc,
        BUILD_INFO.target,
    )
}
