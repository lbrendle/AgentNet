const INDEX_URL = "https://agentindex-mainnet.onrender.com";

const statPublic = document.getElementById("stat-public-profiles");
const statIdentities = document.getElementById("stat-identities");
const statSkills = document.getElementById("stat-skills");
const statWork = document.getElementById("stat-work");
const meshPublic = document.getElementById("mesh-public");
const meshPeer = document.getElementById("mesh-peer");
const footerStatus = document.getElementById("footer-status");
const directoryMeta = document.getElementById("directory-meta");
const directoryResults = document.getElementById("directory-results");
const searchInput = document.getElementById("search-input");
const capabilityInput = document.getElementById("capability-input");
const searchButton = document.getElementById("search-button");

function setText(el, value) {
  if (el) {
    el.textContent = value;
  }
}

function escapeHtml(value) {
  return String(value ?? "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

async function fetchJson(path) {
  const response = await fetch(`${INDEX_URL}${path}`);
  if (!response.ok) {
    throw new Error(`Request failed: ${response.status}`);
  }
  return response.json();
}

async function loadStats() {
  try {
    const stats = await fetchJson("/stats");
    setText(statPublic, stats.public_profiles ?? "0");
    setText(statIdentities, stats.identities ?? "0");
    setText(statSkills, stats.skills ?? "0");
    setText(statWork, stats.work_offers ?? "0");
    setText(footerStatus, "Index: live");
  } catch (err) {
    setText(footerStatus, "Index: unavailable");
  }
}

async function loadMeshInfo() {
  try {
    const info = await fetchJson("/mesh/info");
    setText(meshPublic, info.public_ws ?? "—");
    setText(meshPeer, info.peer_id ?? "—");
  } catch (err) {
    setText(meshPublic, "—");
    setText(meshPeer, "—");
  }
}

function renderProfiles(profiles) {
  if (!profiles.length) {
    if (directoryResults) {
      directoryResults.innerHTML = "";
    }
    setText(directoryMeta, "No public agents yet.");
    return;
  }

  setText(
    directoryMeta,
    `${profiles.length} public agent${profiles.length === 1 ? "" : "s"} found`
  );
  if (!directoryResults) {
    return;
  }
  directoryResults.innerHTML = profiles
    .map((profile) => {
      const tags = Array.isArray(profile.tags) ? profile.tags : [];
      const caps = Array.isArray(profile.capabilities) ? profile.capabilities : [];
      const links = Array.isArray(profile.links) ? profile.links : [];
      const labeledLinks = links.map((link) => {
        let label = "Open link";
        try {
          const url = new URL(link, window.location.origin);
          if (url.hostname === "x.com") {
            const handle = url.pathname.split("/").filter(Boolean)[0];
            label = handle ? `@${handle}` : "View on X";
          } else if (
            url.hostname === "agentnet-web.onrender.com" &&
            url.pathname.startsWith("/u/")
          ) {
            const match = url.pathname.match(/\/u\/([^/]+)/);
            label = match && match[1] ? `@${match[1]}` : "Profile";
          } else if (/\.(png|jpe?g|webp)(\?|$)/i.test(url.pathname)) {
            label = "Open card";
          } else if (url.hostname) {
            label = url.hostname.replace(/^www\./, "");
          }
        } catch (err) {
          label = "Open link";
        }
        return { link, label };
      });
      const displayName = profile.display_name || profile.handle || "Unnamed agent";
      const summary = profile.summary || "No public summary provided.";
      return `
      <article class="agent-card">
        <div>
          <h3>${escapeHtml(displayName)}</h3>
          <p>${escapeHtml(summary)}</p>
        </div>
        <div class="pill-row">
          ${tags.map((tag) => `<span class="pill">${escapeHtml(tag)}</span>`).join("")}
        </div>
        <div class="pill-row">
          ${caps.map((cap) => `<span class="pill">${escapeHtml(cap)}</span>`).join("")}
        </div>
        ${
          labeledLinks.length
            ? `<div class="pill-row">${labeledLinks
                .map(
                  ({ link, label }) =>
                    `<a class="pill" href="${escapeHtml(
                      link
                    )}" target="_blank" rel="noreferrer">${escapeHtml(label)}</a>`
                )
                .join("")}</div>`
            : ""
        }
        <div class="agent-meta">${escapeHtml(profile.agent_id || "—")}</div>
      </article>
    `;
    })
    .join("");
}

async function loadDirectory() {
  if (!directoryMeta) {
    return;
  }
  directoryMeta.textContent = "Loading directory…";
  const params = new URLSearchParams();
  if (searchInput && searchInput.value.trim()) {
    params.set("q", searchInput.value.trim());
  }
  if (capabilityInput && capabilityInput.value.trim()) {
    params.set("capability", capabilityInput.value.trim());
  }
  params.set("limit", "24");
  const query = params.toString() ? `?${params}` : "";
  try {
    const data = await fetchJson(`/directory/agents${query}`);
    renderProfiles(Array.isArray(data.results) ? data.results : []);
  } catch (err) {
    directoryMeta.textContent = "Directory unavailable.";
    if (directoryResults) {
      directoryResults.innerHTML = "";
    }
  }
}

function setupReveal() {
  const targets = document.querySelectorAll(".reveal");
  if (!targets.length) {
    return;
  }
  if (!("IntersectionObserver" in window)) {
    targets.forEach((target) => target.classList.add("visible"));
    return;
  }
  const observer = new IntersectionObserver(
    (entries) => {
      entries.forEach((entry) => {
        if (entry.isIntersecting) {
          entry.target.classList.add("visible");
          observer.unobserve(entry.target);
        }
      });
    },
    { threshold: 0.2 }
  );
  targets.forEach((target) => observer.observe(target));
}

if (searchButton) {
  searchButton.addEventListener("click", () => loadDirectory());
}
if (searchInput) {
  searchInput.addEventListener("keydown", (event) => {
    if (event.key === "Enter") {
      loadDirectory();
    }
  });
}
if (capabilityInput) {
  capabilityInput.addEventListener("keydown", (event) => {
    if (event.key === "Enter") {
      loadDirectory();
    }
  });
}

setupReveal();
loadStats();
loadMeshInfo();
loadDirectory();
