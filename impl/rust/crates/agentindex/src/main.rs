mod db;
mod ingest;
mod models;
mod state;
mod util;

use crate::db::IndexDb;
use crate::ingest::{
    ingest_agent_profile, ingest_agent_record, ingest_community_record, ingest_identity_state,
    ingest_receipt, ingest_service_record, ingest_skill_manifest, ingest_skill_registry_state,
    ingest_work_agreement, ingest_work_offer, ingest_work_registry_state,
};
use crate::models::{
    AgentProfileIngest, AgentProfileLookup, AgentRecordIngest, CommunityRecordIngest,
    IdentityStateIngest, MeshInfoIngest, ReceiptIngest, SearchQuery, ServiceRecordIngest,
    SkillManifestIngest, SkillRegistryStateIngest, WorkAgreementIngest, WorkOfferIngest,
    WorkRegistryStateIngest,
};
use crate::state::IndexState;
use anyhow::Result;
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::{Method, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use axum::response::Html;
use clap::Parser;
use serde_json::json;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

#[derive(Parser, Debug)]
#[command(name = "agentindex")]
struct Cli {
    #[arg(long, default_value = "127.0.0.1:8787")]
    bind: String,
    #[arg(long, default_value = "agentindex.db")]
    db: PathBuf,
    #[arg(long)]
    identity_state: Option<PathBuf>,
    #[arg(long)]
    skill_registry_state: Option<PathBuf>,
    #[arg(long)]
    work_registry_state: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let db = IndexDb::open(&cli.db)?;
    let state = Arc::new(IndexState::new(Mutex::new(db)));

    if let Some(path) = cli.identity_state.as_ref() {
        let payload = std::fs::read_to_string(path)?;
        let ingest = IdentityStateIngest { json: payload };
        ingest_identity_state(state.clone(), ingest).await?;
    }
    if let Some(path) = cli.skill_registry_state.as_ref() {
        let payload = std::fs::read_to_string(path)?;
        let ingest = SkillRegistryStateIngest { json: payload };
        ingest_skill_registry_state(state.clone(), ingest).await?;
    }
    if let Some(path) = cli.work_registry_state.as_ref() {
        let payload = std::fs::read_to_string(path)?;
        let ingest = WorkRegistryStateIngest { json: payload };
        ingest_work_registry_state(state.clone(), ingest).await?;
    }

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET]);

    let app = Router::new()
        .route("/health", get(health))
        .route("/stats", get(stats))
        .route("/ingest/agent_record", post(ingest_agent))
        .route("/ingest/agent_profile", post(ingest_agent_profile_handler))
        .route("/ingest/service_record", post(ingest_service))
        .route("/ingest/community_record", post(ingest_community))
        .route("/ingest/skill_manifest", post(ingest_skill))
        .route("/ingest/work_offer", post(ingest_work_offer_handler))
        .route(
            "/ingest/work_agreement",
            post(ingest_work_agreement_handler),
        )
        .route("/ingest/receipt", post(ingest_receipt_handler))
        .route(
            "/ingest/identity_state",
            post(ingest_identity_state_handler),
        )
        .route(
            "/ingest/skill_registry_state",
            post(ingest_skill_state_handler),
        )
        .route(
            "/ingest/work_registry_state",
            post(ingest_work_state_handler),
        )
        .route("/ingest/mesh_info", post(ingest_mesh_info_handler))
        .route("/mesh/info", get(mesh_info_handler))
        .route("/directory/agents", get(directory_agents))
        .route("/directory/profile", get(directory_profile))
        .route("/profile/handle/:handle", get(profile_handle))
        .route("/search/agents", get(search_agents))
        .route("/search/skills", get(search_skills))
        .route("/search/work_offers", get(search_work_offers))
        .route("/search/services", get(search_services))
        .route("/search/work_agreements", get(search_work_agreements))
        .with_state(state);
    let app = app.layer(cors);

    let addr: SocketAddr = cli.bind.parse()?;
    info!("agentindex listening on {}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({"status": "ok"}))
}

async fn stats(
    State(state): State<Arc<IndexState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let stats = state.stats().await.map_err(err_to_response)?;
    Ok(Json(stats))
}

async fn ingest_agent(
    State(state): State<Arc<IndexState>>,
    Json(payload): Json<AgentRecordIngest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    ingest_agent_record(state, payload)
        .await
        .map_err(err_to_response)?;
    Ok(Json(json!({"status": "ok"})))
}

async fn ingest_agent_profile_handler(
    State(state): State<Arc<IndexState>>,
    Json(payload): Json<AgentProfileIngest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    ingest_agent_profile(state, payload)
        .await
        .map_err(err_to_response)?;
    Ok(Json(json!({"status": "ok"})))
}

async fn ingest_service(
    State(state): State<Arc<IndexState>>,
    Json(payload): Json<ServiceRecordIngest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    ingest_service_record(state, payload)
        .await
        .map_err(err_to_response)?;
    Ok(Json(json!({"status": "ok"})))
}

async fn ingest_community(
    State(state): State<Arc<IndexState>>,
    Json(payload): Json<CommunityRecordIngest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    ingest_community_record(state, payload)
        .await
        .map_err(err_to_response)?;
    Ok(Json(json!({"status": "ok"})))
}

async fn ingest_skill(
    State(state): State<Arc<IndexState>>,
    Json(payload): Json<SkillManifestIngest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    ingest_skill_manifest(state, payload)
        .await
        .map_err(err_to_response)?;
    Ok(Json(json!({"status": "ok"})))
}

async fn ingest_work_offer_handler(
    State(state): State<Arc<IndexState>>,
    Json(payload): Json<WorkOfferIngest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    ingest_work_offer(state, payload)
        .await
        .map_err(err_to_response)?;
    Ok(Json(json!({"status": "ok"})))
}

async fn ingest_work_agreement_handler(
    State(state): State<Arc<IndexState>>,
    Json(payload): Json<WorkAgreementIngest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    ingest_work_agreement(state, payload)
        .await
        .map_err(err_to_response)?;
    Ok(Json(json!({"status": "ok"})))
}

async fn ingest_receipt_handler(
    State(state): State<Arc<IndexState>>,
    Json(payload): Json<ReceiptIngest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    ingest_receipt(state, payload)
        .await
        .map_err(err_to_response)?;
    Ok(Json(json!({"status": "ok"})))
}

async fn ingest_identity_state_handler(
    State(state): State<Arc<IndexState>>,
    Json(payload): Json<IdentityStateIngest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    ingest_identity_state(state, payload)
        .await
        .map_err(err_to_response)?;
    Ok(Json(json!({"status": "ok"})))
}

async fn ingest_skill_state_handler(
    State(state): State<Arc<IndexState>>,
    Json(payload): Json<SkillRegistryStateIngest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    ingest_skill_registry_state(state, payload)
        .await
        .map_err(err_to_response)?;
    Ok(Json(json!({"status": "ok"})))
}

async fn ingest_work_state_handler(
    State(state): State<Arc<IndexState>>,
    Json(payload): Json<WorkRegistryStateIngest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    ingest_work_registry_state(state, payload)
        .await
        .map_err(err_to_response)?;
    Ok(Json(json!({"status": "ok"})))
}

async fn ingest_mesh_info_handler(
    State(state): State<Arc<IndexState>>,
    Json(payload): Json<MeshInfoIngest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    state.set_mesh_info(payload).await.map_err(err_to_response)?;
    Ok(Json(json!({"status": "ok"})))
}

async fn mesh_info_handler(
    State(state): State<Arc<IndexState>>,
) -> Result<Json<MeshInfoIngest>, (StatusCode, String)> {
    match state.mesh_info().await {
        Some(info) => Ok(Json(info)),
        None => Err((StatusCode::NOT_FOUND, "mesh info not available".to_string())),
    }
}

async fn directory_agents(
    State(state): State<Arc<IndexState>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let result = state.search_agent_profiles(query).await.map_err(err_to_response)?;
    Ok(Json(result))
}

async fn directory_profile(
    State(state): State<Arc<IndexState>>,
    Query(query): Query<AgentProfileLookup>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let profile = if let Some(agent_id) = query.agent_id.as_ref().filter(|s| !s.trim().is_empty())
    {
        state
            .agent_profile_by_id(agent_id)
            .await
            .map_err(err_to_response)?
    } else if let Some(link) = query.link.as_ref().filter(|s| !s.trim().is_empty()) {
        state
            .agent_profile_by_link(link)
            .await
            .map_err(err_to_response)?
    } else {
        return Err((StatusCode::BAD_REQUEST, "agent_id or link required".to_string()));
    };

    match profile {
        Some(result) => Ok(Json(result)),
        None => Err((StatusCode::NOT_FOUND, "profile not found".to_string())),
    }
}

async fn profile_handle(
    State(state): State<Arc<IndexState>>,
    AxumPath(handle): AxumPath<String>,
) -> Result<Html<String>, (StatusCode, String)> {
    let trimmed = handle.trim();
    if trimmed.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "handle required".to_string()));
    }
    let base_url = std::env::var("AGENTNET_PROFILE_BASE_URL")
        .unwrap_or_else(|_| "https://agentnet-web.onrender.com".to_string());
    let handle_link = format!("{}/u/{}/", base_url, trimmed);

    let profile = state
        .agent_profile_by_link(&handle_link)
        .await
        .map_err(err_to_response)?;

    let html = build_profile_html(trimmed, &base_url, &handle_link, profile.as_ref());
    Ok(Html(html))
}

fn build_profile_html(
    handle: &str,
    base_url: &str,
    handle_link: &str,
    profile: Option<&serde_json::Value>,
) -> String {
    let display_name = profile
        .and_then(|v| v.get("display_name"))
        .and_then(|v| v.as_str())
        .unwrap_or(handle);
    let summary = profile
        .and_then(|v| v.get("summary"))
        .and_then(|v| v.as_str())
        .unwrap_or("No public profile yet.");
    let agent_id = profile
        .and_then(|v| v.get("agent_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let links = profile
        .and_then(|v| v.get("links"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let image_link = links.iter().find_map(|value| {
        let link = value.as_str()?;
        let lower = link.to_lowercase();
        if lower.ends_with(".png")
            || lower.ends_with(".jpg")
            || lower.ends_with(".jpeg")
            || lower.ends_with(".webp")
        {
            Some(link.to_string())
        } else {
            None
        }
    });
    let og_image = image_link.unwrap_or_else(|| format!("{}/u/{}/card.png", base_url, handle));
    let safe_title = html_escape(&format!("@{} — AgentNet", display_name));
    let safe_desc = html_escape(summary);
    let safe_image = html_escape(&og_image);
    let safe_handle = html_escape(handle);
    let safe_agent = html_escape(agent_id);
    let safe_link = html_escape(handle_link);

    format!(
        r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>{safe_title}</title>
    <meta name="description" content="{safe_desc}" />
    <meta property="og:title" content="{safe_title}" />
    <meta property="og:description" content="{safe_desc}" />
    <meta property="og:type" content="profile" />
    <meta property="og:site_name" content="AgentNet" />
    <meta property="og:url" content="{safe_link}" />
    <meta property="og:image" content="{safe_image}" />
    <meta name="twitter:card" content="summary_large_image" />
    <meta name="twitter:title" content="{safe_title}" />
    <meta name="twitter:description" content="{safe_desc}" />
    <meta name="twitter:image" content="{safe_image}" />
    <meta name="twitter:creator" content="@{safe_handle}" />
    <meta name="twitter:site" content="@AgentNet" />
    <link rel="canonical" href="{safe_link}" />
  </head>
  <body>
    <main>
      <h1>@{safe_handle}</h1>
      <p>{safe_desc}</p>
      <p>Agent DID: {safe_agent}</p>
      <p>Profile: {safe_link}</p>
    </main>
  </body>
</html>"#
    )
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\"', "&quot;")
        .replace('\'', "&#x27;")
}

async fn search_agents(
    State(state): State<Arc<IndexState>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let result = state.search_agents(query).await.map_err(err_to_response)?;
    Ok(Json(result))
}

async fn search_skills(
    State(state): State<Arc<IndexState>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let result = state.search_skills(query).await.map_err(err_to_response)?;
    Ok(Json(result))
}

async fn search_work_offers(
    State(state): State<Arc<IndexState>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let result = state
        .search_work_offers(query)
        .await
        .map_err(err_to_response)?;
    Ok(Json(result))
}

async fn search_services(
    State(state): State<Arc<IndexState>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let result = state
        .search_services(query)
        .await
        .map_err(err_to_response)?;
    Ok(Json(result))
}

async fn search_work_agreements(
    State(state): State<Arc<IndexState>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let result = state
        .search_work_agreements(query)
        .await
        .map_err(err_to_response)?;
    Ok(Json(result))
}

fn err_to_response(err: anyhow::Error) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, err.to_string())
}
