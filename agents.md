# AGENTS.md — Codex contributor setup

You are an autonomous contributor on this project. Read this entire file before
doing anything. Then read CLAUDE.md, primer.md, and CHANGELOG.md in that order.

---

# Your identity

Before doing anything else, set your git identity:
```bash
git config user.name "codex"
git config user.email "codex@users.noreply.github.com"
```

This ensures commits are attributed correctly without exposing personal emails.

---

# Critical rules — never break these

- Never touch `main` or `master` directly. Ever.
- Never commit or push to main. Ever.
- Never delete files without explicitly telling Misbah first.
- Never rewrite an entire file when a small fix is needed. Surgical changes only.
- Never add dependencies without telling Misbah first.
- Never push without running verification first.

---

# Who you are working with

- Misbah is the owner. He codes on `main`/`master`.
- You are a contributor. You code on the `codex` branch.
- You maintain `primer.md` — rewrite it at the start and end of every session.
- You update `CHANGELOG.md` whenever you merge, fix conflicts, or resolve issues.
- You update `CLAUDE.md` when the project meaningfully evolves.

---

# Session start — do this every time

1. Set your git identity (see above)
2. Check which branch you're on:
```bash
git branch --show-current
```
3. If not on `codex` branch, switch to it:
```bash
git checkout codex 2>/dev/null || git checkout -b codex
git push -u origin codex 2>/dev/null || true
```
4. Read `primer.md` — this is where you left off last session
5. Read `CLAUDE.md` — project context and conventions
6. Run git log to understand recent history:
```bash
git log --oneline -10
git log main..codex --oneline
```
7. Rewrite `primer.md` with current state before starting work
8. Ask Misbah what to work on — or if he says "go ahead", check primer.md
   for the next unimplemented feature and start there

---

# How to work

- State what you're about to do before doing it
- Make small, targeted changes. Don't refactor unless asked.
- Match existing code style — camelCase, compact, minimal comments
- Comments only when non-obvious. Style: `// bad workaround - does x`
- Explain the WHY behind decisions, not just what you changed
- If something is wrong or Misbah has a bad approach, say so directly

---

# Git workflow

## Committing
After meaningful work:
```bash
git add -A
git commit -m "type(scope): description"
git push origin codex
```

Conventional commit format:
- `feat(lexer): add comment tokenization`
- `fix(parser): handle empty array literal`
- `chore(codex): update primer`

Commit frequently. Do not batch large changes unnecessarily.

## Pull Requests
- ONLY open a PR when a feature is **fully implemented and verified**
- Do NOT open PRs for partial, incomplete, or work-in-progress features

When a feature is complete:
```bash
gh pr create --base main --head codex \
  --title "feat: your feature title" \
  --body "## What
brief description of what was done

## Why
reasoning behind the approach

## Notes
anything Misbah should pay attention to when reviewing"
```

---

# Never do this

```bash
git push origin main    # NEVER
git checkout main       # NEVER (read only if needed)
git merge main          # use rebase instead
```

---

# Autonomous mode

If Misbah says "go ahead" or "keep going":

1. Check primer.md for the next unimplemented feature
2. Implement it fully
3. Verify it works (compile, run tests if they exist)
4. Commit and push to codex branch
5. Open a PR ONLY if the feature is fully complete
6. Pick the next feature from the list and repeat
7. Don't stop until the session ends or you're genuinely stuck
8. If stuck — document exactly what's blocking in primer.md, commit, stop

---

# Verification — always do this before saying done

- Rust project → `cargo build`, check warnings, `cargo test` if tests exist
- .rey files → `cargo run -- .rey`, check output
- Never say done without verifying

---

# Code style

- camelCase for everything
- Compact formatting, no excessive blank lines
- Comments minimal and lowercase
- No abstractions that weren't asked for
- No premature optimization
- Match whatever style already exists in the file

---

# Session end — do this before stopping

1. Commit any uncommitted work
2. Push to codex branch
3. Open a PR ONLY if a feature is fully complete
4. Rewrite primer.md with:
   - what was done this session
   - current state of the project
   - what's next
   - anything that's blocked or broken
5. Commit the updated primer:
```bash
git add primer.md CHANGELOG.md
git commit -m "chore(codex): update primer and changelog"
git push origin codex
```

---

# Conflict resolution

If merging causes conflicts:
1. Read both sides, understand what each was trying to do
2. Resolve correctly — don't just pick one side
3. Log the conflict in CHANGELOG.md:
```
## [sync] — date
### Conflicts resolved
- path/to/file — what conflicted and how it was resolved
```

---

# Critical rules (repeated — read this last)

- Never touch main. Ever.
- Never delete files without asking.
- Never rewrite entire files for small fixes.
- Never add dependencies without asking.
- Always verify before saying done.

