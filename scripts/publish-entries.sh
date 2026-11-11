#!/usr/bin/env bash
set -uo pipefail

binary="${IWE_TEMPLATE:-target/release/iwe-template}"
scripts="$(cd "$(dirname "$0")" && pwd)"
work="$(mktemp -d)"
failed=0

publish_entry() {
  local id="$1" version="$2" repository="$3" rev="$4" path="$5"
  local release="$id-$version"
  local checkout="$work/$release/checkout"
  local assets="$work/$release/assets"

  "$scripts/fetch-template.sh" "$repository" "$rev" "$checkout" || return 1
  "$binary" check "$checkout/$path" --id "$id" --version "$version" || return 1
  "$binary" pack "$checkout/$path" --out "$assets" --repository "$repository" --rev "$rev" || return 1
  gh release create "$release" "$assets"/* \
    --repo "$GITHUB_REPOSITORY" \
    --target "$GITHUB_SHA" \
    --title "$id $version" \
    --notes "Template \`$id\` $version, packaged from $repository at \`$rev\`." \
    --latest=false
}

"$binary" entries templates.toml > "$work/entries.json" || exit 1

while IFS=$'\t' read -r id version repository rev path; do
  release="$id-$version"
  if gh release view "$release" --repo "$GITHUB_REPOSITORY" > /dev/null 2>&1; then
    echo "$release is already published"
    continue
  fi
  echo "::group::publish $release"
  if ! publish_entry "$id" "$version" "$repository" "$rev" "$path"; then
    echo "::error::publishing $release failed"
    failed=1
  fi
  echo "::endgroup::"
done < <(jq -r '.[] | [.id, .version, .repository, .rev, (if .path == "" then "." else .path end)] | @tsv' "$work/entries.json")

exit "$failed"
