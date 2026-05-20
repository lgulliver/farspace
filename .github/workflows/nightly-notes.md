---
name: nightly-notes
description: Generate meaningful release notes for a FARSPACE nightly build
on:
  workflow_call:
    inputs:
      nightly_tag:
        description: Nightly tag being created (e.g. nightly-20260520-1430)
        required: true
        type: string
permissions:
  contents: read
  issues: read
  pull-requests: read
engine: claude
strict: true
timeout-minutes: 10
network:
  allowed: [defaults, github]
tools:
  github:
    mode: gh-proxy
    toolsets: [default]
  bash: ["git", "grep", "wc", "head", "tail", "cut", "date", "tee", "cat"]
safe-outputs:
  upload-artifact:
    max-uploads: 1
    allowed-paths:
      - "release-notes.md"
---

# Generate FARSPACE Nightly Release Notes

You are writing release notes for **FARSPACE**, a deterministic turn-based 4X space strategy game with a terminal TUI, written in Rust.

Nightly tag: `${{ inputs.nightly_tag }}`

## Instructions

1. Find the most recent `v*` release tag:
   ```
   git tag --list 'v*' --sort=-version:refname | head -1
   ```
   If no release tag exists, use the full history.

2. Collect non-merge commits since that tag (subject lines only):
   ```
   git log <last-tag>..HEAD --no-merges --pretty=format:"%s"
   ```

3. For any commits that look significant (new features, bug fixes, gameplay changes), inspect the actual diff to understand the real impact:
   ```
   git log <last-tag>..HEAD --no-merges --pretty=format:"%H %s" | head -30
   git show --stat <hash>
   git diff <last-tag>..HEAD -- crates/game_core/src/ | head -200
   git diff <last-tag>..HEAD -- crates/game_tui/src/ | head -200
   ```

4. Write player- and contributor-friendly release notes. Group under these sections (omit any with nothing relevant):
   - **New Features** — new gameplay, commands, screens, or content
   - **Improvements** — UX polish, balance tuning, performance
   - **Bug Fixes** — broken things now fixed
   - **Under the Hood** — refactors, test coverage, CI, dependencies (brief)

   Rules:
   - One sentence per bullet, describing WHAT changed and WHY it matters
   - Skip pure chore commits (fmt, clippy lint-only) unless they signal something notable
   - Do not invent changes not evidenced in the commits or diffs
   - Do not add a title or date header

5. Write the final notes to `release-notes.md` — nothing else in that file.

6. Upload `release-notes.md` as an artifact named `release-notes`.
