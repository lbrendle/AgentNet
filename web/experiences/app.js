const INDEX_URL = "https://agentindex-mainnet.onrender.com";

const experienceCount = document.getElementById("experience-count");
const experienceUpdated = document.getElementById("experience-updated");
const experienceMeta = document.getElementById("experience-meta");
const experienceResults = document.getElementById("experience-results");
const searchInput = document.getElementById("experience-search");
const capabilityInput = document.getElementById("experience-capability");
const statusSelect = document.getElementById("experience-status");
const searchButton = document.getElementById("experience-submit");
const footerStatus = document.getElementById("experience-status-text");

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

function formatTimestamp(ts) {
  if (!ts) {
    return "—";
  }
  const date = new Date(ts * 1000);
  if (Number.isNaN(date.getTime())) {
    return "—";
  }
  return date.toLocaleString();
}

function renderExperiences(items) {
  if (!items.length) {
    if (experienceResults) {
      experienceResults.innerHTML = "";
    }
    setText(experienceMeta, "No experiences published yet.");
    return;
  }

  setText(
    experienceMeta,
    `${items.length} experience${items.length === 1 ? "" : "s"} found`
  );
  if (!experienceResults) {
    return;
  }
  experienceResults.innerHTML = items
    .map((item) => {
      const caps = Array.isArray(item.capabilities) ? item.capabilities : [];
      const endpoints = Array.isArray(item.endpoints) ? item.endpoints : [];
      const isRevoked = item.revoked === true;
      const statusLabel = isRevoked ? "Revoked" : "Active";
      const endpointLinks = endpoints
        .slice(0, 3)
        .map((endpoint) => {
          const safe = escapeHtml(endpoint);
          return `<a class="pill" href="${safe}" target="_blank" rel="noreferrer">Endpoint</a>`;
        })
        .join("");
      return `
      <article class="agent-card">
        <div>
          <h3>${escapeHtml(item.name || item.skill_id)}</h3>
          <p>${escapeHtml(item.summary || "")}</p>
        </div>
        <div class="pill-row">
          <span class="pill">${escapeHtml(statusLabel)}</span>
          <span class="pill">${escapeHtml(item.version || "")}</span>
          <span class="pill">Sandbox ${escapeHtml(String(item.sandbox_class ?? ""))}</span>
        </div>
        <div class="pill-row">
          ${caps.map((cap) => `<span class="pill">${escapeHtml(cap)}</span>`).join("")}
        </div>
        <div class="pill-row">
          ${endpointLinks}
        </div>
        <div class="agent-meta">${escapeHtml(item.author || "—")}</div>
      </article>
    `;
    })
    .join("");
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

async function loadExperienceStats() {
  try {
    const data = await fetchJson("/search/experiences?limit=1");
    const count = data.count ?? 0;
    setText(experienceCount, count.toString());
    const results = Array.isArray(data.results) ? data.results : [];
    const latest = results.length ? results[0] : null;
    setText(experienceUpdated, latest ? formatTimestamp(latest.updated_at) : "—");
    setText(footerStatus, "Index: live");
  } catch (err) {
    setText(experienceCount, "—");
    setText(experienceUpdated, "—");
    setText(footerStatus, "Index: unavailable");
  }
}

async function loadDirectory() {
  if (!experienceMeta) {
    return;
  }
  experienceMeta.textContent = "Loading experiences…";
  const params = new URLSearchParams();
  if (searchInput && searchInput.value.trim()) {
    params.set("q", searchInput.value.trim());
  }
  if (capabilityInput && capabilityInput.value.trim()) {
    params.set("capability", capabilityInput.value.trim());
  }
  if (statusSelect && statusSelect.value) {
    params.set("status", statusSelect.value);
  }
  params.set("limit", "24");
  const query = params.toString() ? `?${params}` : "";
  try {
    const data = await fetchJson(`/search/experiences${query}`);
    renderExperiences(Array.isArray(data.results) ? data.results : []);
  } catch (err) {
    experienceMeta.textContent = "Experiences unavailable.";
    if (experienceResults) {
      experienceResults.innerHTML = "";
    }
  }
}

if (searchButton) {
  searchButton.addEventListener("click", () => {
    loadDirectory();
  });
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

if (statusSelect) {
  statusSelect.addEventListener("change", () => {
    loadDirectory();
  });
}

loadExperienceStats();
loadDirectory();
setupReveal();
