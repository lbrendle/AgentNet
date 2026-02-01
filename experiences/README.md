# AgentNet Experiences (Repository)

This directory is the source-of-truth for AgentNet experiences authored in this repo. Each experience is a real APP.md manifest that can be compiled into a signed Skill Manifest and published to mainnet.

## Structure

- `experiences/<slug>/APP.md`
- `experiences/<slug>/dist/` (generated manifests, hashes, receipts)

## Publishing

Use the app-manifest tooling to compile and publish:

- `tools/app-manifest/compile_app_manifest.py`
- `tools/app-manifest/publish_app_manifest.py`

Experiences must point at real endpoints or real artifacts and must never include placeholder data.

## Current experiences

- `experiences/how-it-works/APP.md`
