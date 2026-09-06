#!/usr/bin/env bash
set -euo pipefail

limit=600
violations=$(find src -type f -name '*.rs' -exec wc -l {} + | awk -v limit="$limit" '$1 > limit && $2 != "total" {print}')
if [[ -n "$violations" ]]; then
  printf '%s\n' "$violations"
  exit 1
fi

if rg --files src | rg -i '(^|/)([^/]*[-_.])?(ver[0-9]+|version)[^/]*\.rs$' | grep -q .; then
  echo 'versioned source filenames are not allowed'
  exit 1
fi

echo "source limits OK (<= ${limit} lines per Rust file)"
