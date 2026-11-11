# Template report: `field-notes` `1.2.0`

- Name: `Field Notes`
- Description: `A small notebook with a board and one skill.`
- Authors: `Ada Example`
- License: `MIT`
- Not installed: `README.md, drafts`
- New template: read every file

## Files

18 added, 0 changed, 0 removed.

### Added

- `.claude/skills/plan/SKILL.md`
- `.claude/skills/plan/reference/steps.md`
- `.claude/skills/review/SKILL.md`
- `.gitignore`
- `.iwe/config.toml`
- `.iwe/schemas/note.yaml`
- `AGENTS.md`
- `CLAUDE.md`
- `LICENSE`
- `README.md`
- `data/assets/app.js`
- `data/assets/style.css`
- `data/board.html`
- `data/index.md`
- `data/notes/first.md`
- `drafts/idea.md`
- `preview.png`
- `template.toml`

## Agent instructions and skills

- skill `plan`: `.claude/skills/plan/SKILL.md` (added)
- skill `plan`: `.claude/skills/plan/reference/steps.md` (added)
- skill `review`: `.claude/skills/review/SKILL.md` (added)
- `AGENTS.md` (added)
- `CLAUDE.md` (added)

<details>
<summary>.claude/skills/plan/SKILL.md (added)</summary>

```markdown
# Plan

Write a plan before any change.
```

</details>

<details>
<summary>.claude/skills/plan/reference/steps.md (added)</summary>

```markdown
# Steps
```

</details>

<details>
<summary>.claude/skills/review/SKILL.md (added)</summary>

```markdown
# Review

Read the diff.
```

</details>

<details>
<summary>AGENTS.md (added)</summary>

```markdown
# Agents

Keep notes short.
```

</details>

<details>
<summary>CLAUDE.md (added)</summary>

```markdown
@AGENTS.md
```

</details>

## Prompts

- `data-prompt` in `data/board.html`

  ```text
  Triage the new notes
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

- `data/assets/app.js`, file, 3 lines
- `data/board.html` line 5, inline, 0 lines, loads `assets/app.js`
- `data/board.html` line 9, inline, 1 line
- `data/index.md` line 7, inline, 3 lines

<details>
<summary>data/assets/app.js line 1</summary>

```js
function render(root) {
  root.dataset.ready = "yes"
}
```

</details>

<details>
<summary>data/board.html line 9</summary>

```js
      render(document.body)
    
```

</details>

<details>
<summary>data/index.md line 7</summary>

```js
  document.querySelector("button").addEventListener("click", () => {
    iwe.prompt(`Review ${key} and list what is missing`)
  })
```

</details>

## External URLs

- `https://example.com/guide`: `data/index.md`
