# iWe workspace templates

The registry of community workspace templates for [iWe](https://iwe.md). A
template is the starting content of a whole workspace: documents, schemas, the
workspace config, and the instructions and skills the agent runs with.

Templates live in their authors' own GitHub repositories. This repository lists
them in [`templates.toml`](templates.toml), each pinned to a commit a maintainer
reviewed, and publishes what the app reads. Nothing an author pushes reaches a
user until a pull request here moves the pinned revision.

## How the app uses it

- The gallery (File ▸ New Workspace from Template…) reads one file, the index:
  `https://github.com/iwe-org/templates/releases/download/index/index.json`.
- Creating a workspace downloads one archive named by the index, checks its
  SHA-256, unpacks it in memory, runs the same validator this repository's CI
  ran, and only then writes files.
- The app never contacts a template's own repository.

## What is in here

| Path | What it is |
| --- | --- |
| `templates.toml` | The entries: repository, revision, version, path |
| `crates/iwe-template` | The validator library and the `iwe-template` binary |
| `.github/workflows/check.yml` | On a pull request: validates each changed entry and writes the reviewer's report to the check summary |
| `.github/workflows/publish.yml` | On merge: packages each new version as a release, then rebuilds the index |
| `.github/workflows/stars.yml` | Daily: refreshes the star counts in the index |
| `.github/workflows/bump.yml` | A reusable workflow a template repository runs on a tag to open its update pull request |
| `scripts/` | The shell the workflows share |

## Releases

- `<id>-<version>` holds `<id>-<version>.tar.gz`, `entry.json` (the template's
  index record) and `preview.png` when the template has one. A published
  archive is never replaced; a change is a new version.
- `index` is a rolling release that holds `index.json`.

## The `iwe-template` tool

```
cargo install --git https://github.com/iwe-org/templates iwe-template

iwe-template check <dir> [--id <id> --version <version> [--previous-version <version>]]
iwe-template report <dir> [--since <dir>] [--repository <url> --rev <sha> --since-rev <sha>]
iwe-template pack <dir> --out <dir> [--repository <url>] [--rev <sha>] [--published <date>]
iwe-template index --entries <dir> [--registry templates.toml] [--stars <file>] [--base-url <url>] [--generated <time>] [--out <file>]
iwe-template entries [templates.toml] [--since <older templates.toml>]
```

`check` exits 0 when the template passes, 1 when it breaks a rule, and 2 when
it cannot be read.

The library is used by the app with the command-line interface switched off:

``` toml
iwe-template = { git = "https://github.com/iwe-org/templates", rev = "<commit>", default-features = false }
```

## Submitting a template

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

The registry and the validator are MIT licensed. Each template carries its own
license, named in its `template.toml`.
