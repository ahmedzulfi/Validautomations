#!/usr/bin/env bash
# Publish every glowkit crate to crates.io in dependency order.
#
#   ./scripts/publish.sh            publish for real (needs `cargo login` first)
#   ./scripts/publish.sh --dry-run  package and verify each crate without uploading
#   PAUSE=60 ./scripts/publish.sh   seconds to wait between crates (default 30), so the index
#                                   has the previous crate before the next one depends on it
#   FROM=glowkit-ai ./scripts/publish.sh   resume from a crate after a failure
#
# --dry-run uses `cargo publish --workspace --dry-run` (cargo 1.90+), which packages and verifies
# every crate against a local overlay of its unpublished siblings, so all of them can be checked
# before anything is on crates.io. On older cargo it falls back to one crate at a time, where
# crates whose glowkit dependencies are not published yet can only be packaged, not verified.
set -euo pipefail
cd "$(dirname "$0")/.."

DRY_RUN=0
for arg in "$@"; do
  case "$arg" in
    --dry-run) DRY_RUN=1 ;;
    -h|--help) sed -n '2,13p' "$0"; exit 0 ;;
    *) echo "unknown argument: $arg" >&2; exit 2 ;;
  esac
done
PAUSE="${PAUSE:-30}"

# Dependency order (each crate only depends on crates above it):
#   glowkit-core
#   leaf UI crates that need only glowkit-core
#   crates that build on those
#   the umbrella crate
#   the tools (no glowkit dependencies; last so the docs they point at are live)
CRATES=(
  glowkit-core
  glowkit-text
  glowkit-backgrounds
  glowkit-effects
  glowkit-components
  glowkit-charts
  glowkit-ai
  glowkit-blocks
  glowkit-motion
  glowkit-3d
  glowkit-canvas
  glowkit-interactions   # core, effects
  glowkit-agents         # core, effects, ai, charts
  glowkit-templates      # core, components, charts, text, ai, effects, backgrounds
  glowkit-stream         # core, ai (optional)
  glowkit                # everything above
  glowkit-mcp
  glowkit-cli
)

if [[ -n "$(git status --porcelain 2>/dev/null)" ]]; then
  echo "warning: the working tree has uncommitted changes" >&2
  [[ $DRY_RUN == 1 ]] || { echo "commit or stash them before publishing" >&2; exit 1; }
fi

# glowkit-cli ships every component's source in sources.bundle (see crates/glowkit-cli/build.rs).
# Regenerated every run so it always matches the sources being published.
echo "==> generating crates/glowkit-cli/sources.bundle"
GLOWKIT_CLI_WRITE_BUNDLE=1 cargo build --locked -q -p glowkit-cli

if [[ $DRY_RUN == 1 && -z "${FROM:-}" ]]; then
  echo "==> cargo publish --workspace --dry-run"
  if cargo publish --workspace --dry-run --locked --allow-dirty; then
    echo
    echo "dry run passed for: ${CRATES[*]}"
    exit 0
  fi
  echo "workspace dry run failed or is unsupported (cargo < 1.90); checking crates one by one" >&2
fi

started=0
[[ -z "${FROM:-}" ]] && started=1
passed=()
packaged=()
failed=()
for crate in "${CRATES[@]}"; do
  if [[ $started == 0 ]]; then
    [[ "$crate" == "$FROM" ]] && started=1 || continue
  fi
  if [[ $DRY_RUN == 1 ]]; then
    echo "==> cargo publish --dry-run -p $crate"
    if cargo publish --dry-run --locked -p "$crate" --allow-dirty; then
      passed+=("$crate")
    else
      echo "    dry run failed (expected while its glowkit dependencies are unpublished); checking it packages"
      if cargo package --list --locked -p "$crate" --allow-dirty > /dev/null; then
        packaged+=("$crate")
      else
        failed+=("$crate")
      fi
    fi
  else
    echo "==> cargo publish -p $crate"
    cargo publish --locked -p "$crate"
    if [[ "$crate" != "${CRATES[-1]}" ]]; then
      echo "    waiting ${PAUSE}s for the index"
      sleep "$PAUSE"
    fi
  fi
done

if [[ $DRY_RUN == 1 ]]; then
  echo
  echo "dry run passed:         ${passed[*]:-none}"
  echo "packaged (deps unpublished): ${packaged[*]:-none}"
  echo "failed:                 ${failed[*]:-none}"
  [[ ${#failed[@]} == 0 ]]
else
  echo "all crates published"
fi
