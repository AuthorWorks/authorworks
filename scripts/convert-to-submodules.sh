#!/usr/bin/env bash
set -euo pipefail

APPLY=false
if [[ "${1:-}" == "--apply" ]]; then
  APPLY=true
fi

echo "Converting nested repos to submodules (APPLY=$APPLY)"

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "Top-level is not a git repository. Run: git init && git add . && git commit -m 'init'"
  exit 1
fi

declare -a REPOS
while IFS= read -r d; do REPOS+=("$d"); done < <(ls -1d authorworks-* authorworks authorworks-platform authorworks-ui 2>/dev/null || true)

for d in "${REPOS[@]:-}"; do
  [[ -d "$d" ]] || continue
  if [[ ! -d "$d/.git" ]]; then
    echo "[skip] $d is not a git repo"
    continue
  fi
  origin=$(git -C "$d" remote get-url origin 2>/dev/null || true)
  if [[ -z "$origin" ]]; then
    echo "[warn] $d has no origin; skipping"
    continue
  fi

  echo "[plan] $d -> $origin"
  if $APPLY; then
    backup="${d}.localbak.$(date +%s)"
    echo "  - moving $d to $backup"
    mv "$d" "$backup"
    echo "  - adding submodule $d"
    git submodule add "$origin" "$d" || {
      echo "  ! failed to add submodule for $d; restoring backup"
      rm -rf "$d" || true
      mv "$backup" "$d"
      exit 1
    }
    echo "  - submodule added; local changes preserved at $backup"
  fi
done

if $APPLY; then
  git add .gitmodules ${REPOS[@]} || true
  echo "Review changes and commit: git commit -m 'Convert nested repos to submodules'"
fi

