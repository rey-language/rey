#!/usr/bin/env bash
# rey v0.3 toolchain doctor (toolchain-doctor agent). report-only: never fixes,
# never deletes, always exits 0. every check prints OK/WARN/FAIL + detail.
# report mode keeps checking after failures; unset variables and bad pipelines
# are still errors. Every external probe runs under a watchdog. Errexit is
# intentionally omitted so one failed probe cannot suppress the remaining report.

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OK_COUNT=0
WARN_COUNT=0
FAIL_COUNT=0

ok()     { OK_COUNT=$((OK_COUNT + 1));     printf 'OK   %s\n' "$*"; }
warn()   { WARN_COUNT=$((WARN_COUNT + 1)); printf 'WARN %s\n' "$*"; }
fail()   { FAIL_COUNT=$((FAIL_COUNT + 1)); printf 'FAIL %s\n' "$*"; }
detail() { printf '     %s\n' "$*"; }
section() { printf '\n== %s ==\n' "$*"; }

# Prefer standard timeout, then Homebrew coreutils. Perl is the macOS fallback.
TIMEOUT_BIN=""
if [ -x /usr/bin/timeout ]; then
  TIMEOUT_BIN="/usr/bin/timeout"
elif command -v timeout >/dev/null 2>&1; then
  TIMEOUT_BIN="$(command -v timeout)"
elif command -v gtimeout >/dev/null 2>&1; then
  TIMEOUT_BIN="$(command -v gtimeout)"
fi
PERL_BIN=""
if [ -x /usr/bin/perl ]; then
  PERL_BIN=/usr/bin/perl
elif command -v perl >/dev/null 2>&1; then
  PERL_BIN="$(command -v perl)"
fi

t_run() {
  local secs="$1"
  shift
  if [ -n "$TIMEOUT_BIN" ]; then
    "$TIMEOUT_BIN" "$secs" "$@"
  elif [ -n "$PERL_BIN" ]; then
    "$PERL_BIN" -e 'use POSIX qw(:sys_wait_h); my $seconds = shift; my $pid = fork(); die "fork failed: $!" unless defined $pid; if (!$pid) { exec @ARGV or exit 127; } $SIG{ALRM} = sub { kill 15, $pid; sleep 1; kill 9, $pid; waitpid($pid, 0); exit 124; }; alarm $seconds; waitpid($pid, 0); alarm 0; my $status = $?; exit(($status & 127) ? 128 + ($status & 127) : $status >> 8);' "$secs" "$@"
  else
    printf 'no timeout utility or Perl watchdog available\n' >&2
    return 125
  fi
}

first_line()  { local s="$1"; printf '%s\n' "${s%%$'\n'*}"; }
second_line() { local s="$1"; s="${s#*$'\n'}"; printf '%s\n' "${s%%$'\n'*}"; }

check_clang() {
  section "clang"
  local real="/usr/bin/clang"
  local out rc

  if [ -x "$real" ]; then
    out="$(t_run 15 "$real" --version 2>&1)"
    rc=$?
    if [ "$rc" -eq 0 ] && [ -n "$out" ]; then
      ok "/usr/bin/clang --version works: $(first_line "$out")"
    else
      fail "/usr/bin/clang --version failed (exit $rc): $(first_line "$out")"
    fi
  else
    fail "/usr/bin/clang missing or not executable (charter pins it for native builds)"
  fi

  local p=""
  p="$(command -v clang 2>/dev/null || true)"
  if [ -z "$p" ]; then
    warn "no 'clang' on PATH; invoke /usr/bin/clang explicitly"
  elif [ "$p" = "$real" ]; then
    ok "PATH clang resolves to /usr/bin/clang"
  else
    out="$(t_run 15 clang --version 2>&1)"
    rc=$?
    if [ "$rc" -eq 0 ]; then
      ok "PATH clang ($p) runs: $(first_line "$out")"
      detail "charter decision 11: compiler must use /usr/bin/clang explicitly"
    else
      if printf '%s\n' "$out" | grep -qi 'swift'; then
        warn "PATH clang ($p) is the broken swiftly shim (exit $rc) -- use /usr/bin/clang"
      else
        warn "PATH clang ($p) is broken (exit $rc) -- use /usr/bin/clang"
      fi
      detail "$(first_line "$out")"
      detail "$(second_line "$out")"
      detail "owner-only fixes: 'swiftly install Swift 6.3.1' or reorder PATH; agents never edit shell profiles"
    fi
  fi
}

check_rust() {
  section "rust toolchain"
  local tool out rc
  for tool in rustc cargo; do
    if ! command -v "$tool" >/dev/null 2>&1; then
      fail "$tool not on PATH (expected under ~/.cargo/bin; source ~/.cargo/env)"
      continue
    fi
    out="$(t_run 15 "$tool" --version 2>&1)"
    rc=$?
    if [ "$rc" -eq 0 ]; then
      ok "$tool: $(first_line "$out") [$(command -v "$tool")]"
    else
      fail "$tool at $(command -v "$tool") but '$tool --version' failed (exit $rc): $(first_line "$out")"
    fi
  done
  detail "v1 oracle build: cd compiler/v1 && cargo build --release"
}

check_reyc() {
  section "~/.reyc stdlib link"
  local d="$HOME/.reyc"
  if [ -L "$d" ] && [ ! -e "$d" ]; then
    warn "$d is a dangling symlink -> $(t_run 10 readlink "$d" 2>&1)"
    return 0
  fi
  if [ ! -e "$d" ]; then
    warn "$d does not exist (no stdlib link; std imports will fail)"
    return 0
  fi
  ok "$d exists"
  detail "contents: $(t_run 10 ls -A "$d" 2>/dev/null | tr '\n' ' ' | sed 's/ $//')"

  local s="$d/std"
  if [ -L "$s" ]; then
    local tgt
    tgt="$(t_run 10 readlink "$s" 2>&1)"
    if [ -e "$s" ]; then
      ok "~/.reyc/std is a live symlink -> $tgt"
    else
      warn "~/.reyc/std is a DANGLING symlink -> $tgt"
      detail "report only: agents never fix or delete it"
      if [ -d $STD_ROOT ]; then
        detail "live std checkout exists at $STD_ROOT"
        detail "recommended fix (owner approval required; outside repo): ln -sfn $STD_ROOT ~/.reyc/std"
      fi
    fi
  elif [ -d "$s" ]; then
    ok "~/.reyc/std is a real directory"
  else
    warn "~/.reyc/std missing"
  fi
}

check_git() {
  section "git health (metadata may be mid-repair by git-rescuer)"
  local out rc n

  out="$(cd "$REPO_ROOT" && t_run 20 git rev-parse --show-toplevel 2>&1)"
  rc=$?
  if [ "$rc" -eq 0 ]; then
    ok "git rev-parse --show-toplevel: $(first_line "$out")"
  elif [ "$rc" -eq 124 ]; then
    fail "git rev-parse --show-toplevel timed out after 20s"
  else
    fail "git rev-parse --show-toplevel failed (exit $rc): $(first_line "$out")"
  fi

  out="$(cd "$REPO_ROOT" && t_run 20 git status --porcelain 2>&1)"
  rc=$?
  if [ "$rc" -eq 0 ]; then
    if [ -z "$out" ]; then
      n=0
    else
      n="$(printf '%s\n' "$out" | wc -l | tr -d ' ')"
    fi
    ok "git status --porcelain: $n entry/entries"
  else
    fail "git status --porcelain failed (exit $rc)"
    detail "$(first_line "$out")"
    detail "expected mid-sprint: git-rescuer repairs .git worktree metadata in parallel; re-run doctor later"
  fi

  out="$(cd "$REPO_ROOT" && t_run 20 git rev-parse --short HEAD 2>&1)"
  rc=$?
  if [ "$rc" -eq 0 ]; then
    ok "git rev-parse HEAD: $(first_line "$out")"
  else
    fail "git rev-parse HEAD failed (exit $rc): $(first_line "$out")"
  fi
}

check_machine() {
  section "machine"
  local out rc avail_g mem mem_gb ncpu

  out="$(t_run 15 df -h /mnt/storage 2>&1)"
  rc=$?
  if [ "$rc" -eq 0 ]; then
    ok "df -h /mnt/storage:"
    printf '%s\n' "$out" | tail -n +2 | sed 's/^/     /'
    avail_g="$(t_run 15 df -g /mnt/storage 2>/dev/null | awk 'NR==2 {print $4}')"
    case "$avail_g" in
      ''|*[!0-9]*) : ;;
      *) [ "$avail_g" -lt 10 ] && warn "low disk headroom: ${avail_g} GiB free (cargo + clang builds want several GB)" ;;
    esac
  else
    fail "df -h /mnt/storage failed (exit $rc): $(first_line "$out")"
  fi

  out="$(t_run 10 sysctl -n hw.memsize 2>&1)"
  rc=$?
  case "$out" in
    ''|*[!0-9]*)
      fail "sysctl hw.memsize failed (exit $rc): $(first_line "$out")"
      ;;
    *)
      mem_gb=$((out / 1073741824))
      ok "RAM (hw.memsize): ${out} bytes = ${mem_gb} GiB"
      [ "$mem_gb" -lt 16 ] && detail "under 16 GiB: cap parallelism if the fleet OOMs (cargo build --release -j 4)"
      ;;
  esac

  out="$(t_run 10 sysctl -n hw.ncpu 2>&1)"
  rc=$?
  case "$out" in
    ''|*[!0-9]*) fail "sysctl hw.ncpu failed (exit $rc): $(first_line "$out")" ;;
    *) ok "CPUs (hw.ncpu): $out" ;;
  esac
}

check_artifacts() {
  section "key artifacts"
  local f n

  # native compiler binary (committed, arm64)
  f="$REPO_ROOT/rey-compiler/main"
  if [ -f "$f" ] && [ -x "$f" ]; then
    ok "rey-compiler/main: present + executable"
    detail "$(t_run 10 stat -f '%z bytes, modified %Sm' -t '%Y-%m-%d %H:%M' "$f" 2>/dev/null) [$(t_run 10 file -b "$f" 2>/dev/null)]"
  elif [ -f "$f" ]; then
    warn "rey-compiler/main exists but is not executable"
  else
    fail "rey-compiler/main missing (committed native binary; investigate, never delete)"
  fi

  # v1 oracle release binary
  f="$REPO_ROOT/compiler/v1/target/release/rey-v0"
  if [ -x "$f" ]; then
    ok "compiler/v1/target/release/rey-v0: present + executable"
    detail "not executed: artifact presence only (avoids invoking the engine or inheriting its diagnostic formatting)"
  elif [ -f "$f" ]; then
    warn "rey-v0 present but not executable"
  else
    warn "rey-v0 not built: cd compiler/v1 && cargo build --release"
  fi

  # conformance corpus
  if [ -d "$REPO_ROOT/tests/conformance" ]; then
    n="$(t_run 15 find "$REPO_ROOT/tests/conformance" -name main.rey -type f 2>/dev/null | wc -l | tr -d ' ')"
    ok "tests/conformance/: present ($n fixture(s) with main.rey)"
  else
    warn "tests/conformance/ missing (owned by harness-conformance + fixture agents; expected later in sprint)"
  fi

  # conformance runner
  if [ -f "$REPO_ROOT/scripts/conformance.sh" ]; then
    if [ -x "$REPO_ROOT/scripts/conformance.sh" ]; then
      ok "scripts/conformance.sh: present + executable"
    else
      warn "scripts/conformance.sh present but not executable ('bash scripts/conformance.sh' still works)"
    fi
  else
    warn "scripts/conformance.sh missing (owned by harness-conformance; expected later in sprint)"
  fi

  # timeout availability
  if [ -n "$TIMEOUT_BIN" ]; then
    ok "timeout watchdog: $TIMEOUT_BIN"
  elif [ -n "$PERL_BIN" ]; then
    warn "timeout utility not found; using Perl watchdog: $PERL_BIN"
  else
    fail "no watchdog (checked /usr/bin/timeout, PATH timeout, gtimeout, Perl); external probes are skipped/limited"
  fi
  if [ -x /usr/bin/timeout ]; then detail "/usr/bin/timeout: yes"; else detail "/usr/bin/timeout: no"; fi
  detail "PATH timeout: $(command -v timeout 2>/dev/null || echo 'not found')"
  detail "gtimeout: $(command -v gtimeout 2>/dev/null || echo 'not found')"
}

print_summary() {
  printf '\n== summary ==\n'
  printf 'SUMMARY: %d OK, %d WARN, %d FAIL (repo: %s)\n' "$OK_COUNT" "$WARN_COUNT" "$FAIL_COUNT" "$REPO_ROOT"
  printf 'note: report-only, exit is always 0. docs: docs/ENVIRONMENT.md\n'
}

printf 'rey toolchain doctor -- %s -- repo: %s\n' "$(date '+%Y-%m-%d %H:%M:%S')" "$REPO_ROOT"
check_clang
check_rust
check_reyc
check_git
check_machine
check_artifacts
print_summary
exit 0
