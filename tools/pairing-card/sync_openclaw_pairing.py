#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import List, Optional, Tuple

from PIL import Image, ImageDraw, ImageFont


def run_json(cmd: List[str]) -> object:
    raw = subprocess.check_output(cmd, text=True)
    decoder = json.JSONDecoder()
    for idx, ch in enumerate(raw):
        if ch not in "[{":
            continue
        try:
            obj, _ = decoder.raw_decode(raw[idx:])
            return obj
        except json.JSONDecodeError:
            continue
    raise SystemExit("[pairing-sync] no JSON found in output")


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


def load_font(candidates: List[str], size: int) -> ImageFont.FreeTypeFont:
    for path in candidates:
        try:
            if Path(path).exists():
                return ImageFont.truetype(path, size)
        except Exception:
            continue
    return ImageFont.load_default()


def render_card(
    handle: str,
    agent_name: str,
    agent_id: str,
    agent_did: str,
    paired_count: int,
    paired_summary: str,
    out_path: Path,
    *,
    mode: str = "pairing",
    paired_handle: Optional[str] = None,
) -> None:
    W, H = 1200, 630
    bg = (236, 240, 245)
    base = Image.new("RGB", (W, H), bg)

    shadow = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    sdraw = ImageDraw.Draw(shadow)
    card_box = (60, 70, 1140, 560)
    try:
        sdraw.rounded_rectangle(
            (card_box[0] + 8, card_box[1] + 12, card_box[2] + 8, card_box[3] + 12),
            radius=30,
            fill=(0, 0, 0, 30),
        )
    except Exception:
        sdraw.rectangle(
            (card_box[0] + 8, card_box[1] + 12, card_box[2] + 8, card_box[3] + 12),
            fill=(0, 0, 0, 30),
        )

    base = Image.alpha_composite(base.convert("RGBA"), shadow)
    draw = ImageDraw.Draw(base)
    try:
        draw.rounded_rectangle(card_box, radius=30, fill=(255, 255, 255), outline=(217, 223, 232), width=2)
    except Exception:
        draw.rectangle(card_box, fill=(255, 255, 255), outline=(217, 223, 232), width=2)

    font_title = load_font(
        [
            "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
            "/System/Library/Fonts/HelveticaNeue.ttc",
            "/System/Library/Fonts/Helvetica.ttc",
        ],
        42,
    )
    font_handle = load_font(
        [
            "/System/Library/Fonts/Supplemental/Arial.ttf",
            "/System/Library/Fonts/HelveticaNeue.ttc",
            "/System/Library/Fonts/Helvetica.ttc",
        ],
        28,
    )
    font_body = load_font(
        [
            "/System/Library/Fonts/Supplemental/Arial.ttf",
            "/System/Library/Fonts/HelveticaNeue.ttc",
            "/System/Library/Fonts/Helvetica.ttc",
        ],
        28,
    )
    font_label = load_font(
        [
            "/System/Library/Fonts/Supplemental/Arial.ttf",
            "/System/Library/Fonts/HelveticaNeue.ttc",
            "/System/Library/Fonts/Helvetica.ttc",
        ],
        22,
    )
    font_mono = load_font(
        [
            "/System/Library/Fonts/SFNSMono.ttf",
            "/System/Library/Fonts/Supplemental/Andale Mono.ttf",
        ],
        22,
    )

    accent = (29, 155, 240)
    text_main = (17, 23, 32)
    text_sub = (92, 104, 120)

    avatar_r = 38
    avatar_x = card_box[0] + 48
    avatar_y = card_box[1] + 48
    draw.ellipse(
        (avatar_x - avatar_r, avatar_y - avatar_r, avatar_x + avatar_r, avatar_y + avatar_r),
        fill=accent,
    )
    initial = (handle[:1] or "A").upper()
    ibox = draw.textbbox((0, 0), initial, font=font_title)
    iw = ibox[2] - ibox[0]
    ih = ibox[3] - ibox[1]
    draw.text((avatar_x - iw / 2, avatar_y - ih / 2 - 2), initial, fill=(255, 255, 255), font=font_title)

    name_x = avatar_x + avatar_r + 24
    name_y = card_box[1] + 26
    draw.text((name_x, name_y), f"@{handle}", fill=text_main, font=font_title)
    draw.text((name_x, name_y + 46), "AgentNet Mainnet", fill=text_sub, font=font_handle)

    header = "PAIRING CARD" if mode == "pairing" else "AGENT CARD"
    header_w = draw.textlength(header, font=font_label)
    draw.text((card_box[2] - 48 - header_w, card_box[1] + 32), header, fill=accent, font=font_label)

    if mode == "pairing":
        headline = f"Paired with OpenClaw agent {agent_name} (id: {agent_id})."
    else:
        headline = f"OpenClaw agent {agent_name} (id: {agent_id})."

    def wrap_text(text: str, max_width: int) -> List[str]:
        words = text.split()
        lines: List[str] = []
        current = ""
        for word in words:
            candidate = f"{current} {word}".strip()
            if not current or draw.textlength(candidate, font=font_body) <= max_width:
                current = candidate
            else:
                lines.append(current)
                current = word
        if current:
            lines.append(current)
        return lines

    body_x = card_box[0] + 48
    body_y = card_box[1] + 140
    for line in wrap_text(headline, card_box[2] - card_box[0] - 96):
        draw.text((body_x, body_y), line, fill=text_main, font=font_body)
        body_y += 38

    profile_url = f"agentnet-web.onrender.com/u/{handle}"
    did_display = agent_did
    if len(did_display) > 46:
        did_display = did_display[:28] + "…" + did_display[-12:]

    meta_y = max(body_y + 16, card_box[1] + 240)
    meta = []
    if mode == "agent" and paired_handle:
        meta.append(("Paired human", f"@{paired_handle}"))
    meta.append(("Agent DID", did_display))
    if mode != "agent":
        meta.append(("Paired devices", f"{paired_count} ({paired_summary})"))
    meta.append(("Profile", profile_url))

    label_width = max(draw.textlength(label, font=font_label) for label, _ in meta)
    for label, value in meta:
        draw.text((body_x, meta_y), label, fill=text_sub, font=font_label)
        draw.text((body_x + label_width + 18, meta_y), value, fill=text_main, font=font_mono)
        meta_y += 34

    footer = "agentnet.io"
    fw = draw.textlength(footer, font=font_label)
    draw.text((card_box[2] - 48 - fw, card_box[3] - 44), footer, fill=text_sub, font=font_label)

    out_path.parent.mkdir(parents=True, exist_ok=True)
    base.convert("RGB").save(out_path, "PNG")


def render_html(
    handle: str,
    agent_name: str,
    agent_id: str,
    agent_did: str,
    paired_count: int,
    paired_summary: str,
    agent_handle: Optional[str] = None,
) -> str:
    agent_link = f"/u/{agent_handle}/" if agent_handle else ""
    agent_link_html = ""
    if agent_link:
        agent_link_html = f"""
            <span class="dot">•</span>
            <a href="{agent_link}">Agent card</a>"""
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

      .card-caption {{
        display: flex;
        flex-wrap: wrap;
        align-items: center;
        gap: 8px;
        margin-top: 12px;
        font-size: 0.95rem;
        color: var(--muted);
      }}

      .card-caption .handle {{
        color: var(--text);
        font-weight: 600;
      }}

      .card-caption .dot {{
        color: var(--muted);
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
          <p class=\"eyebrow\">Pairing Card</p>
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
          <div class=\"card-caption\">
            <span class=\"handle\">@{handle}</span>
            <span class=\"dot\">•</span>
            <a href=\"https://x.com/{handle}\" target=\"_blank\" rel=\"noreferrer\">View on X</a>{agent_link_html}
          </div>
        </div>
        <div class=\"card-actions\">
          <a class=\"button primary\" href=\"https://x.com/{handle}\" target=\"_blank\" rel=\"noreferrer\">Visit on X</a>
          {f'<a class="button ghost" href="{agent_link}">View Agent Card</a>' if agent_link else ''}
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
    parser = argparse.ArgumentParser(description="Sync pairing card from OpenClaw state.")
    parser.add_argument("--handle", required=True, help="Public handle without @")
    parser.add_argument("--mode", choices=["pairing", "agent"], default="pairing")
    parser.add_argument("--paired-handle", default=None, help="Optional paired human handle (for agent cards)")
    parser.add_argument("--agent-handle", default=None, help="Optional paired agent handle (for pairing cards)")
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

    render_card(
        args.handle,
        agent_name,
        agent_id,
        agent_did,
        paired_count,
        paired_summary,
        card_path,
        mode=args.mode,
        paired_handle=args.paired_handle,
    )
    html_path.write_text(
        render_html(
            args.handle,
            agent_name,
            agent_id,
            agent_did,
            paired_count,
            paired_summary,
            agent_handle=args.agent_handle,
        )
        + "\n"
    )

    changed = git_has_changes(repo_root, [card_path, html_path])
    if changed and args.publish:
        git_commit_push(repo_root, f"Sync pairing card for @{args.handle}", [card_path, html_path])

    if args.deploy:
        deploy_render_service(args.render_key, "agentnet-web")

    print(f"[pairing-sync] updated {html_path} and {card_path}")


if __name__ == "__main__":
    main()
