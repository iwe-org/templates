#!/usr/bin/env bash
set -euo pipefail

repository="$1"
rev="$2"
destination="$3"

export GIT_LFS_SKIP_SMUDGE=1
export GIT_TERMINAL_PROMPT=0

git init --quiet "$destination"
git -C "$destination" remote add origin "$repository"
git -C "$destination" fetch --quiet --depth 1 --no-tags --no-recurse-submodules origin "$rev"
git -C "$destination" -c core.hooksPath=/dev/null -c advice.detachedHead=false checkout --quiet FETCH_HEAD

checked_out="$(git -C "$destination" rev-parse HEAD)"
if [[ "$checked_out" != "$rev" ]]; then
  echo "expected $rev, checked out $checked_out" >&2
  exit 1
fi
