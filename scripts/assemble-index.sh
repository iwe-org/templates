#!/usr/bin/env bash
set -euo pipefail

binary="${IWE_TEMPLATE:-target/release/iwe-template}"
work="$(mktemp -d)"
download_url="https://github.com/$GITHUB_REPOSITORY/releases/download"

mkdir -p "$work/entries"
echo '{}' > "$work/stars.json"
"$binary" entries templates.toml > "$work/entries.json"

if ! curl --fail --silent --show-error --location "$download_url/index/index.json" --output "$work/published.json"; then
  echo '{"templates":[]}' > "$work/published.json"
fi

while IFS=$'\t' read -r id version repository; do
  release="$id-$version"
  gh release download "$release" --repo "$GITHUB_REPOSITORY" --pattern entry.json --dir "$work/entries/$release"
  stars="$(gh api "repos/${repository#https://github.com/}" --jq .stargazers_count 2> /dev/null || true)"
  if [[ ! "$stars" =~ ^[0-9]+$ ]]; then
    stars="$(jq --arg id "$id" '[.templates[] | select(.id == $id) | .stars][0] // 0' "$work/published.json")"
    echo "::warning::no star count for $repository, keeping $stars"
  fi
  jq --arg id "$id" --argjson stars "$stars" '. + {($id): $stars}' "$work/stars.json" > "$work/stars.next.json"
  mv "$work/stars.next.json" "$work/stars.json"
done < <(jq -r '.[] | [.id, .version, .repository] | @tsv' "$work/entries.json")

"$binary" index \
  --registry templates.toml \
  --entries "$work/entries" \
  --stars "$work/stars.json" \
  --base-url "$download_url" \
  --out "$work/index.json"

if ! gh release view index --repo "$GITHUB_REPOSITORY" > /dev/null 2>&1; then
  gh release create index \
    --repo "$GITHUB_REPOSITORY" \
    --target "$GITHUB_SHA" \
    --title "Template index" \
    --notes "The rolling release that holds index.json, the list the iWe template gallery reads. It is regenerated on every merge and once a day." \
    --latest=false
fi

gh release upload index "$work/index.json" --repo "$GITHUB_REPOSITORY" --clobber
cat "$work/index.json"
