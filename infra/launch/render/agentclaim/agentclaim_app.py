from __future__ import annotations

import base64
import hashlib
import os
import re
import secrets
import sqlite3
import time
import uuid
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Optional

import cbor2
import httpx
from cryptography.hazmat.primitives.asymmetric import ed25519
from fastapi import FastAPI, HTTPException, Request
from pydantic import BaseModel


def canonical_cbor(value: object) -> bytes:
    return cbor2.dumps(value, canonical=True)


def sha256(data: bytes) -> bytes:
    return hashlib.sha256(data).digest()


def env_required(key: str) -> str:
    value = os.getenv(key)
    if not value:
        raise RuntimeError(f"missing required env var: {key}")
    return value


def env_int(key: str, default: int) -> int:
    raw = os.getenv(key)
    if raw is None:
        return default
    try:
        return int(raw)
    except ValueError as exc:
        raise RuntimeError(f"invalid int for {key}") from exc


def env_bool(key: str, default: bool) -> bool:
    raw = os.getenv(key)
    if raw is None:
        return default
    if raw.lower() in {"1", "true", "yes", "on"}:
        return True
    if raw.lower() in {"0", "false", "no", "off"}:
        return False
    raise RuntimeError(f"invalid bool for {key}")


def read_base64_key(path: Path) -> bytes:
    data = path.read_text().strip()
    if not data:
        raise RuntimeError(f"empty key file: {path}")
    raw = base64.b64decode(data)
    if len(raw) != 32:
        raise RuntimeError(f"issuer key must be 32 bytes: {path}")
    return raw


def validate_agent_did(did: str) -> bytes:
    if not did.startswith("did:anet:agent:"):
        raise ValueError("agent DID must start with did:anet:agent:")
    b64 = did.split("did:anet:agent:", 1)[1]
    raw = base64.b64decode(b64, validate=True)
    if len(raw) != 32:
        raise ValueError("agent DID pubkey must be 32 bytes")
    return raw


def normalize_handle(handle: str) -> str:
    handle = handle.strip()
    if handle.startswith("@"):
        handle = handle[1:]
    if not re.fullmatch(r"[A-Za-z0-9_]{1,15}", handle):
        raise ValueError("x_handle must match X username rules")
    return handle.lower()


def now_sec() -> int:
    return int(time.time())


def parse_created_at(value: str) -> Optional[int]:
    if not value:
        return None
    try:
        if value.endswith("Z"):
            dt = datetime.fromisoformat(value.replace("Z", "+00:00"))
        else:
            try:
                dt = datetime.strptime(value, "%a %b %d %H:%M:%S %z %Y")
            except ValueError:
                dt = datetime.fromisoformat(value)
        return int(dt.timestamp())
    except Exception:
        return None


@dataclass(frozen=True)
class Config:
    db_path: Path
    x_bearer_token: str
    x_api_base: str
    claim_ttl_sec: int
    claim_required_tag: str
    claim_require_handle: bool
    claim_rate_window_sec: int
    claim_max_per_ip: int
    claim_max_per_agent: int
    claim_max_per_handle: int
    claim_check_interval_sec: int
    claim_min_post_age_sec: int
    claim_api_key: Optional[str]
    issuer_did: str
    issuer_private: ed25519.Ed25519PrivateKey
    voucher_amount: int
    voucher_currency: str
    voucher_purpose: str
    voucher_ttl_sec: int


def load_config() -> Config:
    db_path = Path(env_required("ANET_CLAIM_DB_PATH"))
    bearer = env_required("X_BEARER_TOKEN")
    x_api_base = os.getenv("X_API_BASE", "https://api.x.com/2").rstrip("/")
    claim_ttl_sec = env_int("ANET_CLAIM_TTL_SEC", 1800)
    claim_required_tag = os.getenv("ANET_CLAIM_REQUIRED_TAG", "ANET-CLAIM")
    claim_require_handle = env_bool("ANET_CLAIM_REQUIRE_HANDLE", True)
    claim_rate_window_sec = env_int("ANET_CLAIM_RATE_WINDOW_SEC", 600)
    claim_max_per_ip = env_int("ANET_CLAIM_MAX_PER_IP", 20)
    claim_max_per_agent = env_int("ANET_CLAIM_MAX_PER_AGENT", 10)
    claim_max_per_handle = env_int("ANET_CLAIM_MAX_PER_HANDLE", 10)
    claim_check_interval_sec = env_int("ANET_CLAIM_CHECK_INTERVAL_SEC", 15)
    claim_min_post_age_sec = env_int("ANET_CLAIM_MIN_POST_AGE_SEC", 0)
    claim_api_key = os.getenv("ANET_CLAIM_API_KEY")
    issuer_did = env_required("ANET_VOUCHER_ISSUER_DID")
    issuer_key_path = Path(env_required("ANET_VOUCHER_ISSUER_KEY_PATH"))
    issuer_private = ed25519.Ed25519PrivateKey.from_private_bytes(read_base64_key(issuer_key_path))
    voucher_amount = env_int("ANET_VOUCHER_AMOUNT", 1)
    voucher_currency = env_required("ANET_VOUCHER_CURRENCY")
    voucher_purpose = env_required("ANET_VOUCHER_PURPOSE")
    voucher_ttl_sec = env_int("ANET_VOUCHER_TTL_SEC", 3600)
    if not claim_required_tag:
        raise RuntimeError("ANET_CLAIM_REQUIRED_TAG must be non-empty")
    return Config(
        db_path=db_path,
        x_bearer_token=bearer,
        x_api_base=x_api_base,
        claim_ttl_sec=claim_ttl_sec,
        claim_required_tag=claim_required_tag,
        claim_require_handle=claim_require_handle,
        claim_rate_window_sec=claim_rate_window_sec,
        claim_max_per_ip=claim_max_per_ip,
        claim_max_per_agent=claim_max_per_agent,
        claim_max_per_handle=claim_max_per_handle,
        claim_check_interval_sec=claim_check_interval_sec,
        claim_min_post_age_sec=claim_min_post_age_sec,
        claim_api_key=claim_api_key,
        issuer_did=issuer_did,
        issuer_private=issuer_private,
        voucher_amount=voucher_amount,
        voucher_currency=voucher_currency,
        voucher_purpose=voucher_purpose,
        voucher_ttl_sec=voucher_ttl_sec,
    )


def open_db(path: Path) -> sqlite3.Connection:
    conn = sqlite3.connect(path, check_same_thread=False)
    conn.row_factory = sqlite3.Row
    return conn


def init_db(path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    conn = open_db(path)
    conn.execute("PRAGMA journal_mode=WAL")
    conn.execute("PRAGMA foreign_keys=ON")
    conn.execute(
        """
        CREATE TABLE IF NOT EXISTS claims (
            claim_id TEXT PRIMARY KEY,
            agent_did TEXT NOT NULL,
            x_handle TEXT,
            x_user_id TEXT,
            claim_code TEXT NOT NULL,
            status TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            expires_at INTEGER NOT NULL,
            last_checked_at INTEGER,
            tweet_id TEXT,
            tweet_ts INTEGER,
            voucher_hex TEXT
        )
        """
    )
    conn.execute(
        """
        CREATE TABLE IF NOT EXISTS rate_limits (
            key TEXT PRIMARY KEY,
            count INTEGER NOT NULL,
            window_start INTEGER NOT NULL
        )
        """
    )
    conn.execute("CREATE INDEX IF NOT EXISTS idx_claims_agent ON claims(agent_did)")
    conn.execute("CREATE INDEX IF NOT EXISTS idx_claims_status ON claims(status)")
    conn.commit()
    conn.close()


def client_for(config: Config) -> httpx.Client:
    return httpx.Client(
        base_url=config.x_api_base,
        headers={"Authorization": f"Bearer {config.x_bearer_token}"},
        timeout=10.0,
    )


def require_api_key(config: Config, request: Request) -> None:
    if not config.claim_api_key:
        return
    header = request.headers.get("authorization", "")
    if not header.startswith("Bearer "):
        raise HTTPException(status_code=401, detail="missing claim api key")
    token = header[len("Bearer ") :].strip()
    if token != config.claim_api_key:
        raise HTTPException(status_code=403, detail="invalid claim api key")


def rate_limit(
    conn: sqlite3.Connection, key: str, limit: int, window_sec: int
) -> None:
    if limit <= 0:
        return
    now = now_sec()
    row = conn.execute(
        "SELECT count, window_start FROM rate_limits WHERE key = ?",
        (key,),
    ).fetchone()
    if row is None:
        conn.execute(
            "INSERT INTO rate_limits (key, count, window_start) VALUES (?, ?, ?)",
            (key, 1, now),
        )
        return
    window_start = int(row["window_start"])
    count = int(row["count"])
    if now - window_start >= window_sec:
        conn.execute(
            "UPDATE rate_limits SET count = ?, window_start = ? WHERE key = ?",
            (1, now, key),
        )
        return
    if count >= limit:
        raise HTTPException(status_code=429, detail="rate limit exceeded")
    conn.execute(
        "UPDATE rate_limits SET count = ? WHERE key = ?",
        (count + 1, key),
    )


def claim_required_post(config: Config, claim_id: str, claim_code: str, agent_did: str) -> str:
    return f"{config.claim_required_tag} {claim_id} {claim_code} {agent_did}"


def lookup_x_user(client: httpx.Client, handle: str) -> tuple[str, str]:
    resp = client.get(f"/users/by/username/{handle}")
    if resp.status_code == 200:
        data = resp.json().get("data") or {}
        user_id = data.get("id")
        username = data.get("username")
        if user_id and username:
            return str(user_id), str(username).lower()
    if resp.status_code == 429:
        raise HTTPException(status_code=429, detail="x api rate limited")
    if resp.status_code == 404:
        raise HTTPException(status_code=400, detail="x_handle not found")
    raise HTTPException(status_code=502, detail=f"x api lookup failed ({resp.status_code})")


def search_recent(
    client: httpx.Client, query: str, max_results: int
) -> list[dict[str, Any]]:
    resp = client.get(
        "/tweets/search/recent",
        params={
            "query": query,
            "max_results": max_results,
            "tweet.fields": "author_id,created_at",
        },
    )
    if resp.status_code == 429:
        raise HTTPException(status_code=429, detail="x api rate limited")
    if resp.status_code != 200:
        raise HTTPException(status_code=502, detail=f"x api search failed ({resp.status_code})")
    data = resp.json()
    items = data.get("data")
    if not isinstance(items, list):
        return []
    return items


def build_query(
    config: Config, claim_id: str, claim_code: str, agent_did: str, handle: Optional[str]
) -> str:
    parts = [
        f"\"{config.claim_required_tag}\"",
        f"\"{claim_id}\"",
        f"\"{claim_code}\"",
        f"\"{agent_did}\"",
        "-is:retweet",
        "-is:reply",
    ]
    if handle:
        parts.append(f"from:{handle}")
    return " ".join(parts)


def contains_required_text(text: str, tokens: list[str]) -> bool:
    lowered = text.lower()
    for token in tokens:
        if token.lower() not in lowered:
            return False
    return True


def issue_voucher(config: Config, agent_did: str) -> str:
    now = now_sec()
    exp = now + config.voucher_ttl_sec
    nonce = secrets.token_bytes(16)
    payload = {
        0: config.issuer_did,
        1: agent_did,
        2: config.voucher_amount,
        3: config.voucher_currency,
        4: config.voucher_purpose,
        5: now,
        6: exp,
        7: nonce,
    }
    payload_cbor = canonical_cbor(payload)
    payload_hash = sha256(payload_cbor)
    sig = config.issuer_private.sign(payload_hash)
    voucher_map = dict(payload)
    voucher_map[8] = sig
    voucher_cbor = canonical_cbor(voucher_map)
    return voucher_cbor.hex()


def serialize_claim(config: Config, row: sqlite3.Row) -> dict[str, Any]:
    claim_id = row["claim_id"]
    claim_code = row["claim_code"]
    agent_did = row["agent_did"]
    return {
        "claim_id": claim_id,
        "agent_did": agent_did,
        "x_handle": row["x_handle"],
        "x_user_id": row["x_user_id"],
        "status": row["status"],
        "created_at": row["created_at"],
        "expires_at": row["expires_at"],
        "last_checked_at": row["last_checked_at"],
        "tweet_id": row["tweet_id"],
        "tweet_ts": row["tweet_ts"],
        "voucher_hex": row["voucher_hex"],
        "required_post": claim_required_post(config, claim_id, claim_code, agent_did),
    }


class ClaimRequest(BaseModel):
    agent_did: str
    x_handle: Optional[str] = None


app = FastAPI(title="AgentNet Claim Service", version="1.0.0")


@app.on_event("startup")
def on_startup() -> None:
    config = load_config()
    init_db(config.db_path)
    app.state.config = config
    app.state.client = client_for(config)


def fetch_claim(conn: sqlite3.Connection, claim_id: str) -> sqlite3.Row:
    row = conn.execute("SELECT * FROM claims WHERE claim_id = ?", (claim_id,)).fetchone()
    if row is None:
        raise HTTPException(status_code=404, detail="claim not found")
    return row


def maybe_mark_expired(conn: sqlite3.Connection, row: sqlite3.Row) -> sqlite3.Row:
    if row["status"] in {"issued", "expired", "revoked"}:
        return row
    if now_sec() >= int(row["expires_at"]):
        conn.execute(
            "UPDATE claims SET status = ? WHERE claim_id = ?",
            ("expired", row["claim_id"]),
        )
        conn.commit()
        return fetch_claim(conn, row["claim_id"])
    return row


def verify_and_issue(
    config: Config, conn: sqlite3.Connection, client: httpx.Client, row: sqlite3.Row
) -> sqlite3.Row:
    row = maybe_mark_expired(conn, row)
    if row["status"] in {"issued", "expired"}:
        return row
    now = now_sec()
    last_checked = row["last_checked_at"] or 0
    if now - int(last_checked) < config.claim_check_interval_sec:
        return row
    claim_id = row["claim_id"]
    claim_code = row["claim_code"]
    agent_did = row["agent_did"]
    handle = row["x_handle"]
    query = build_query(config, claim_id, claim_code, agent_did, handle)
    items = search_recent(client, query, max_results=10)
    required_tokens = [config.claim_required_tag, claim_id, claim_code, agent_did]
    matched = None
    for item in items:
        text = item.get("text") or ""
        if not contains_required_text(text, required_tokens):
            continue
        if row["x_user_id"]:
            author_id = item.get("author_id")
            if author_id and str(author_id) != str(row["x_user_id"]):
                continue
        created_at = parse_created_at(item.get("created_at") or "")
        if created_at is None:
            continue
        min_post_ts = int(row["created_at"]) + config.claim_min_post_age_sec
        if created_at < min_post_ts:
            continue
        matched = item
        break
    conn.execute(
        "UPDATE claims SET last_checked_at = ? WHERE claim_id = ?",
        (now, claim_id),
    )
    if matched:
        tweet_id = matched.get("id")
        tweet_ts = parse_created_at(matched.get("created_at") or "")
        voucher_hex = issue_voucher(config, agent_did)
        conn.execute(
            """
            UPDATE claims
            SET status = ?, tweet_id = ?, tweet_ts = ?, voucher_hex = ?
            WHERE claim_id = ?
            """,
            ("issued", tweet_id, tweet_ts, voucher_hex, claim_id),
        )
    conn.commit()
    return fetch_claim(conn, claim_id)


@app.get("/health")
def health() -> dict[str, Any]:
    return {"ok": True}


@app.get("/stats")
def stats() -> dict[str, Any]:
    config: Config = app.state.config
    conn = open_db(config.db_path)
    total = conn.execute("SELECT COUNT(*) AS c FROM claims").fetchone()["c"]
    issued = conn.execute(
        "SELECT COUNT(*) AS c FROM claims WHERE status = ?",
        ("issued",),
    ).fetchone()["c"]
    pending = conn.execute(
        "SELECT COUNT(*) AS c FROM claims WHERE status = ?",
        ("pending",),
    ).fetchone()["c"]
    conn.close()
    return {"total_claims": total, "issued_claims": issued, "pending_claims": pending}


@app.post("/v1/claims")
def create_claim(request: Request, payload: ClaimRequest) -> dict[str, Any]:
    config: Config = app.state.config
    client: httpx.Client = app.state.client
    require_api_key(config, request)
    try:
        validate_agent_did(payload.agent_did)
    except Exception as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc
    handle = None
    user_id = None
    if payload.x_handle:
        try:
            handle = normalize_handle(payload.x_handle)
        except Exception as exc:
            raise HTTPException(status_code=400, detail=str(exc)) from exc
        user_id, handle = lookup_x_user(client, handle)
    elif config.claim_require_handle:
        raise HTTPException(status_code=400, detail="x_handle required")
    conn = open_db(config.db_path)
    ip = request.headers.get("x-forwarded-for", request.client.host or "unknown")
    ip = (ip or "unknown").split(",")[0].strip()
    try:
        with conn:
            existing = conn.execute(
                """
                SELECT * FROM claims
                WHERE agent_did = ? AND status IN ('pending', 'verified', 'issued')
                ORDER BY created_at DESC
                LIMIT 1
                """,
                (payload.agent_did,),
            ).fetchone()
            if existing and now_sec() < int(existing["expires_at"]):
                return serialize_claim(config, existing)
            rate_limit(conn, f"ip:{ip}", config.claim_max_per_ip, config.claim_rate_window_sec)
            rate_limit(
                conn,
                f"agent:{payload.agent_did}",
                config.claim_max_per_agent,
                config.claim_rate_window_sec,
            )
            if handle:
                rate_limit(
                    conn,
                    f"handle:{handle}",
                    config.claim_max_per_handle,
                    config.claim_rate_window_sec,
                )
            claim_id = str(uuid.uuid4())
            claim_code = secrets.token_hex(8)
            created_at = now_sec()
            expires_at = created_at + config.claim_ttl_sec
            conn.execute(
                """
                INSERT INTO claims (
                    claim_id, agent_did, x_handle, x_user_id, claim_code, status,
                    created_at, expires_at
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                """,
                (
                    claim_id,
                    payload.agent_did,
                    handle,
                    user_id,
                    claim_code,
                    "pending",
                    created_at,
                    expires_at,
                ),
            )
            row = fetch_claim(conn, claim_id)
            return serialize_claim(config, row)
    finally:
        conn.close()


@app.get("/v1/claims/{claim_id}")
def get_claim(claim_id: str) -> dict[str, Any]:
    config: Config = app.state.config
    client: httpx.Client = app.state.client
    conn = open_db(config.db_path)
    try:
        row = fetch_claim(conn, claim_id)
        row = verify_and_issue(config, conn, client, row)
        return serialize_claim(config, row)
    finally:
        conn.close()


@app.post("/v1/claims/{claim_id}/revoke")
def revoke_claim(claim_id: str, request: Request) -> dict[str, Any]:
    config: Config = app.state.config
    require_api_key(config, request)
    conn = open_db(config.db_path)
    try:
        fetch_claim(conn, claim_id)
        conn.execute(
            "UPDATE claims SET status = ? WHERE claim_id = ?",
            ("revoked", claim_id),
        )
        conn.commit()
        row = fetch_claim(conn, claim_id)
        return serialize_claim(config, row)
    finally:
        conn.close()


@app.post("/v1/claims/{claim_id}/verify")
def verify_claim(claim_id: str) -> dict[str, Any]:
    config: Config = app.state.config
    client: httpx.Client = app.state.client
    conn = open_db(config.db_path)
    try:
        row = fetch_claim(conn, claim_id)
        row = verify_and_issue(config, conn, client, row)
        return serialize_claim(config, row)
    finally:
        conn.close()
