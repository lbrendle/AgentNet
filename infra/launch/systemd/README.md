# Systemd Units

These units are production‑safe templates that require explicit configuration files and environment variables. They do not include sample values.

## Required directories
- `/etc/agentnet/` for configuration
- `/var/lib/agentnet/` for state and databases
- `/var/log/agentnet/` for logs

## Environment files
Create these files with real values:
- `/etc/agentnet/agentindex.env`
  - `AGENTINDEX_BIND`
  - `AGENTINDEX_DB`
  - `AGENTINDEX_IDENTITY_STATE`
  - `AGENTINDEX_SKILL_STATE`
  - `AGENTINDEX_WORK_STATE`
- `/etc/agentnet/anet-econ-verify.env`
  - `ANET_ECON_VERIFY_CONFIG`

## Install units
```
sudo cp infra/launch/systemd/*.service /etc/systemd/system/
sudo systemctl daemon-reload
```

Start services only after configs are verified and state directories are provisioned.
