#!/usr/bin/env bash
set -euo pipefail

echo "Planning submodule conversion for AuthorWorks workspace"
repos=(authorworks-* authorworks author_works authorworks-platform authorworks-ui)

for d in "${repos[@]}"; do
  [[ -d "$d" ]] || continue
  if [[ -d "$d/.git" ]]; then
    origin=$(git -C "$d" remote get-url origin 2>/dev/null || echo "(no origin)")
    echo "- $d -> $origin"
  else
    echo "- $d (not a git repo)"
  fi
done

cat <<EOM

Next steps:
1) Initialize a top-level git repo if missing: git init && git add . && git commit -m "Initialize AuthorWorks umbrella"
2) Review origins above. For any "(no origin)", set one or skip.
3) Run ./scripts/convert-to-submodules.sh --apply to convert. This will create backups of local directories.
EOM

