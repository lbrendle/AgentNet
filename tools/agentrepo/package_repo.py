#!/usr/bin/env python3
from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import json
import subprocess
import tarfile
import time
from pathlib import Path
from typing import Dict, List, Tuple


class RepoPackageError(ValueError):
    pass


def run_git(repo: Path, args: List[str]) -> str:
    cmd = ["git", "-C", str(repo), *args]
    return subprocess.check_output(cmd, text=True).strip()


def git_clean(repo: Path) -> bool:
    status = run_git(repo, ["status", "--porcelain"])
    return status == ""


def load_tree_entries(repo: Path, commit: str) -> List[Tuple[str, str, str]]:
    raw = subprocess.check_output(
        ["git", "-C", str(repo), "ls-tree", "-r", commit],
        text=True,
    )
    entries: List[Tuple[str, str, str]] = []
    for line in raw.splitlines():
        if not line.strip():
            continue
        # mode type sha\tpath
        left, path = line.split("\t", 1)
        mode, _typ, sha = left.split(" ", 2)
        entries.append((mode, sha, path))
    return entries


def git_blob(repo: Path, commit: str, path: str) -> bytes:
    return subprocess.check_output(["git", "-C", str(repo), "show", f"{commit}:{path}"])


def git_commit_ts(repo: Path, commit: str) -> int:
    ts = run_git(repo, ["show", "-s", "--format=%ct", commit])
    return int(ts)


def build_archive(repo: Path, commit: str, out_path: Path, prefix: str) -> None:
    entries = load_tree_entries(repo, commit)
    commit_ts = git_commit_ts(repo, commit)

    out_path.parent.mkdir(parents=True, exist_ok=True)
    with out_path.open("wb") as raw_out:
        with gzip.GzipFile(filename="", mode="wb", fileobj=raw_out, mtime=0) as gz:
            with tarfile.open(mode="w|", fileobj=gz) as tar:
                for mode, _sha, path in entries:
                    tar_name = f"{prefix}/{path}"
                    info = tarfile.TarInfo(name=tar_name)
                    info.mtime = commit_ts
                    info.uid = 0
                    info.gid = 0
                    info.uname = ""
                    info.gname = ""
                    if mode == "120000":
                        target = git_blob(repo, commit, path).decode("utf-8")
                        info.type = tarfile.SYMTYPE
                        info.linkname = target
                        info.mode = 0o777
                        tar.addfile(info)
                        continue
                    info.mode = 0o755 if mode == "100755" else 0o644
                    data = git_blob(repo, commit, path)
                    info.size = len(data)
                    tar.addfile(info, io.BytesIO(data))


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


def main() -> None:
    parser = argparse.ArgumentParser(description="Package a git repo into a deterministic archive.")
    parser.add_argument("--repo", type=Path, default=Path.cwd(), help="Repo path")
    parser.add_argument("--commit", default="HEAD", help="Commit or ref to package")
    parser.add_argument("--out", type=Path, required=True, help="Output archive path (tar.gz)")
    parser.add_argument("--prefix", default=None, help="Archive prefix directory")
    parser.add_argument("--allow-dirty", action="store_true", help="Allow dirty working tree")
    parser.add_argument("--metadata-out", type=Path, default=None, help="Output metadata JSON path")
    args = parser.parse_args()

    repo = args.repo.expanduser().resolve()
    if not (repo / ".git").exists():
        raise SystemExit(f"[agentrepo] not a git repo: {repo}")
    if not args.allow_dirty and not git_clean(repo):
        raise SystemExit("[agentrepo] working tree not clean (use --allow-dirty to override)")

    commit = run_git(repo, ["rev-parse", args.commit])
    tree = run_git(repo, ["rev-parse", f"{commit}^{{tree}}"])
    prefix = args.prefix or repo.name
    out_path = args.out.expanduser().resolve()

    build_archive(repo, commit, out_path, prefix)

    digest, size = sha256_file(out_path)
    metadata = {
        "repo_path": str(repo),
        "commit": commit,
        "tree": tree,
        "prefix": prefix,
        "archive_path": str(out_path),
        "archive_sha256": digest,
        "archive_size": size,
        "created_at": int(time.time()),
    }

    if args.metadata_out:
        meta_path = args.metadata_out.expanduser().resolve()
        meta_path.parent.mkdir(parents=True, exist_ok=True)
        meta_path.write_text(json.dumps(metadata, indent=2) + "\n")

    print(json.dumps(metadata, indent=2))


if __name__ == "__main__":
    main()
