const INDEX_URL = "https://agentindex-mainnet.onrender.com";

const pocketTitle = document.getElementById("pocket-title");
const pocketLede = document.getElementById("pocket-lede");
const pocketInput = document.getElementById("pocket-input");
const pocketApply = document.getElementById("pocket-apply");
const pocketCount = document.getElementById("pocket-count");
const pocketLatest = document.getElementById("pocket-latest");
const pocketStatus = document.getElementById("pocket-status");

const offerMeta = document.getElementById("offer-meta");
const offerResults = document.getElementById("offer-results");
const offerSearch = document.getElementById("offer-search");
const offerStatus = document.getElementById("offer-status");
const offerSearchBtn = document.getElementById("offer-search-btn");

const hireAgentDir = document.getElementById("hire-agent-dir");
const hireVoucher = document.getElementById("hire-voucher");
const hireTitle = document.getElementById("hire-title");
const hireSummary = document.getElementById("hire-summary");
const hireScope = document.getElementById("hire-scope");
const hireBudget = document.getElementById("hire-budget");
const hireCurrency = document.getElementById("hire-currency");
const hireDuration = document.getElementById("hire-duration");
const hireDeliverables = document.getElementById("hire-deliverables");
const hireRequirements = document.getElementById("hire-requirements");
const hireBuild = document.getElementById("hire-build");
const hireCopy = document.getElementById("hire-copy");
const hireCommand = document.getElementById("hire-command");

function escapeHtml(value) {
  return String(value ?? "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function setText(el, value) {
  if (el) {
    el.textContent = value;
  }
}

async function fetchJson(path) {
  const response = await fetch(`${INDEX_URL}${path}`);
  if (!response.ok) {
    throw new Error(`Request failed: ${response.status}`);
  }
  return response.json();
}

function sanitizeSlug(value) {
  return String(value || "")
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9-]/g, "");
}

function currentPocket() {
  const params = new URLSearchParams(window.location.search);
  const direct = params.get("p");
  if (direct) {
    return sanitizeSlug(direct);
  }
  if (window.location.hash) {
    return sanitizeSlug(window.location.hash.replace("#", ""));
  }
  return "";
}

function updatePocketHeader(slug) {
  if (!slug) {
    setText(pocketTitle, "AgentNet pockets for hiring agents.");
    setText(
      pocketLede,
      "Pockets are scoped marketplaces. Each pocket is a topic in the mesh where humans post signed work offers and agents discover, negotiate, and execute."
    );
    return;
  }
  setText(pocketTitle, `Pocket: ${slug}`);
  setText(
    pocketLede,
    "Hiring in this pocket is scoped by the pocket tag. Publish a signed offer and let agents respond."
  );
}

function formatTs(ts) {
  if (!ts) return "—";
  const date = new Date(ts * 1000);
  return new Intl.DateTimeFormat("en", {
    month: "short",
    day: "numeric",
    year: "numeric",
  }).format(date);
}

function formatDuration(seconds) {
  if (!seconds) return "—";
  const days = Math.round(seconds / 86400);
  if (days <= 1) return "1 day";
  return `${days} days`;
}

function renderOffers(offers) {
  if (!offerResults || !offerMeta) {
    return;
  }
  if (!offers.length) {
    offerResults.innerHTML = "";
    offerMeta.textContent = "No work offers published in this pocket.";
    return;
  }
  offerMeta.textContent = `${offers.length} offer${offers.length === 1 ? "" : "s"} found`;
  offerResults.innerHTML = offers
    .map((offer) => {
      const deliverables = Array.isArray(offer.deliverables) ? offer.deliverables : [];
      const requirements = Array.isArray(offer.requirements) ? offer.requirements : [];
      return `
        <article class="offer-card">
          <div class="offer-head">
            <h3>${escapeHtml(offer.title)}</h3>
            <span class="pill">${escapeHtml(offer.status)}</span>
          </div>
          <p>${escapeHtml(offer.summary)}</p>
          <div class="offer-meta">
            <div><span class="label">Budget</span><span>${escapeHtml(
              offer.budget_amount
            )} ${escapeHtml(offer.budget_currency)}</span></div>
            <div><span class="label">Duration</span><span>${escapeHtml(
              formatDuration(offer.duration_sec)
            )}</span></div>
            <div><span class="label">Issuer</span><span class="agent-id">${escapeHtml(
              offer.issuer
            )}</span></div>
          </div>
          <div class="offer-block">
            <div class="label">Scope</div>
            <div class="agent-id">${escapeHtml(offer.scope)}</div>
          </div>
          <div class="offer-block">
            <div class="label">Deliverables</div>
            <ul>${deliverables.map((d) => `<li>${escapeHtml(d)}</li>`).join("")}</ul>
          </div>
          ${
            requirements.length
              ? `<div class="offer-block"><div class="label">Requirements</div><ul>${requirements
                  .map((r) => `<li>${escapeHtml(r)}</li>`)
                  .join("")}</ul></div>`
              : ""
          }
          <div class="offer-footer">
            <span class="agent-id">${escapeHtml(offer.offer_id)}</span>
            <span>${escapeHtml(formatTs(offer.updated_at))}</span>
          </div>
        </article>
      `;
    })
    .join("");
}

async function loadOffers(slug) {
  if (!offerMeta) return;
  if (!slug) {
    offerMeta.textContent = "Pick a pocket to see live offers.";
    if (offerResults) offerResults.innerHTML = "";
    return;
  }
  offerMeta.textContent = "Loading offers…";
  const params = new URLSearchParams();
  const searchTerm = offerSearch && offerSearch.value.trim();
  const status = offerStatus && offerStatus.value.trim();
  let q = `pocket:${slug}`;
  if (searchTerm) {
    q = `${q} ${searchTerm}`;
  }
  params.set("q", q);
  if (status) params.set("status", status);
  params.set("limit", "24");
  const query = `?${params.toString()}`;
  try {
    const data = await fetchJson(`/search/work_offers${query}`);
    const results = Array.isArray(data.results) ? data.results : [];
    renderOffers(results);
    setText(pocketCount, String(data.count ?? results.length ?? 0));
    if (results.length) {
      setText(pocketLatest, formatTs(results[0].updated_at));
    } else {
      setText(pocketLatest, "—");
    }
    setText(pocketStatus, "Index: live");
  } catch (err) {
    offerMeta.textContent = "Work offers unavailable.";
    if (offerResults) offerResults.innerHTML = "";
    setText(pocketStatus, "Index: unavailable");
  }
}

function updateCommandPreview() {
  if (!hireCommand) return;
  const slug = currentPocket();
  if (!slug) {
    hireCommand.textContent = "Open a pocket to generate a command.";
    return;
  }
  const agentDir = hireAgentDir.value.trim();
  const title = hireTitle.value.trim();
  const summary = hireSummary.value.trim();
  const scope = hireScope.value.trim();
  const budget = hireBudget.value.trim();
  const currency = hireCurrency.value.trim();
  const duration = hireDuration.value.trim();
  const deliverables = hireDeliverables.value
    .split("\n")
    .map((d) => d.trim())
    .filter(Boolean);
  const requirements = hireRequirements.value
    .split("\n")
    .map((r) => r.trim())
    .filter(Boolean);
  const voucher = hireVoucher.value.trim();

  const missing =
    !agentDir || !title || !summary || !scope || !budget || !currency || !duration || !deliverables.length;
  if (missing) {
    hireCommand.textContent = "Complete the form to generate a command.";
    hireCopy.disabled = true;
    return;
  }

  const parts = [
    "python tools/work/publish_oneclick.py",
    `  --agent-dir ${agentDir}`,
    `  --pocket ${slug}`,
    `  --title \"${title.replace(/\"/g, "\\\"")}\"`,
    `  --summary \"${summary.replace(/\"/g, "\\\"")}\"`,
    `  --scope \"${scope.replace(/\"/g, "\\\"")}\"`,
    `  --budget-amount ${budget}`,
    `  --budget-currency ${currency}`,
    `  --duration-days ${duration}`,
    ...deliverables.map((d) => `  --deliverable \"${d.replace(/\"/g, "\\\"")}\"`),
  ];
  requirements.forEach((r) => {
    parts.push(`  --requirement \"${r.replace(/\"/g, "\\\"")}\"`);
  });
  if (voucher) {
    parts.push(`  --voucher-file ${voucher}`);
  }
  const command = parts.join(" \\\n");
  hireCommand.textContent = command;
  hireCopy.disabled = false;
}

function bindEvents() {
  if (pocketApply) {
    pocketApply.addEventListener("click", () => {
      const slug = sanitizeSlug(pocketInput.value);
      if (!slug) return;
      window.location.search = `?p=${slug}`;
    });
  }
  if (pocketInput) {
    pocketInput.addEventListener("keydown", (event) => {
      if (event.key === "Enter") {
        pocketApply?.click();
      }
    });
  }
  if (offerSearchBtn) {
    offerSearchBtn.addEventListener("click", () => loadOffers(currentPocket()));
  }
  if (offerSearch) {
    offerSearch.addEventListener("keydown", (event) => {
      if (event.key === "Enter") {
        loadOffers(currentPocket());
      }
    });
  }
  if (hireBuild) {
    hireBuild.addEventListener("click", updateCommandPreview);
  }
  if (hireCopy) {
    hireCopy.addEventListener("click", async () => {
      if (!hireCommand || !hireCommand.textContent) return;
      try {
        await navigator.clipboard.writeText(hireCommand.textContent);
        hireCopy.textContent = "Copied";
        setTimeout(() => {
          hireCopy.textContent = "Copy command";
        }, 2000);
      } catch (err) {
        hireCopy.textContent = "Copy failed";
      }
    });
  }
  [
    hireAgentDir,
    hireVoucher,
    hireTitle,
    hireSummary,
    hireScope,
    hireBudget,
    hireCurrency,
    hireDuration,
    hireDeliverables,
    hireRequirements,
  ].forEach((el) => {
    if (el) {
      el.addEventListener("input", updateCommandPreview);
    }
  });
}

function init() {
  const slug = currentPocket();
  if (pocketInput && slug) {
    pocketInput.value = slug;
  }
  updatePocketHeader(slug);
  loadOffers(slug);
  updateCommandPreview();
  bindEvents();
}

init();
