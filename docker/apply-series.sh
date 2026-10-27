#!/bin/sh
# shellcheck shell=dash
# Applies the patch series plus overlays onto a pristine upstream clone.
#
# This mirrors `jx apply` for the fresh-tree case (image builds always start
# from a clean checkout, so no idempotency or three-way fallback is needed:
# any failure aborts the build layer, leaving no half-applied state behind).
# It also mirrors the `jx lint` pre-checks that matter for builds: every
# series entry must exist, and no overlay may collide with an upstream file.
#
# Usage: apply-series.sh <upstream-dir> <patches-dir> <series-file> <overlays-dir> <expected-sha>
set -eu

UPSTREAM_DIR="${1:?upstream dir required}"
PATCHES_DIR="${2:?patches dir required}"
SERIES_FILE="${3:?series file required}"
OVERLAYS_DIR="${4:?overlays dir required}"
EXPECTED_SHA="${5:?expected upstream SHA required}"

HEAD_SHA="$(git -C "$UPSTREAM_DIR" rev-parse HEAD)"
if [ "$HEAD_SHA" != "$EXPECTED_SHA" ]; then
  echo "error: upstream HEAD ${HEAD_SHA} != pinned ${EXPECTED_SHA}" >&2
  exit 1
fi

# Parse the series exactly like `jx lint` (quilt style): a `#` preceded by
# whitespace or at column 0 starts a comment; blanks are ignored.
ENTRIES="$(sed -e 's/[[:space:]]#.*$//' -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//' "$SERIES_FILE" | sed -e '/^$/d' -e '/^#/d')"
if [ -z "$ENTRIES" ]; then
  echo "error: series is empty: $SERIES_FILE" >&2
  exit 1
fi

# Fail fast before touching the tree: every entry must resolve to a patch file.
COUNT=0
for entry in $ENTRIES; do
  if [ ! -f "$PATCHES_DIR/$entry" ]; then
    echo "error: series entry has no patch file: $entry" >&2
    exit 1
  fi
  COUNT=$((COUNT + 1))
done
echo "series: $COUNT patch(es), HEAD matches $EXPECTED_SHA"

# Overlay collision check (same rule as `jx apply`): an overlay must be a new
# file. Checking against HEAD is exact here because the clone is pristine.
if [ -d "$OVERLAYS_DIR" ]; then
  COLLISIONS=0
  for overlay in $(cd "$OVERLAYS_DIR" && find . -type f | sed 's|^\./||' | sort); do
    if git -C "$UPSTREAM_DIR" cat-file -e "HEAD:$overlay" 2>/dev/null; then
      echo "error: overlay collides with upstream file: $overlay (use a patch instead)" >&2
      COLLISIONS=$((COLLISIONS + 1))
    fi
  done
  if [ "$COLLISIONS" -gt 0 ]; then
    exit 1
  fi
fi

# Apply in series order and verify each patch is immediately reversible at its
# position in the stack (later patches like 0024-gateway may layer on top of
# files touched by earlier i18n patches).
APPLIED=0
for entry in $ENTRIES; do
  echo "applying $entry"
  git -C "$UPSTREAM_DIR" apply --verbose "$PATCHES_DIR/$entry"
  git -C "$UPSTREAM_DIR" apply --check --reverse "$PATCHES_DIR/$entry"
  APPLIED=$((APPLIED + 1))
done

if [ "$APPLIED" -ne "$COUNT" ]; then
  echo "error: applied $APPLIED of $COUNT patches" >&2
  exit 1
fi
echo "verified: $APPLIED patch(es) applied and reverse-check clean"

# Overlays win over patches and are copied last.
if [ -d "$OVERLAYS_DIR" ]; then
  cp -r "$OVERLAYS_DIR/." "$UPSTREAM_DIR/"
  echo "overlays copied from $OVERLAYS_DIR"
fi
