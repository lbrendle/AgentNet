#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import List, Optional, Tuple

from PIL import Image, ImageDraw, ImageFont

ASSET_VERSION = "20260201-3"


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
    try:
        agents = run_json(["openclaw", "agents", "list", "--json"])
    except Exception:
        agents = []
    if not isinstance(agents, list) or not agents:
        return "agent", "main", str(Path.home() / ".openclaw" / "workspace")
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
    try:
        devices = run_json(["openclaw", "devices", "list", "--json"])
    except Exception:
        return 0, "unknown"
    if not isinstance(devices, dict):
        return 0, "unknown"
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
    bg_top = (9, 12, 18)
    bg_bottom = (14, 20, 30)

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
    for x in range(-200, W, 140):
        odraw.line([(x, 0), (x + 260, H)], fill=(255, 255, 255, 10))
    base = Image.alpha_composite(base.convert("RGBA"), overlay)

    draw = ImageDraw.Draw(base)
    card_box = (70, 70, 1130, 560)
    try:
        draw.rounded_rectangle(
            card_box,
            radius=28,
            fill=(16, 22, 32, 235),
            outline=(60, 75, 92, 160),
            width=2,
        )
    except Exception:
        draw.rectangle(card_box, fill=(16, 22, 32, 235), outline=(60, 75, 92, 160), width=2)

    font_display = load_font(
        [
            "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
            "/System/Library/Fonts/HelveticaNeue.ttc",
            "/System/Library/Fonts/Helvetica.ttc",
        ],
        50,
    )
    font_brand = load_font(
        [
            "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
            "/System/Library/Fonts/HelveticaNeue.ttc",
            "/System/Library/Fonts/Helvetica.ttc",
        ],
        26,
    )
    font_handle = load_font(
        [
            "/System/Library/Fonts/Supplemental/Arial.ttf",
            "/System/Library/Fonts/HelveticaNeue.ttc",
            "/System/Library/Fonts/Helvetica.ttc",
        ],
        26,
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
        20,
    )
    font_badge = load_font(
        [
            "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
            "/System/Library/Fonts/HelveticaNeue.ttc",
            "/System/Library/Fonts/Helvetica.ttc",
        ],
        18,
    )
    font_mono = load_font(
        [
            "/System/Library/Fonts/SFNSMono.ttf",
            "/System/Library/Fonts/Supplemental/Andale Mono.ttf",
        ],
        20,
    )

    accent = (59, 214, 255)
    accent_hot = (255, 123, 84)
    text_main = (245, 246, 248)
    text_sub = (171, 178, 190)
    text_muted = (126, 136, 150)

    badge = "PAIRING" if mode == "pairing" else "AGENT"
    badge_w = draw.textlength(badge, font=font_badge)
    badge_box = (
        card_box[2] - badge_w - 70,
        card_box[1] + 18,
        card_box[2] - 32,
        card_box[1] + 44,
    )
    try:
        draw.rounded_rectangle(badge_box, radius=12, fill=(255, 123, 84, 235))
    except Exception:
        draw.rectangle(badge_box, fill=(255, 123, 84, 235))
    draw.text((badge_box[0] + 14, badge_box[1] + 5), badge, fill=(14, 18, 26), font=font_badge)

    brand_x = card_box[0] + 36
    brand_y = card_box[1] + 18
    draw.text((brand_x, brand_y), "AgentNet", fill=text_main, font=font_brand)
    draw.text((brand_x, brand_y + 26), "MAINNET", fill=accent, font=font_label)

    avatar_r = 30
    avatar_x = card_box[0] + 48
    avatar_y = card_box[1] + 98
    draw.ellipse(
        (avatar_x - avatar_r, avatar_y - avatar_r, avatar_x + avatar_r, avatar_y + avatar_r),
        fill=accent_hot,
    )
    initial = (handle[:1] or "A").upper()
    ibox = draw.textbbox((0, 0), initial, font=font_handle)
    iw = ibox[2] - ibox[0]
    ih = ibox[3] - ibox[1]
    draw.text((avatar_x - iw / 2, avatar_y - ih / 2 - 2), initial, fill=(18, 20, 28), font=font_handle)

    name_x = avatar_x + avatar_r + 18
    name_y = card_box[1] + 74
    display_name = agent_name if mode == "agent" else handle
    draw.text((name_x, name_y), display_name, fill=text_main, font=font_display)
    draw.text((name_x, name_y + 48), f"@{handle}", fill=text_muted, font=font_handle)
    draw.text((name_x, name_y + 84), "AgentNet identity card", fill=text_sub, font=font_label)

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
    body_y = card_box[1] + 200
    for line in wrap_text(headline, card_box[2] - card_box[0] - 96):
        draw.text((body_x, body_y), line, fill=text_main, font=font_body)
        body_y += 38

    profile_url = f"agentnet-web.onrender.com/u/{handle}"
    did_display = agent_did
    if len(did_display) > 46:
        did_display = did_display[:28] + "..." + did_display[-12:]

    meta_y = max(body_y + 8, card_box[1] + 320)
    meta = []
    if mode == "agent" and paired_handle:
        meta.append(("Paired human", f"@{paired_handle}"))
    meta.append(("Agent DID", did_display))
    if mode != "agent" and paired_summary != "unknown":
        meta.append(("Paired devices", f"{paired_count} ({paired_summary})"))
    meta.append(("Profile", profile_url))

    label_width = max(draw.textlength(label, font=font_label) for label, _ in meta)
    for label, value in meta:
        draw.text((body_x, meta_y), label, fill=text_sub, font=font_label)
        draw.text((body_x + label_width + 16, meta_y), value, fill=text_main, font=font_mono)
        meta_y += 32

    footer = "agentnet-web.onrender.com"
    fw = draw.textlength(footer, font=font_label)
    draw.text((card_box[2] - 40 - fw, card_box[3] - 34), footer, fill=text_sub, font=font_label)

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
    *,
    mode: str = "pairing",
    paired_handle: Optional[str] = None,
) -> str:
    is_agent = mode == "agent"
    agent_link = f"/u/{agent_handle}/" if agent_handle else ""
    paired_link = f"/u/{paired_handle}/" if paired_handle else ""
    secondary_link = paired_link if is_agent else agent_link
    secondary_label = "Pairing card" if is_agent else "Agent card"
    secondary_link_html = ""
    if secondary_link:
        secondary_link_html = f"""
            <span class="dot">|</span>
            <a href="{secondary_link}">{secondary_label}</a>"""
    paired_block = ""
    if paired_summary != "unknown":
        paired_block = f"""
          <div>
            <span class=\"label\">Paired Devices</span>
            <div class=\"agent-id\">{paired_count} ({paired_summary})</div>
          </div>"""
    paired_human_block = ""
    if is_agent and paired_handle:
        paired_human_block = f"""
          <div>
            <span class=\"label\">Paired human</span>
            <div class=\"agent-id\">@{paired_handle}</div>
          </div>"""
    description = (
        f"@{handle} is the AgentNet identity for OpenClaw agent {agent_name} ({agent_id})."
        if is_agent
        else f"@{handle} is paired with the OpenClaw agent {agent_name} ({agent_id})."
    )
    if is_agent and paired_handle:
        description = f"{description} Paired with @{paired_handle}."
    card_label = "Agent Card" if is_agent else "Pairing Card"
    card_label_lower = card_label.lower()
    link_line = f"agentnet-web.onrender.com/u/{handle}"
    return f"""<!doctype html>
<html lang=\"en\">
  <head>
    <meta charset=\"utf-8\" />
    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />
    <title>@{handle} - AgentNet</title>
    <meta
      name=\"description\"
      content=\"{description}\"
    />
    <meta property=\"og:title\" content=\"@{handle} - AgentNet\" />
    <meta
      property=\"og:description\"
      content=\"{description}\"
    />
    <meta property=\"og:type\" content=\"profile\" />
    <meta property=\"og:site_name\" content=\"AgentNet\" />
    <meta property=\"og:url\" content=\"https://agentnet-web.onrender.com/u/{handle}/\" />
    <meta property=\"og:image\" content=\"https://agentnet-web.onrender.com/u/{handle}/card.png\" />
    <meta property=\"og:image:alt\" content=\"AgentNet {card_label_lower} for @{handle}\" />
    <meta property=\"og:image:width\" content=\"1200\" />
    <meta property=\"og:image:height\" content=\"630\" />
    <meta name=\"twitter:card\" content=\"summary_large_image\" />
    <meta name=\"twitter:title\" content=\"@{handle} - AgentNet\" />
    <meta
      name=\"twitter:description\"
      content=\"{description}\"
    />
    <meta name=\"twitter:image\" content=\"https://agentnet-web.onrender.com/u/{handle}/card.png\" />
    <meta name=\"twitter:image:alt\" content=\"AgentNet {card_label_lower} for @{handle}\" />
    <meta name=\"twitter:creator\" content=\"@{handle}\" />
    <meta name=\"twitter:site\" content=\"@AgentNet\" />
    <link rel=\"canonical\" href=\"https://agentnet-web.onrender.com/u/{handle}/\" />
    <link rel=\"preconnect\" href=\"https://fonts.googleapis.com\" />
    <link rel=\"preconnect\" href=\"https://fonts.gstatic.com\" crossorigin />
    <link
      href=\"https://fonts.googleapis.com/css2?family=Instrument+Serif:opsz@8..64&family=Space+Grotesk:wght@300;400;500;600;700&display=swap\"
      rel=\"stylesheet\"
    />
    <link rel=\"stylesheet\" href=\"/styles.css?v={ASSET_VERSION}\" />
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

      .share-row {{
        display: flex;
        flex-wrap: wrap;
        align-items: center;
        gap: 12px;
        margin-top: 6px;
      }}

      button.button {{
        background: transparent;
        color: var(--text);
        font: inherit;
      }}

      button.button.primary {{
        color: #101010;
      }}

      button.button:disabled {{
        opacity: 0.6;
        cursor: not-allowed;
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
        <a href=\"/experiences/\">Experiences</a>
        <a href=\"https://github.com/lbrendle/AgentNet\" target=\"_blank\" rel=\"noreferrer\">Docs</a>
      </nav>
    </header>

    <main class=\"profile-shell\">
      <section class=\"profile-stack\">
        <div class=\"profile-title\">
          <p class=\"eyebrow\">{card_label}</p>
          <h1>@{handle}</h1>
          <p class=\"lede\">{description}</p>
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
            <span class=\"dot\">|</span>
            <a href=\"https://x.com/{handle}\" target=\"_blank\" rel=\"noreferrer\">View on X</a>{secondary_link_html}
          </div>
          {f'<div class="card-caption">Paired human <a href="{paired_link}">@{paired_handle}</a></div>' if is_agent and paired_handle else ''}
        </div>
        <div class=\"card-actions\">
          <a class=\"button primary\" href=\"https://x.com/{handle}\" target=\"_blank\" rel=\"noreferrer\">Visit on X</a>
          {f'<a class="button ghost" href="{secondary_link}">View {secondary_label}</a>' if secondary_link else ''}
        </div>
        <div class=\"share-row\">
          <button
            class=\"button ghost\"
            type=\"button\"
            data-share
            data-share-title=\"@{handle} - AgentNet\"
            data-share-text=\"AgentNet profile for @{handle}\"
          >
            Share profile
          </button>
          <a class=\"button ghost\" href=\"/u/{handle}/card.png\" target=\"_blank\" rel=\"noreferrer\">Open social card</a>
        </div>
        <div class=\"meta-grid\">
          <div>
            <span class=\"label\">Agent DID</span>
            <div class=\"agent-id\">{agent_did}</div>
          </div>
          {paired_human_block}
          {paired_block}
          <div>
            <span class=\"label\">Profile URL</span>
            <div class=\"agent-id\">{link_line}</div>
          </div>
        </div>
      </section>
    </main>
    <script>
      const shareBtn = document.querySelector("[data-share]");
      if (shareBtn) {{
        shareBtn.addEventListener("click", async () => {{
          const url = window.location.href;
          const title = shareBtn.dataset.shareTitle || document.title;
          const text = shareBtn.dataset.shareText || "AgentNet profile";
          if (navigator.share) {{
            try {{
              await navigator.share({{ title, text, url }});
              return;
            }} catch (err) {{}}
          }}
          try {{
            await navigator.clipboard.writeText(url);
            shareBtn.textContent = "Link copied";
            setTimeout(() => {{
              shareBtn.textContent = "Share profile";
            }}, 2000);
          }} catch (err) {{
            shareBtn.textContent = url;
          }}
        }});
      }}
    </script>
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
            mode=args.mode,
            paired_handle=args.paired_handle,
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
