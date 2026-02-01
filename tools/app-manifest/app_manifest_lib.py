#!/usr/bin/env python3
from __future__ import annotations

import base64
import hashlib
import json
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple
from urllib.parse import urlparse

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "impl" / "python"))

try:
    from agentnet_py.cbor import CborMap, CborValue
    from agentnet_py.skill import (
        SANDBOX_MAX,
        SANDBOX_MIN,
        SkillArtifact,
        SkillManifestPayload,
        build_skill_manifest,
    )
except Exception as exc:  # pragma: no cover
    raise SystemExit(f"[app-manifest] missing agentnet_py deps: {exc}")


class AppManifestError(ValueError):
    pass


@dataclass
class ParsedArtifact:
    kind: int
    digest_hex: Optional[str]
    size: Optional[int]
    uris: List[str]
    path: Optional[Path]
    extras: Dict[str, Any]


@dataclass
class ParsedApp:
    skill_id: str
    author: str
    name: str
    version: str
    summary: str
    license: str
    capabilities: List[str]
    permissions: List[str]
    sandbox_class: int
    endpoints: Optional[List[str]]
    artifacts: Optional[List[ParsedArtifact]]
    requirements: Optional[List[str]]
    pricing: Optional[CborValue]
    attestations: Optional[CborValue]
    metadata: Optional[CborValue]
    ts: int


def read_b64_key(path: Path) -> bytes:
    data = path.read_text().strip()
    if not data:
        raise AppManifestError(f"key file empty: {path}")
    raw = base64.b64decode(data)
    if len(raw) != 32:
        raise AppManifestError(f"key must be 32 bytes: {path}")
    return raw


def sha256_file(path: Path) -> Tuple[str, int]:
    h = hashlib.sha256()
    size = 0
    with path.open("rb") as handle:
        while True:
            chunk = handle.read(1024 * 1024)
            if not chunk:
                break
            size += len(chunk)
            h.update(chunk)
    return h.hexdigest(), size


def parse_app_markdown(text: str, artifact_root: Optional[Path]) -> ParsedApp:
    sections = _split_sections(text)
    identity = _parse_identity(sections.get("identity"))

    summary = _parse_summary(sections.get("summary"))
    capabilities = _parse_list_section(sections.get("capabilities"), "capability")
    if not capabilities:
        raise AppManifestError("capabilities section required")
    permissions = _parse_list_section(sections.get("permissions"), "permission")
    if permissions is None:
        permissions = []
    sandbox_class = _parse_sandbox(sections.get("sandbox"))
    endpoints = _parse_endpoints(sections.get("endpoints"))
    artifacts = _parse_artifacts(sections.get("artifacts"), artifact_root)
    requirements = _parse_list_section(sections.get("requirements"), None)
    pricing = _parse_json_block(sections.get("pricing"), "pricing")
    attestations = _parse_json_block(sections.get("attestations"), "attestations")
    metadata = _parse_json_block(sections.get("metadata"), "metadata")
    repo_meta = _parse_kv_section(sections.get("repository"))
    if repo_meta:
        metadata = _merge_metadata(metadata, {"repository": repo_meta})

    if not endpoints and not artifacts:
        raise AppManifestError("APP.md requires endpoints or artifacts")

    ts = int(time.time())
    return ParsedApp(
        skill_id=identity["skill_id"],
        author=identity["author"],
        name=identity["name"],
        version=identity["version"],
        summary=summary,
        license=identity["license"],
        capabilities=capabilities,
        permissions=permissions,
        sandbox_class=sandbox_class,
        endpoints=endpoints,
        artifacts=artifacts,
        requirements=requirements,
        pricing=pricing,
        attestations=attestations,
        metadata=metadata,
        ts=ts,
    )


def build_manifest_from_app(
    app_path: Path,
    agent_key: Path,
    agent_did: Optional[str],
    out_dir: Path,
    artifact_root: Optional[Path],
) -> Dict[str, Any]:
    text = app_path.read_text()
    parsed = parse_app_markdown(text, artifact_root)

    if agent_did and parsed.author != agent_did:
        raise AppManifestError("APP.md author does not match provided agent DID")

    secret_key = read_b64_key(agent_key)
    artifacts, artifact_extras = _build_artifacts(parsed.artifacts)
    metadata = _merge_metadata(parsed.metadata, {"artifacts": artifact_extras}) if artifact_extras else parsed.metadata

    payload = SkillManifestPayload(
        skill_id=parsed.skill_id,
        author=parsed.author,
        name=parsed.name,
        version=parsed.version,
        summary=parsed.summary,
        license=parsed.license,
        capabilities=parsed.capabilities,
        permissions=parsed.permissions,
        sandbox_class=parsed.sandbox_class,
        endpoints=parsed.endpoints,
        artifacts=artifacts,
        requirements=parsed.requirements,
        pricing=parsed.pricing,
        attestations=parsed.attestations,
        metadata=metadata,
        ts=parsed.ts,
    )

    manifest = build_skill_manifest(payload, secret_key)
    manifest_hash = hashlib.sha256(manifest).hexdigest()

    out_dir.mkdir(parents=True, exist_ok=True)
    manifest_path = out_dir / "app-manifest.cbor"
    manifest_hex_path = out_dir / "app-manifest.hex"
    manifest_json_path = out_dir / "app-manifest.json"

    manifest_path.write_bytes(manifest)
    manifest_hex_path.write_text(manifest.hex() + "\n")

    summary = {
        "app_id": parsed.skill_id,
        "author": parsed.author,
        "name": parsed.name,
        "version": parsed.version,
        "summary": parsed.summary,
        "license": parsed.license,
        "manifest_sha256": manifest_hash,
        "manifest_cbor": str(manifest_path),
        "manifest_hex": str(manifest_hex_path),
        "artifacts": artifact_extras,
        "ts": parsed.ts,
    }
    manifest_json_path.write_text(json.dumps(summary, indent=2) + "\n")

    return {
        "summary": summary,
        "manifest_bytes": manifest,
        "manifest_path": manifest_path,
        "manifest_hex_path": manifest_hex_path,
        "manifest_json_path": manifest_json_path,
    }


def _split_sections(text: str) -> Dict[str, List[str]]:
    sections: Dict[str, List[str]] = {}
    current: Optional[str] = None
    buf: List[str] = []
    for raw in text.replace("\r\n", "\n").replace("\r", "\n").split("\n"):
        line = raw.rstrip()
        if line.startswith("## "):
            if current:
                sections[current] = buf
                buf = []
            current = line[3:].strip().lower()
            continue
        if current is not None:
            buf.append(line)
    if current:
        sections[current] = buf
    return sections


def _parse_identity(lines: Optional[List[str]]) -> Dict[str, str]:
    entries = _parse_kv_section(lines, required=True)
    skill_id = entries.get("skill_id") or entries.get("app_id") or entries.get("id")
    if not skill_id:
        raise AppManifestError("identity requires skill_id or app_id")
    author = entries.get("author")
    name = entries.get("name")
    version = entries.get("version")
    license_name = entries.get("license")
    if not author or not name or not version or not license_name:
        raise AppManifestError("identity requires author, name, version, and license")
    return {
        "skill_id": skill_id,
        "author": author,
        "name": name,
        "version": version,
        "license": license_name,
    }


def _parse_summary(lines: Optional[List[str]]) -> str:
    if not lines:
        raise AppManifestError("summary section required")
    summary = "\n".join([line for line in lines]).strip()
    if not summary:
        raise AppManifestError("summary cannot be empty")
    return summary


def _parse_list_section(lines: Optional[List[str]], key_hint: Optional[str]) -> Optional[List[str]]:
    if lines is None:
        return None
    items: List[str] = []
    for raw in lines:
        line = raw.strip()
        if not line:
            continue
        if not line.startswith("- "):
            raise AppManifestError("list items must start with '-'")
        item = line[2:].strip()
        if key_hint and ":" in item:
            key, value = item.split(":", 1)
            if key.strip().lower() == key_hint:
                item = value.strip()
        if not item:
            raise AppManifestError("list item required")
        items.append(item)
    return items or None


def _parse_sandbox(lines: Optional[List[str]]) -> int:
    entries = _parse_kv_section(lines, required=True)
    raw = entries.get("class") or entries.get("sandbox_class")
    if not raw:
        raise AppManifestError("sandbox class required")
    try:
        sandbox = int(raw)
    except ValueError as exc:
        raise AppManifestError("sandbox class must be integer") from exc
    if sandbox < SANDBOX_MIN or sandbox > SANDBOX_MAX:
        raise AppManifestError("sandbox class out of range")
    return sandbox


def _parse_endpoints(lines: Optional[List[str]]) -> Optional[List[str]]:
    endpoints = _parse_list_section(lines, None)
    if not endpoints:
        return None
    for endpoint in endpoints:
        parsed = urlparse(endpoint)
        if parsed.scheme not in {"https", "wss", "agentnet"}:
            raise AppManifestError(f"invalid endpoint scheme: {endpoint}")
    return endpoints


def _parse_artifacts(lines: Optional[List[str]], artifact_root: Optional[Path]) -> Optional[List[ParsedArtifact]]:
    if not lines:
        return None
    artifacts: List[ParsedArtifact] = []
    current: Optional[ParsedArtifact] = None
    for raw in lines:
        if not raw.strip():
            continue
        indent = len(raw) - len(raw.lstrip(" "))
        line = raw.strip()
        if indent == 0 and line.startswith("- "):
            if current:
                artifacts.append(current)
            current = _parse_artifact_line(line[2:].strip(), artifact_root)
            continue
        if current is None:
            raise AppManifestError("artifact fields must follow an artifact entry")
        if indent < 2:
            raise AppManifestError("artifact fields must be indented")
        key, value = _parse_kv(line)
        _apply_artifact_field(current, key, value, artifact_root)
    if current:
        artifacts.append(current)
    if not artifacts:
        return None
    return artifacts


def _parse_artifact_line(line: str, artifact_root: Optional[Path]) -> ParsedArtifact:
    if not line:
        raise AppManifestError("artifact entry cannot be empty")
    if ":" not in line:
        raise AppManifestError("artifact entry requires key:value")
    key, value = _parse_kv(line)
    artifact = ParsedArtifact(kind=0, digest_hex=None, size=None, uris=[], path=None, extras={})
    _apply_artifact_field(artifact, key, value, artifact_root)
    return artifact


def _apply_artifact_field(artifact: ParsedArtifact, key: str, value: str, artifact_root: Optional[Path]) -> None:
    key = key.lower()
    if key == "kind":
        try:
            artifact.kind = int(value)
        except ValueError as exc:
            raise AppManifestError("artifact kind must be integer") from exc
        return
    if key == "digest":
        digest = value.strip()
        if digest.startswith("sha256:"):
            digest = digest.split(":", 1)[1]
        if len(digest) != 64:
            raise AppManifestError("artifact digest must be sha256 hex")
        artifact.digest_hex = digest.lower()
        return
    if key == "size":
        try:
            artifact.size = int(value)
        except ValueError as exc:
            raise AppManifestError("artifact size must be integer") from exc
        return
    if key == "uri":
        uri = value.strip()
        if uri:
            artifact.uris.append(uri)
        return
    if key == "uris":
        for uri in value.split(","):
            u = uri.strip()
            if u:
                artifact.uris.append(u)
        return
    if key == "path":
        path = Path(value.strip())
        if not path.is_absolute() and artifact_root:
            path = artifact_root / path
        artifact.path = path
        return
    artifact.extras[key] = value.strip()


def _build_artifacts(parsed: Optional[List[ParsedArtifact]]) -> Tuple[Optional[List[SkillArtifact]], List[Dict[str, Any]]]:
    if not parsed:
        return None, []
    artifacts: List[SkillArtifact] = []
    extras_out: List[Dict[str, Any]] = []
    for artifact in parsed:
        digest_hex = artifact.digest_hex
        size = artifact.size
        path = artifact.path
        if path:
            if not path.exists():
                raise AppManifestError(f"artifact path missing: {path}")
            computed_digest, computed_size = sha256_file(path)
            if digest_hex and computed_digest != digest_hex:
                raise AppManifestError(f"artifact digest mismatch: {path}")
            digest_hex = computed_digest
            if size and computed_size != size:
                raise AppManifestError(f"artifact size mismatch: {path}")
            size = computed_size
        if not digest_hex:
            raise AppManifestError("artifact digest required")
        if size is None:
            raise AppManifestError("artifact size required")
        if not artifact.uris:
            raise AppManifestError("artifact uris required")
        if artifact.kind <= 0:
            raise AppManifestError("artifact kind required")

        artifacts.append(
            SkillArtifact(
                kind=artifact.kind,
                digest=bytes.fromhex(digest_hex),
                size=size,
                uris=artifact.uris,
            )
        )
        extras = dict(artifact.extras)
        extras.update(
            {
                "kind": artifact.kind,
                "digest_hex": digest_hex,
                "size": size,
                "uris": artifact.uris,
            }
        )
        if path:
            extras["path"] = str(path)
        extras_out.append(extras)
    return artifacts, extras_out


def _parse_kv_section(lines: Optional[List[str]], required: bool = False) -> Dict[str, str]:
    if not lines:
        if required:
            raise AppManifestError("required section missing")
        return {}
    entries: Dict[str, str] = {}
    for raw in lines:
        line = raw.strip()
        if not line:
            continue
        if not line.startswith("- "):
            raise AppManifestError("section entries must start with '-'")
        key, value = _parse_kv(line[2:].strip())
        if key in entries:
            raise AppManifestError(f"duplicate field: {key}")
        entries[key] = value
    if required and not entries:
        raise AppManifestError("required section missing")
    return entries


def _parse_kv(value: str) -> Tuple[str, str]:
    if ":" not in value:
        raise AppManifestError("entry must be key: value")
    key, val = value.split(":", 1)
    key = key.strip().lower()
    val = val.strip()
    if not key or not val:
        raise AppManifestError("entry must be key: value")
    return key, val


def _parse_json_block(lines: Optional[List[str]], label: str) -> Optional[CborValue]:
    if not lines:
        return None
    in_block = False
    lang = None
    buf: List[str] = []
    for raw in lines:
        line = raw.strip()
        if line.startswith("```"):
            if not in_block:
                lang = line[3:].strip().lower()
                if lang and lang != "json":
                    raise AppManifestError(f"{label} block must be json")
                in_block = True
                continue
            in_block = False
            break
        if in_block:
            buf.append(raw)
    if not buf:
        return None
    try:
        data = json.loads("\n".join(buf))
    except json.JSONDecodeError as exc:
        raise AppManifestError(f"{label} json invalid") from exc
    return _json_to_cbor(data)


def _merge_metadata(existing: Optional[CborValue], extra: Dict[str, Any]) -> CborValue:
    extra_cbor = _json_to_cbor(extra)
    if existing is None:
        return extra_cbor
    if isinstance(existing, CborMap):
        entries = dict(existing.entries)
        if isinstance(extra_cbor, CborMap):
            entries.update(dict(extra_cbor.entries))
        return CborMap([(k, entries[k]) for k in sorted(entries, key=_cbor_sort_key)])
    raise AppManifestError("metadata must be a map")


def _json_to_cbor(value: Any) -> CborValue:
    if isinstance(value, dict):
        entries = [(k, _json_to_cbor(value[k])) for k in sorted(value, key=str)]
        return CborMap(entries)
    if isinstance(value, list):
        return [_json_to_cbor(item) for item in value]
    if isinstance(value, (str, int, bool)) or value is None:
        return value
    raise AppManifestError("unsupported json type in metadata")


def _cbor_sort_key(key: Any) -> str:
    if isinstance(key, str):
        return key
    return str(key)
