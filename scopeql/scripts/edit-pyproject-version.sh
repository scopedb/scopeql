#!/usr/bin/env bash
# Copyright 2025 ScopeDB, Inc.
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

set -o nounset

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
PACKAGE_DIR=$( dirname "$SCRIPT_DIR" )
WORKSPACE_DIR=$( dirname "$PACKAGE_DIR" )

cd "$WORKSPACE_DIR"

update_pyproject_version() {
  local file="$1"
  VERSION="$VERSION" NEW_VERSION="$NEW_VERSION" perl -0pi -e 's/^version = "\Q$ENV{VERSION}\E"$/version = "$ENV{NEW_VERSION}"/m' "$file"
}

if [[ -z "${NEW_VERSION:-}" ]]; then
  echo "NEW_VERSION must not be empty"
  exit 1
fi

VERSION=$( cat pyproject.toml | yq -p toml -o toml -r '.project.version' )
echo "PREVIOUS PYTHON VERSION: $VERSION"
if [[ -z "$VERSION" || "$VERSION" == "null" ]]; then
  echo "Version from pyproject.toml must not be empty"
  exit 1
fi

if [[ "$VERSION" != "$NEW_VERSION" ]]; then
  if [[ "${DRY_RUN:-false}" == "true" ]]; then
    echo "Would update version in pyproject.toml to $NEW_VERSION"
    TEMP_PYPROJECT=$( mktemp )
    trap 'rm -f "$TEMP_PYPROJECT"' EXIT
    cp pyproject.toml "$TEMP_PYPROJECT"
    update_pyproject_version "$TEMP_PYPROJECT"

    VERSION=$( yq -p toml -o toml -r '.project.version' "$TEMP_PYPROJECT" )
    if [[ "$VERSION" != "$NEW_VERSION" ]]; then
      echo "failed to update version in pyproject.toml"
      exit 1
    fi

    diff -u --label pyproject.toml --label "pyproject.toml (dry run)" pyproject.toml "$TEMP_PYPROJECT" || true
    exit 0
  fi

  echo "Updating version in pyproject.toml to $NEW_VERSION"
  update_pyproject_version pyproject.toml

  VERSION=$( cat pyproject.toml | yq -p toml -o toml -r '.project.version' )
  if [[ "$VERSION" != "$NEW_VERSION" ]]; then
    echo "failed to update version in pyproject.toml"
    exit 1
  fi
else
  echo "Version in pyproject.toml already matches the new version, skipping update"
fi
