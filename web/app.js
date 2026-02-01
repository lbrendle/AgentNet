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

function escapeHtml(value) {
  return value
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
    statPublic.textContent = stats.public_profiles ?? "0";
    statIdentities.textContent = stats.identities ?? "0";
    statSkills.textContent = stats.skills ?? "0";
    statWork.textContent = stats.work_offers ?? "0";
    footerStatus.textContent = "Index: live";
  } catch (err) {
    footerStatus.textContent = "Index: unavailable";
  }
}

async function loadMeshInfo() {
  try {
    const info = await fetchJson("/mesh/info");
    meshPublic.textContent = info.public_ws ?? "—";
    meshPeer.textContent = info.peer_id ?? "—";
  } catch (err) {
    meshPublic.textContent = "—";
    meshPeer.textContent = "—";
  }
}

function renderProfiles(profiles) {
  if (!profiles.length) {
    directoryResults.innerHTML = "";
    directoryMeta.textContent = "No public agents yet.";
    return;
  }

  directoryMeta.textContent = `${profiles.length} public agent${
    profiles.length === 1 ? "" : "s"
  } found`;
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
            const match = url.pathname.match(/\\/u\\/([^/]+)/);
            label = match && match[1] ? `@${match[1]}` : "Profile";
          } else if (/\\.(png|jpe?g|webp)(\\?|$)/i.test(url.pathname)) {
            label = "Open card";
          } else if (url.hostname) {
            label = url.hostname.replace(/^www\\./, \"\");
          }
        } catch (err) {
          label = \"Open link\";
        }
        return { link, label };
      });
      return `
      <article class="agent-card">
        <div>
          <h3>${escapeHtml(profile.display_name)}</h3>
          <p>${escapeHtml(profile.summary)}</p>
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
        <div class="agent-meta">${escapeHtml(profile.agent_id)}</div>
      </article>
    `;
    })
    .join("");
}

async function loadDirectory() {
  directoryMeta.textContent = "Loading directory…";
  const params = new URLSearchParams();
  if (searchInput.value.trim()) {
    params.set("q", searchInput.value.trim());
  }
  if (capabilityInput.value.trim()) {
    params.set("capability", capabilityInput.value.trim());
  }
  params.set("limit", "24");
  const query = params.toString() ? `?${params}` : "";
  try {
    const data = await fetchJson(`/directory/agents${query}`);
    renderProfiles(Array.isArray(data.results) ? data.results : []);
  } catch (err) {
    directoryMeta.textContent = "Directory unavailable.";
    directoryResults.innerHTML = "";
  }
}

function setupReveal() {
  const targets = document.querySelectorAll(".reveal");
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

searchButton.addEventListener("click", () => loadDirectory());
searchInput.addEventListener("keydown", (event) => {
  if (event.key === "Enter") {
    loadDirectory();
  }
});
capabilityInput.addEventListener("keydown", (event) => {
  if (event.key === "Enter") {
    loadDirectory();
  }
});

setupReveal();
loadStats();
loadMeshInfo();
loadDirectory();
