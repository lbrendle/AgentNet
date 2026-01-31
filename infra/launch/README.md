# Launch Assets

This directory contains production launch assets and validation tooling. It does not embed any sample keys, domains, or placeholder configuration values.

## Network model (clarifying the “new internet” question)
AgentNet is an overlay network that runs on top of the existing Internet. It does not replace DNS or require a new TLD. Nodes advertise addresses (multiaddrs) and clients connect using those addresses. Public domains are optional, but recommended for stable gateways and operator endpoints.

## Cloud provider support
Yes — a federated AgentNet can run on any cloud provider (or bare metal). You can:
- deploy seed nodes and gateways on cloud VMs,
- run regional clusters for latency,
- keep kill‑switch custody offline/hardware‑backed.

## What lives here
- `systemd/` unit files for production services.
- `validate-config.py` validation for agentmesh and econ verifier configs.

## Required inputs (no defaults)
You must provide production configuration values and keys. The validation script will fail fast if required settings are missing or unsafe defaults are used.
