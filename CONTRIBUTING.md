# Contributing a template

A template is a working iWe workspace with a manifest beside it. You keep it in
your own GitHub repository; this registry pins it to a commit a maintainer has
read. Submitting and updating are both a pull request to `templates.toml`.

## The template format

```
template.toml          the manifest; not installed
preview.png            optional, shown in the gallery; not installed
.iwe/config.toml       the workspace config
.iwe/schemas/*.yaml    the schemas
AGENTS.md  CLAUDE.md   optional agent instructions, at the root only
.claude/skills/**      optional skills, markdown only
.gitignore             optional
…                      documents, pages and their assets
```

Everything is installed into the new workspace except `template.toml`,
`preview.png`, `.github/`, a `LICENSE` file (`LICENSE`, `LICENSE.md`,
`LICENSE.txt`) and whatever the manifest's `exclude` names.

### `template.toml`

``` toml
format = 1
id = "dev-workspace"
name = "Dev Workspace"
description = "Memory for your coding agent: specs, plans, bugs and releases as a graph."
version = "1.0.0"
authors = ["iWe"]
license = "MIT"
exclude = ["README.md", "CHANGELOG.md"]
```

| Field | Rule |
| --- | --- |
| `format` | `1` |
| `id` | Lowercase kebab-case, 3 to 40 characters, starting with a letter. Unique in the registry; it never changes once published |
| `name` | At most 40 characters |
| `description` | One plain-text sentence on one line, at most 140 characters |
| `version` | Semver, without build metadata. It must increase every time the registry moves to a new revision |
| `authors` | At least one name |
| `license` | One of `MIT`, `Apache-2.0`, `CC0-1.0`, `CC-BY-4.0` |
| `exclude` | Optional glob patterns, relative to the template, of files that are checked but not installed. `*` does not cross a `/`, `**` does, and a pattern that names a folder excludes everything in it. A pattern may not exclude `.iwe/config.toml` |

No other field is accepted.

## The rules

`iwe-template check` enforces these. The app runs the same check before it
writes a file, so a template that breaks one is neither published nor installed.

- **Files outside dot folders.** `.md`, `.html`, `.css`, `.js`, `.json`,
  `.jsonl`, `.csv`, `.tsv`, `.png`, `.jpg`, `.gif`, `.webp`, `.svg` in lowercase,
  plus `template.toml` and a license file at the root. Anything else fails.
- **Dot paths.** Exactly these: `.iwe/config.toml`, `.iwe/schemas/*.yaml`,
  `.claude/skills/**/*.md`, and `.gitignore` at the root. `.github/` is ignored
  and never packaged. Every other path with a component that starts with a dot
  fails: `.claude/settings.json`, hooks, `.mcp.json`, `.iwe/app/`, `.env`.
- **Agent instructions.** `AGENTS.md` and `CLAUDE.md` at the template root only,
  spelled exactly so. A file with either name in any letter case anywhere else
  fails.
- **Links and paths.** No symlink, no hard link, no path that leaves the
  template, and no two paths that differ only in letter case.
- **Size.** At most 1,500 files, 1 MB per file and 5 MB in total. `preview.png` is
  at most 500 KB.
- **Scripts.** No minified or bundled JavaScript. A line over 500 characters
  fails in a `.js` file, in a `<script>` element of an `.html` or `.svg` file,
  and in a `<script>` element inside an html fenced block of a `.md` file.
- **Config.** `.iwe/config.toml` is required and must parse. Its top-level keys
  are limited to `version`, `format`, `markdown`, `djot`, `workspace`,
  `completion`, `search`, `templates`, `actions` and `schemas`. `version` is 4 or
  later. A `[library]` table fails; use `[workspace]`. `[commands]` fails. An
  action of type `transform`, or any action with a `command` key, fails.
  `workspace.path` is empty or a relative path, without `..`, to a folder the
  template contains.
- **Manifest.** The field rules above, and an `id` and `version` equal to the
  registry entry's.

CI also runs two quality checks with a pinned `iwe` CLI (the version is
`IWE_VERSION` in `.github/workflows/check.yml`), from the template's folder:

```
iwe schema validate
iwe normalize && git diff --exit-code
```

## Check it locally

```
cargo install --git https://github.com/iwe-org/templates iwe-template
iwe-template check path/to/your/template
iwe-template report path/to/your/template
```

`check` prints one line per broken rule and exits 1 when there is any. `report`
prints what a reviewer will be shown. Opening the repository as a workspace in
iWe is the other half of the test.

## Submit

1. Push your template to a public GitHub repository and note the full commit
   SHA you want reviewed.
2. Open a pull request here that adds a table to `templates.toml`:

   ``` toml
   [your-template-id]
   repository = "https://github.com/you/your-template"
   rev = "<the full 40-character commit SHA>"
   version = "<the version in template.toml at that commit>"
   path = ""
   ```

   - The table name is the template's `id`.
   - `rev` is a full commit SHA. A branch or a tag name fails the check.
   - `path` is the template's folder inside the repository, empty for the root.
   - Leave `featured` out. Maintainers set it.
3. The `check` workflow clones your repository at `rev`, runs both validations
   and writes the reviewer's report to the check's summary page.

## Update

Raise `version` in your `template.toml`, push, and open a pull request that
changes `rev` and `version` in your table. A new `rev` needs a higher `version`:
a published archive is never replaced.

A template repository can open that pull request from a tag with the reusable
workflow:

``` yaml
name: release
on:
  push:
    tags: ["v*"]
jobs:
  registry:
    uses: iwe-org/templates/.github/workflows/bump.yml@main
    with:
      id: your-template-id
    secrets:
      registry-token: ${{ secrets.REGISTRY_TOKEN }}
```

- The version is the tag name without its leading `v`; pass `version` to
  override it. It must equal the `version` in `template.toml` at the tagged
  commit.
- The branch is pushed to your fork of this repository, `<your account>/templates`
  by default; pass `fork: owner/name` when the fork is named differently.
- `REGISTRY_TOKEN` is a secret you create in your template repository: a
  personal access token (classic) with the `public_repo` scope, which is what it
  takes to push to your fork and open a pull request against this repository.
  The workflow uses it for nothing else.
- The workflow only updates an entry that exists. The first submission is
  written by hand.

## Review

Every change to `templates.toml` needs a maintainer's approval. Nothing merges
on CI alone, a first submission or a one-line fix alike.

The reviewer reads the whole template on a submission and the whole diff on an
update, following the compare link in the report. The report lists what the
validator cannot judge:

- the files added, changed and removed;
- `AGENTS.md`, `CLAUDE.md` and every skill that was added or changed, in full;
- every `data-prompt` and `iwe.prompt` string;
- every script, inline or in a file;
- every external URL.

A template is declined when its instructions or skills tell the agent to do
something its description does not say, when a prompt does something other than
what its button says, or when a script does something other than what its page
shows.

## Removal

Deleting a table drops the template from the next index. Workspaces already
created from it are not touched.

## Working on the validator

```
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

Snapshots under `crates/iwe-template/tests/snapshots/` are rewritten with
`UPDATE_SNAPSHOTS=1 cargo test`. Source files carry no comments; names say what
the code does.
