# Template report: `field-notes` `1.3.0`

- Name: `Field Notes`
- Description: `A small notebook with a board and one skill.`
- Authors: `Ada Example`
- License: `MIT`
- Not installed: `README.md, drafts`
- Update from version `1.2.0`
- Compare: <https://github.com/example/field-notes/compare/0123456789abcdef0123456789abcdef01234567...89abcdef0123456789abcdef0123456789abcdef>

## Files

2 added, 5 changed, 1 removed.

### Added

- `.claude/skills/ship/SKILL.md`
- `data/notes/second.md`

### Changed

- `.claude/skills/plan/SKILL.md`
- `AGENTS.md`
- `data/assets/app.js`
- `data/board.html`
- `template.toml`

### Removed

- `data/notes/first.md`

## Agent instructions and skills

- skill `plan`: `.claude/skills/plan/SKILL.md` (changed)
- skill `ship`: `.claude/skills/ship/SKILL.md` (added)
- `AGENTS.md` (changed)

<details>
<summary>.claude/skills/plan/SKILL.md (changed)</summary>

```markdown
# Plan

Write a plan, then push it to the remote.
```

</details>

<details>
<summary>.claude/skills/ship/SKILL.md (added)</summary>

```markdown
# Ship

Tag the release.
```

</details>

<details>
<summary>AGENTS.md (changed)</summary>

```markdown
# Agents

Keep notes short and dated.
```

</details>

## Prompts

- `data-prompt` in `data/board.html` (new)

  ```text
  Archive every note older than a year
  ```

- `data-prompt` in `data/index.md`

  ```text
  Summarize this week's notes
  ```

- `iwe.prompt` in `data/index.md`

  ```text
  `Review ${key} and list what is missing`
  ```


## Scripts

- `data/assets/app.js`, file, 4 lines (new or changed)
- `data/board.html` line 5, inline, 0 lines, loads `assets/app.js` (unchanged)
- `data/board.html` line 9, inline, 1 line (unchanged)
- `data/index.md` line 7, inline, 3 lines (unchanged)

<details>
<summary>data/assets/app.js line 1</summary>

```js
function render(root) {
  root.dataset.ready = "yes"
}
render.version = 2
```

</details>

## External URLs

- `https://example.com/guide`: `data/index.md`
- `https://example.org/second` (new): `data/notes/second.md`
