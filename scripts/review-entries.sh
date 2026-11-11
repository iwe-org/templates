#!/usr/bin/env bash
set -uo pipefail

changed_entries="$1"
binary="${IWE_TEMPLATE:-target/release/iwe-template}"
scripts="$(cd "$(dirname "$0")" && pwd)"
work="$(mktemp -d)"
summary="${GITHUB_STEP_SUMMARY:-/dev/stdout}"
summary_byte_limit=250000
failed=0

record_failure() {
  local title="$1"
  local output="$2"
  failed=1
  echo "::error::$title"
  {
    echo
    echo "### Failed: $title"
    echo
    echo '````text'
    echo "$output" | head -c 20000
    echo '````'
  } >> "$summary"
}

run_step() {
  local title="$1"
  shift
  local output
  if output="$("$@" 2>&1)"; then
    echo "ok: $title"
    return 0
  fi
  record_failure "$title" "$output"
  return 1
}

validate_content() {
  local template="$1"
  (cd "$template" && iwe schema validate) || return 1
  (cd "$template" && iwe normalize && git diff --exit-code --stat)
}

while IFS=$'\t' read -r id version repository rev path previous_repository previous_rev previous_version previous_path; do
  current="$work/$id/current"
  previous="$work/$id/previous"
  echo "::group::$id $version"

  if ! run_step "$id: fetch $repository at $rev" "$scripts/fetch-template.sh" "$repository" "$rev" "$current"; then
    echo "::endgroup::"
    continue
  fi

  check_arguments=(check "$current/$path" --id "$id" --version "$version")
  report_arguments=(report "$current/$path")
  if [[ "$previous_rev" != "-" && "$previous_rev" != "$rev" ]]; then
    check_arguments+=(--previous-version "$previous_version")
    if run_step "$id: fetch the previous revision $previous_rev" "$scripts/fetch-template.sh" "$previous_repository" "$previous_rev" "$previous"; then
      report_arguments+=(--since "$previous/$previous_path")
      if [[ "$previous_repository" == "$repository" ]]; then
        report_arguments+=(--repository "$repository" --rev "$rev" --since-rev "$previous_rev")
      fi
    fi
  fi

  "$binary" "${report_arguments[@]}" | head -c "$summary_byte_limit" >> "$summary"
  echo >> "$summary"

  if run_step "$id: structural validation" "$binary" "${check_arguments[@]}"; then
    run_step "$id: content validation (iwe schema validate, iwe normalize)" validate_content "$current/$path"
  fi
  echo "::endgroup::"
done < <(jq -r '.[] | [
  .id,
  .version,
  .repository,
  .rev,
  (if .path == "" then "." else .path end),
  (.previous.repository // "-"),
  (.previous.rev // "-"),
  (.previous.version // "-"),
  (if (.previous.path // "") == "" then "." else .previous.path end)
] | @tsv' "$changed_entries")

exit "$failed"
