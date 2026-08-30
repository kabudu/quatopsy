#!/usr/bin/env bash
# Build the presentation-only GitHub Pages site from pinned local inputs.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
output="${1:-$root/target/site}"

case "$output" in
  "$root/target/"*) ;;
  *) printf 'build-site: output must be below target/: %s\n' "$output" >&2; exit 1 ;;
esac

[[ -x "$root/website/node_modules/.bin/tailwindcss" ]] || {
  printf 'build-site: website dependencies are missing; run npm --prefix website ci --ignore-scripts\n' >&2
  exit 1
}

(
  cd "$root/website"
  npm run build
)

mkdir -p "$output/assets"
cp "$root/website/index.html" "$output/index.html"
cp "$root/website/styles.css" "$output/styles.css"
cp "$root/website/robots.txt" "$output/robots.txt"
cp "$root/website/sitemap.xml" "$output/sitemap.xml"
cp "$root/assets/brand/source/quatopsy-symbol.svg" "$output/assets/quatopsy-symbol.svg"
cp "$root/assets/brand/source/quatopsy-lockup-universal.svg" "$output/assets/quatopsy-lockup-universal.svg"
cp "$root/assets/brand/templates/diagram-workflow.svg" "$output/assets/architecture.svg"
cp "$root/assets/brand/templates/diagram-workflow-narrow.svg" "$output/assets/architecture-narrow.svg"
cp "$root/assets/brand/exports/study-lifted-path.png" "$output/assets/study-lifted-path.png"
cp "$root/assets/brand/exports/study-quotient-lens.png" "$output/assets/study-quotient-lens.png"
cp "$root/assets/brand/exports/social-og-1200x630.png" "$output/assets/quatopsy-social.png"
cp "$root/assets/brand/exports/favicon-32.png" "$output/assets/favicon-32.png"
cp "$root/assets/brand/exports/apple-touch-180.png" "$output/assets/apple-touch-180.png"
printf '\n' > "$output/.nojekyll"

python3 "$root/scripts/check-site.py" "$output"
printf 'build-site: built %s\n' "$output"
