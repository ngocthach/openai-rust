#!/usr/bin/env bash
# Compare the openai-python baseline this crate was ported from against
# upstream main, and show what needs porting. See docs/upstream-sync.md.
set -euo pipefail

# Baseline commit the Rust port matches — keep in sync with docs/upstream-sync.md.
BASELINE_COMMIT="6d9262d5c666a1e4d47f63178db907ba3087ac5d"
BASELINE_VERSION="2.44.0"

OPENAI_PYTHON_DIR="${OPENAI_PYTHON_DIR:-$(cd "$(dirname "$0")/.." && pwd)/../openai-python}"

if [ ! -d "$OPENAI_PYTHON_DIR/.git" ]; then
  echo "error: openai-python clone not found at $OPENAI_PYTHON_DIR" >&2
  echo "clone it: git clone https://github.com/openai/openai-python.git" >&2
  echo "or set OPENAI_PYTHON_DIR" >&2
  exit 1
fi

cd "$OPENAI_PYTHON_DIR"
echo "==> Fetching upstream..."
git fetch --quiet origin main

LATEST_COMMIT=$(git rev-parse origin/main)
LATEST_VERSION=$(git show origin/main:src/openai/_version.py | sed -n 's/__version__ = "\(.*\)".*/\1/p')

echo
echo "baseline : v$BASELINE_VERSION (${BASELINE_COMMIT:0:9})"
echo "upstream : v$LATEST_VERSION (${LATEST_COMMIT:0:9})"

if [ "$BASELINE_COMMIT" = "$LATEST_COMMIT" ]; then
  echo "==> Up to date. Nothing to port."
  exit 0
fi

echo
echo "==> Commits since baseline: $(git rev-list --count "$BASELINE_COMMIT..origin/main")"
git log --oneline "$BASELINE_COMMIT..origin/main" | head -40

echo
echo "==> Changelog entries since baseline (top):"
git diff "$BASELINE_COMMIT" origin/main -- CHANGELOG.md | grep '^+' | grep -v '^+++' | head -60

echo
echo "==> API surface diff (api.md):"
git diff --stat "$BASELINE_COMMIT" origin/main -- api.md
git diff "$BASELINE_COMMIT" origin/main -- api.md | head -200

echo
echo "==> Core-module changes (files the Rust port mirrors):"
git diff --stat "$BASELINE_COMMIT" origin/main -- \
  src/openai/_base_client.py \
  src/openai/_client.py \
  src/openai/_constants.py \
  src/openai/_exceptions.py \
  src/openai/_streaming.py \
  src/openai/pagination.py \
  src/openai/resources/webhooks.py \
  src/openai/lib/azure.py \
  src/openai/resources/realtime/realtime.py

echo
echo "==> Ported-resource changes (types + resources dirs, stat only):"
git diff --stat "$BASELINE_COMMIT" origin/main -- \
  src/openai/resources src/openai/types | tail -15

echo
echo "Next: triage the api.md diff, port per docs/upstream-sync.md, then"
echo "update BASELINE_COMMIT/BASELINE_VERSION here and in docs/upstream-sync.md."
