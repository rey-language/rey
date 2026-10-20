# Repository state baseline

Read-only audit, 2026-10-16. Root is on `master` at `f308e2f4d32dbb55118521ae9c1c7b1ff4f94201` (`rescue/pre-v0.3` points there).

## Safe inspection

These commands were verified usable without optional index refreshes; all returned exit 0, and `.git/index` was byte-identical before and after the final check:

```sh
GIT_OPTIONAL_LOCKS=0 git status --short --ignore-submodules=all
GIT_OPTIONAL_LOCKS=0 git diff --ignore-submodules=all
GIT_OPTIONAL_LOCKS=0 git diff --cached --ignore-submodules=all
```

Ignoring submodules is only a read-only workaround for the broken nested gitlink; it does **not** repair normal `git status` or `git diff`. Do not add persistent submodule-ignore config to hide the problem.

## Preserved pre-existing WIP

The original metadata backup is `/private/var/folders/dt/_kd6szqs4gxct6stfgskt30m0000gn/T/opencode/rey-v03/git-backup/`: `config` SHA-256 `156491291a1870ef3220a750ea92c2412a4c906c70daa11125f946b3d43ba524`; `HEAD` `f6f2b945f6c411b02ba3da9c7ace88dcf71b6af65ba2e0d89aa82900042b5a10`; `packed-refs` `ca98795282beb9ae5dfc18bfa6cfb696cf34feb05ac87c1c631307d9e362886c`; and initial `index` `3275e588d78c33ed8695028fb4be3d7efe38fbef10ceabab7eb8a9a3c11873f1`. A later saved `index-after-status` is also retained (SHA-256 `0c7a13554b7bfdcd0aef0febb1ce4c7daafdf22c559f1163873d95f3215e7dda`); at capture, its 237 stage entries and cached patch matched the initial snapshot. The refs backup contains 18 files; per-file checksums are in `refs-sha256.txt` (manifest SHA-256 `2f720e174e7c147b88ceb4e579a5f54960024df8d564dc8ba20ef57e783b5545`). `.git/worktrees/` was absent; `worktree-diagnosis.txt` records that, and `nested-worktree.gitfile` preserves the stale pointer (SHA-256 `8ee4349833112a7731fc88b597e2de66c59c3f311a1d696ee3c2d28b1626b5e2`).

The initial saved index had 237 entries. Compared with HEAD, its staged delta was **59 files, 30,392 insertions, 1,080 deletions**. Treat this as pre-existing user/WIP state: do not clean, unstage, or auto-commit it. The exact binary patch is `/private/var/folders/dt/_kd6szqs4gxct6stfgskt30m0000gn/T/opencode/rey-v03/staged-state.patch` (1,911,480 bytes; SHA-256 `ffa0e4ee3184d3699c53596690ab2560493d5e83caf3b5e714748e50c0c13fbe`); staged paths are in `/private/var/folders/dt/_kd6szqs4gxct6stfgskt30m0000gn/T/opencode/rey-v03/preexisting-staged-paths.txt`.

The untouched stash snapshot is `/private/var/folders/dt/_kd6szqs4gxct6stfgskt30m0000gn/T/opencode/rey-v03/stashes.txt` and contains eight entries. No stash operation was performed.

## Unrepaired nested gitlink

The index has mode `160000`, object `f308e2f4d32dbb55118521ae9c1c7b1ff4f94201`, at `.claude/worktrees/work-on-the-standard-validated-nova`; that path is absent from HEAD. Its nested `.git` file points to `/Users/misbahkhursheed/Developer/rey-language/rey-lang/.git/worktrees/work-on-the-standard-validated-nova`, but the old checkout and current `.git/worktrees/` admin directory are absent. `git worktree list` shows only the root worktree; the matching `worktree-work-on-the-standard-validated-nova` branch ref remains. Do not reconstruct admin metadata or redirect the pointer without the original per-worktree state. Normal status/diff therefore remain blocked.

`.git/config` also has a stale `core.hooksPath` under the old checkout. Local `user.name` is `codex`; it was left unchanged. The hooks path may prevent intended hooks from running.

## Owner decisions before repair or landing

1. Choose whether to recover the exact nested worktree admin directory/index from a trusted backup, or—if unavailable—explicitly decide whether the staged gitlink is intentional WIP to retain or authorize a separate, backed-up index operation to unstage it. No choice has been applied.
2. Approve changing `core.hooksPath` to this checkout’s `.git/hooks`, or specify the intended hooks directory, before relying on commit hooks.
3. Review and explicitly choose whether to include the 59-file WIP delta in the eventual single landing commit, split it, or direct another handling; explicitly approve the landing before any commit/push. No commit, push, reset, branch switch, cleanup, or deletion was performed for this audit.
