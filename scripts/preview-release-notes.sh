#!/usr/bin/env bash
# Render curated notes to a local HTML preview for desktop and narrow widths.
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
python3 "$root/scripts/check-release-notes.py"
version="$(python3 - "$root/Cargo.toml" <<'PY'
import re
import sys
from pathlib import Path
text = Path(sys.argv[1]).read_text(encoding="utf-8")
print(re.search(r'^version = "([^"]+)"', text, re.M).group(1))
PY
)"
notes="$root/.github/release-notes/v${version}.md"
out="${1:-"$root/dist/release-notes-preview.html"}"
mkdir -p "$(dirname "$out")"
python3 - "$notes" "$out" <<'PY'
import html
import re
import sys
from pathlib import Path

src = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
title = html.escape(src[0])
body_src = "\n".join(src[2:])


def inline(text: str) -> str:
    text = html.escape(text)
    text = re.sub(r"\*\*(.+?)\*\*", r"<strong>\1</strong>", text)
    text = re.sub(r"`([^`]+)`", r"<code>\1</code>", text)
    text = re.sub(r"\[([^\]]+)\]\(([^)]+)\)", r'<a href="\2">\1</a>', text)
    return text


def render(md: str) -> str:
    parts: list[str] = []
    fence = False
    code: list[str] = []
    list_items: list[str] = []

    def close_list() -> None:
        if list_items:
            parts.append("<ul>" + "".join(f"<li>{item}</li>" for item in list_items) + "</ul>")
            list_items.clear()

    for line in md.splitlines():
        if line.strip().startswith("```"):
            if fence:
                parts.append("<pre><code>" + html.escape("\n".join(code)) + "</code></pre>")
                code.clear()
                fence = False
            else:
                close_list()
                fence = True
            continue
        if fence:
            code.append(line)
            continue
        if not line.strip():
            close_list()
            continue
        if line.startswith("- "):
            list_items.append(inline(line[2:]))
            continue
        close_list()
        parts.append(f"<p>{inline(line)}</p>")
    close_list()
    return "\n".join(parts)

article = render(body_src)
Path(sys.argv[2]).write_text(
    f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>{title}</title>
<style>
  body {{ margin: 0; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; background: #0d1117; color: #e6edf3; }}
  h1 {{ font-size: 1.5rem; }}
  a {{ color: #58a6ff; }}
  .row {{ display: flex; flex-wrap: wrap; gap: 1.5rem; padding: 1.5rem; }}
  .frame {{ background: #161b22; border: 1px solid #30363d; padding: 1rem; box-sizing: border-box; }}
  .desktop {{ width: min(100%, 980px); }}
  .narrow {{ width: 360px; }}
  pre {{ overflow: auto; background: #010409; padding: 0.75rem; }}
  ul {{ padding-left: 1.2rem; }}
</style>
</head>
<body>
<div class="row">
  <section class="frame desktop">
    <h1>{title}</h1>
    {article}
  </section>
  <section class="frame narrow">
    <h1>{title}</h1>
    {article}
  </section>
</div>
</body>
</html>
""",
    encoding="utf-8",
)
print(sys.argv[2])
PY
printf 'preview: %s\n' "$out"
