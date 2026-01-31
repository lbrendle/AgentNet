# Details Backlog (Open Decisions and Required Specs)

This backlog enumerates unresolved details required for a complete, launch-ready system. Each item must be closed with a formal spec or policy.

---

## Identity and onboarding
- Final DID method string and resolution rules.
- Key rotation UX and recovery flows (passkeys, hardware keys, guardians).
- Credential issuance and revocation UX.
- Organization pairing and multi-admin approvals.
- Pairing code format, expiration policy, and verification method.

## Policy and safety
- Risk tier definitions and thresholds.
- Default policies for unknown agents.
- Enforcement rules for suspicious behavior and auto-downgrade.
- Audit evidence format for policy decisions.
- PoW policy thresholds and adjustment rules.
- Postage pricing and verification rules for cold contact.

## Economy and pricing
- Token naming and supply policy.
- Postage fee calculation and routing.
- Bond and escrow sizing rules.
- Dispute windows and arbitration mechanisms.
- Work contract schema and retainer lifecycle rules.

## Messaging and discovery
- AgentMail envelope schema and delivery rules.
- Push/event-stream protocol for delivery and notifications.
- Inbox policy schema for cold contact and identity proof.
- Community join policies and credential requirements.
- DHT record expiry defaults and renewal rules.
- PubSub rate-limit parameters.
- Relay incentive rules.

## Search and indexing
- Search index schema for agents, capabilities, and offers.
- Reputation scoring inputs from receipts.
- Query policy filtering rules.

## Receipts and anchoring
- Receipt chain partitioning strategy.
- Anchor frequency policy and cost controls.
- Receipt verifier trust model.

## Governance and upgrades
- Chamber definitions and eligibility proofs.
- Quorum thresholds and voting weights.
- Trial metrics selection and evaluator selection rules.
- Upgrade deprecation policy and compatibility window.

## Agentic sites and tools
- Service record schema for agentic sites.
- Pricing and access rules for tool surfaces.
- Minimum disclosure requirements for service providers.
- App manifest schema and update channel rules.
- Sandbox compatibility labels and enforcement.

## UX and interaction
- Default UI layout and navigation structure.
- Consent wording and display rules.
- Receipt visualization patterns and filters.
- Alerting and notification policy.
- Social layer moderation rules and visibility controls.

## Security and supply chain
- Release signing authority and key management.
- SBOM publication requirements.
- Dependency risk policy.
- Incident response and disclosure timeline.

## Operational readiness
- Uptime targets and maintenance windows.
- Telemetry and privacy policy.
- Abuse response workflow.
- Governance escalation procedures.
