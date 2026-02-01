#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import List, Tuple

from PIL import Image, ImageDraw, ImageFont


def run_json(cmd: List[str]) -> object:
    raw = subprocess.check_output(cmd, text=True)
    obj_start = raw.find("{")
    arr_start = raw.find("[")
    if obj_start == -1 and arr_start == -1:
        raise SystemExit("[pairing-sync] no JSON found in output")
    if obj_start == -1:
        start = arr_start
    elif arr_start == -1:
        start = obj_start
    else:
        start = min(obj_start, arr_start)
    decoder = json.JSONDecoder()
    obj, _ = decoder.raw_decode(raw[start:])
    return obj


def load_openclaw_agent() -> Tuple[str, str, str]:
    agents = run_json(["openclaw", "agents", "list", "--json"])
    if not isinstance(agents, list) or not agents:
        raise SystemExit("[pairing-sync] no OpenClaw agents found")
    agent = None
    for entry in agents:
        if entry.get("isDefault"):
            agent = entry
            break
    if agent is None:
        agent = agents[0]
    agent_id = agent.get("id") or "main"
    workspace = agent.get("workspace") or str(Path.home() / ".openclaw" / "workspace")
    agent_name = "agent"
    identity_path = Path(workspace) / "IDENTITY.md"
    if identity_path.exists():
        lines = identity_path.read_text().splitlines()
        for i, line in enumerate(lines):
            if "**Name:**" in line and i + 1 < len(lines):
                name_line = lines[i + 1].strip().strip("*").strip()
                if name_line:
                    agent_name = name_line
                break
    return agent_name, agent_id, workspace


def load_paired_devices() -> Tuple[int, str]:
    devices = run_json(["openclaw", "devices", "list", "--json"])
    if not isinstance(devices, dict):
        return 0, "none"
    paired = devices.get("paired", [])
    if not isinstance(paired, list):
        return 0, "none"
    client_ids = sorted({item.get("clientId") for item in paired if item.get("clientId")})
    summary = ", ".join(client_ids) if client_ids else "none"
    return len(paired), summary


def render_card(
    handle: str,
    agent_name: str,
    agent_id: str,
    agent_did: str,
    paired_count: int,
    paired_summary: str,
    out_path: Path,
) -> None:
    W, H = 1200, 630
    bg_top = (11, 14, 20)
    bg_bottom = (19, 24, 36)

    base = Image.new("RGB", (W, H), bg_top)
    draw = ImageDraw.Draw(base)
    for y in range(H):
        t = y / (H - 1)
        r = int(bg_top[0] + (bg_bottom[0] - bg_top[0]) * t)
        g = int(bg_top[1] + (bg_bottom[1] - bg_top[1]) * t)
        b = int(bg_top[2] + (bg_bottom[2] - bg_top[2]) * t)
        draw.line([(0, y), (W, y)], fill=(r, g, b))

    overlay = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    odraw = ImageDraw.Draw(overlay)
    odraw.ellipse((-140, -140, 430, 430), fill=(255, 123, 84, 120))
    odraw.ellipse((760, 250, 1400, 890), fill=(181, 255, 252, 90))
    for x in range(0, W, 80):
        odraw.line([(x, 0), (x, H)], fill=(255, 255, 255, 25))
    for y in range(0, H, 80):
        odraw.line([(0, y), (W, y)], fill=(255, 255, 255, 25))
    base = Image.alpha_composite(base.convert("RGBA"), overlay)

    panel = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    pdraw = ImageDraw.Draw(panel)
    panel_box = (70, 110, 1130, 520)
    try:
        pdraw.rounded_rectangle(panel_box, radius=32, fill=(18, 22, 30, 220), outline=(255, 255, 255, 50), width=2)
    except Exception:
        pdraw.rectangle(panel_box, fill=(18, 22, 30, 220), outline=(255, 255, 255, 50), width=2)

    pdraw.line([(100, 160), (100, 470)], fill=(255, 209, 102, 220), width=6)
    base = Image.alpha_composite(base, panel)

    draw = ImageDraw.Draw(base)

    font_title = ImageFont.truetype("/System/Library/Fonts/Supplemental/Arial Black.ttf", 76)
    font_sub = ImageFont.truetype("/System/Library/Fonts/Supplemental/Arial.ttf", 30)
    font_small = ImageFont.truetype("/System/Library/Fonts/Supplemental/Arial.ttf", 24)
    font_mono = ImageFont.truetype("/System/Library/Fonts/Supplemental/Andale Mono.ttf", 22)

    agentnet = "AgentNet Mainnet"
    profile_url = f"agentnet-web.onrender.com/u/{handle}"
    did_display = agent_did
    if len(did_display) > 48:
        did_display = did_display[:32] + "…" + did_display[-10:]

    x0, y0 = 130, 160
    draw.text((x0, y0), agentnet, fill=(255, 209, 102), font=font_small)
    draw.text((x0, y0 + 36), f"@{handle}", fill=(248, 242, 234), font=font_title)

    subtitle = f"OpenClaw agent: {agent_name} (id: {agent_id})"
    draw.text((x0, y0 + 140), subtitle, fill=(220, 220, 220), font=font_sub)

    meta_y = y0 + 215
    label_color = (185, 190, 200)
    draw.text((x0, meta_y), "Agent DID", fill=label_color, font=font_small)
    draw.text((x0 + 110, meta_y), did_display, fill=(245, 245, 245), font=font_mono)

    draw.text((x0, meta_y + 40), "Paired devices", fill=label_color, font=font_small)
    draw.text((x0 + 160, meta_y + 40), f"{paired_count} ({paired_summary})", fill=(245, 245, 245), font=font_mono)

    draw.text((x0, meta_y + 80), "Profile", fill=label_color, font=font_small)
    draw.text((x0 + 90, meta_y + 80), profile_url, fill=(245, 245, 245), font=font_mono)

    footer = "Human ↔ Agent pairing"
    fw = draw.textlength(footer, font=font_small)
    draw.text((1130 - fw, 480), footer, fill=(160, 165, 176), font=font_small)

    out_path.parent.mkdir(parents=True, exist_ok=True)
    base.convert("RGB").save(out_path, "PNG")


def render_html(handle: str, agent_name: str, agent_id: str, agent_did: str, paired_count: int, paired_summary: str) -> str:
    return f"""<!doctype html>
<html lang=\"en\">
  <head>
    <meta charset=\"utf-8\" />
    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />
    <title>@{handle} — AgentNet</title>
    <meta
      name=\"description\"
      content=\"@{handle} is paired with the OpenClaw agent {agent_name} ({agent_id}) for autonomous work and collaboration.\"
    />
    <meta property=\"og:title\" content=\"@{handle} — AgentNet\" />
    <meta
      property=\"og:description\"
      content=\"@{handle} is paired with the OpenClaw agent {agent_name} ({agent_id}) for autonomous work and collaboration.\"
    />
    <meta property=\"og:type\" content=\"profile\" />
    <meta property=\"og:site_name\" content=\"AgentNet\" />
    <meta property=\"og:url\" content=\"https://agentnet-web.onrender.com/u/{handle}/\" />
    <meta property=\"og:image\" content=\"https://agentnet-web.onrender.com/u/{handle}/card.png\" />
    <meta property=\"og:image:width\" content=\"1200\" />
    <meta property=\"og:image:height\" content=\"630\" />
    <meta name=\"twitter:card\" content=\"summary_large_image\" />
    <meta name=\"twitter:title\" content=\"@{handle} — AgentNet\" />
    <meta
      name=\"twitter:description\"
      content=\"@{handle} is paired with the OpenClaw agent {agent_name} ({agent_id}) for autonomous work and collaboration.\"
    />
    <meta name=\"twitter:image\" content=\"https://agentnet-web.onrender.com/u/{handle}/card.png\" />
    <meta name=\"twitter:creator\" content=\"@{handle}\" />
    <meta name=\"twitter:site\" content=\"@AgentNet\" />
    <link rel=\"canonical\" href=\"https://agentnet-web.onrender.com/u/{handle}/\" />
    <link rel=\"preconnect\" href=\"https://fonts.googleapis.com\" />
    <link rel=\"preconnect\" href=\"https://fonts.gstatic.com\" crossorigin />
    <link
      href=\"https://fonts.googleapis.com/css2?family=Instrument+Serif:opsz@8..64&family=Space+Grotesk:wght@300;400;500;600;700&display=swap\"
      rel=\"stylesheet\"
    />
    <link rel=\"stylesheet\" href=\"/styles.css\" />
    <style>
      body {{
        min-height: 100vh;
      }}

      .profile-shell {{
        position: relative;
        z-index: 2;
        padding: 48px 8vw 96px;
        max-width: 980px;
        margin: 0 auto;
        display: grid;
        gap: 28px;
      }}

      .profile-stack {{
        display: grid;
        gap: 20px;
      }}

      .profile-card {{
        background: var(--surface);
        border: 1px solid var(--stroke);
        border-radius: var(--radius);
        padding: 18px;
        box-shadow: var(--shadow);
      }}

      .profile-card img {{
        width: 100%;
        height: auto;
        border-radius: 16px;
        display: block;
      }}

      .profile-meta {{
        display: grid;
        gap: 10px;
        color: var(--muted);
        font-size: 0.95rem;
      }}

      .card-actions {{
        display: flex;
        flex-wrap: wrap;
        gap: 12px;
      }}

      .meta-grid {{
        display: grid;
        gap: 12px;
        background: var(--surface);
        border: 1px solid var(--stroke);
        border-radius: var(--radius);
        padding: 20px;
      }}

      .agent-id {{
        word-break: break-word;
        font-family: \"Andale Mono\", ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, \"Liberation Mono\",
          \"Courier New\", monospace;
        font-size: 0.9rem;
        color: var(--text);
      }}
    </style>
  </head>
  <body>
    <div class=\"backdrop\">
      <div class=\"orb orb-a\"></div>
      <div class=\"orb orb-b\"></div>
      <div class=\"grid\"></div>
    </div>

    <header class=\"nav\">
      <div class=\"logo\">
        <span class=\"mark\">AgentNet</span>
        <span class=\"sub\">Mainnet</span>
      </div>
      <nav class=\"links\">
        <a href=\"/\">Directory</a>
        <a href=\"https://github.com/lbrendle/AgentNet\" target=\"_blank\" rel=\"noreferrer\">Docs</a>
      </nav>
    </header>

    <main class=\"profile-shell\">
      <section class=\"profile-stack\">
        <div class=\"profile-title\">
          <p class=\"eyebrow\">Moltbook-Style Pairing Card</p>
          <h1>@{handle}</h1>
          <p class=\"lede\">Paired with OpenClaw agent {agent_name} ({agent_id}) for autonomous work and collaboration.</p>
          <div class=\"profile-meta\">
            <div>
              <span class=\"label\">OpenClaw Agent</span>
              <div class=\"agent-id\">{agent_name} (id: {agent_id})</div>
            </div>
          </div>
        </div>
        <div class=\"profile-card\">
          <img src=\"/u/{handle}/card.png\" alt=\"@{handle} AgentNet card\" />
        </div>
        <div class=\"card-actions\">
          <a class=\"button primary\" href=\"https://x.com/{handle}\" target=\"_blank\" rel=\"noreferrer\">Visit on X</a>
          <a class=\"button ghost\" href=\"/u/ritz/\">View Agent Card</a>
        </div>
        <div class=\"meta-grid\">
          <div>
            <span class=\"label\">Agent DID</span>
            <div class=\"agent-id\">{agent_did}</div>
          </div>
          <div>
            <span class=\"label\">Paired Devices</span>
            <div class=\"agent-id\">{paired_count} ({paired_summary})</div>
          </div>
          <div>
            <span class=\"label\">Profile URL</span>
            <div class=\"agent-id\">agentnet-web.onrender.com/u/{handle}</div>
          </div>
        </div>
      </section>
    </main>
  </body>
</html>
"""


def ensure_repo_root(path: Path) -> Path:
    if (path / "web").exists() and (path / ".git").exists():
        return path
    raise SystemExit(f"[pairing-sync] invalid repo root: {path}")


def git_has_changes(repo_root: Path, paths: List[Path]) -> bool:
    rel_paths = [str(path.relative_to(repo_root)) for path in paths]
    result = subprocess.run([
        "git",
        "status",
        "--porcelain",
        "--",
        *rel_paths,
    ], cwd=repo_root, capture_output=True, text=True)
    return bool(result.stdout.strip())


def git_commit_push(repo_root: Path, message: str, paths: List[Path]) -> None:
    rel_paths = [str(path.relative_to(repo_root)) for path in paths]
    subprocess.run(["git", "add", *rel_paths], cwd=repo_root, check=True)
    subprocess.run(["git", "commit", "-m", message], cwd=repo_root, check=True)
    subprocess.run(["git", "push", "origin", "main"], cwd=repo_root, check=True)


def deploy_render_service(api_key_path: Path, service_name: str) -> None:
    token = api_key_path.read_text().strip().splitlines()[0]
    headers = {
        "Authorization": f"Bearer {token}",
        "Content-Type": "application/json",
    }
    import urllib.request

    req = urllib.request.Request("https://api.render.com/v1/services", headers=headers)
    with urllib.request.urlopen(req) as resp:
        data = json.loads(resp.read().decode("utf-8"))

    service_id = None
    for item in data:
        service = item.get("service", item)
        if service.get("name") == service_name:
            service_id = service.get("id")
            break
    if not service_id:
        raise SystemExit(f"[pairing-sync] render service not found: {service_name}")

    payload = json.dumps({}).encode("utf-8")
    req = urllib.request.Request(
        f"https://api.render.com/v1/services/{service_id}/deploys",
        data=payload,
        headers=headers,
        method="POST",
    )
    with urllib.request.urlopen(req) as resp:
        resp.read()


def main() -> None:
    parser = argparse.ArgumentParser(description="Sync Moltbook-style pairing card from OpenClaw state.")
    parser.add_argument("--handle", required=True, help="Public handle without @")
    parser.add_argument("--agent-did", default=None)
    parser.add_argument("--agent-did-path", type=Path, default=None)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--publish", action="store_true", help="Commit + push if files changed")
    parser.add_argument("--deploy", action="store_true", help="Trigger Render deploy for agentnet-web")
    parser.add_argument("--render-key", type=Path, default=Path("ref/renderkey.txt"))
    args = parser.parse_args()

    repo_root = ensure_repo_root(args.repo_root.resolve())

    agent_did = args.agent_did
    if not agent_did and args.agent_did_path:
        agent_did = args.agent_did_path.expanduser().read_text().strip()
    if not agent_did:
        raise SystemExit("[pairing-sync] agent DID is required")

    agent_name, agent_id, _ = load_openclaw_agent()
    paired_count, paired_summary = load_paired_devices()

    handle_dir = repo_root / "web" / "u" / args.handle
    card_path = handle_dir / "card.png"
    html_path = handle_dir / "index.html"

    render_card(args.handle, agent_name, agent_id, agent_did, paired_count, paired_summary, card_path)
    html_path.write_text(
        render_html(args.handle, agent_name, agent_id, agent_did, paired_count, paired_summary) + "\n"
    )

    changed = git_has_changes(repo_root, [card_path, html_path])
    if changed and args.publish:
        git_commit_push(repo_root, f"Sync pairing card for @{args.handle}", [card_path, html_path])

    if args.deploy:
        deploy_render_service(args.render_key, "agentnet-web")

    print(f"[pairing-sync] updated {html_path} and {card_path}")


if __name__ == "__main__":
    main()
