#!/usr/bin/env bash
# Headless screenshots of the GPUI gallery: Xvfb + Mesa lavapipe (software Vulkan).
#
#   scripts/screenshot.sh OUT_DIR [page[:theme] ...]
#
# e.g. scripts/screenshot.sh /tmp/shots overview chat:daylight sessions
# Needs: xvfb, imagemagick (import), xdotool, x11-utils, mesa-vulkan-drivers.
# Env: WAIT (seconds after map, default 3), HOVER="x y" to move the pointer before capture
#      (window-relative), BIN (gallery binary, default target/debug/glowkit-gpui-gallery).
set -euo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"
out="${1:?usage: screenshot.sh OUT_DIR [page[:theme] ...]}"; shift
mkdir -p "$out"
bin="${BIN:-$here/target/debug/glowkit-gpui-gallery}"
export VK_ICD_FILENAMES="${VK_ICD_FILENAMES:-/usr/share/vulkan/icd.d/lvp_icd.json}"
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-$(mktemp -d)}"
pages=("$@"); [ ${#pages[@]} -eq 0 ] && pages=(overview)
for spec in "${pages[@]}"; do
  page="${spec%%:*}"; theme="midnight"; [[ "$spec" == *:* ]] && theme="${spec#*:}"
  xvfb-run -a -s "-screen 0 1400x900x24" bash -c '
    "$0" --page "$1" --theme "$2" >/dev/null 2>&1 & pid=$!
    for _ in $(seq 50); do w=$(xdotool search --name "glowkit for GPUI" 2>/dev/null | head -1); [ -n "$w" ] && break; sleep 0.2; done
    # gpui 0.2 on a bare X server (no window manager) misses its first MapNotify and never starts
    # its frame loop; re-mapping the window kicks it.
    xdotool windowunmap "$w"; sleep 0.3; xdotool windowmap "$w"; sleep 0.5
    if [ -n "${HOVER:-}" ]; then xdotool mousemove --window "$w" $HOVER; sleep 0.4; xdotool mousemove_relative 1 1; fi
    sleep "${WAIT:-3}"
    import -window "$w" "$3"
    kill $pid' "$bin" "$page" "$theme" "$out/$page-$theme.png"
  echo "$out/$page-$theme.png"
done
