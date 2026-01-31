# AgentNet Interoperability Notes (MCP + A2A)

This document captures how AgentNet can interoperate with existing agent
frameworks and protocols without diluting the core v0.1 design (CBOR canonical
schemas, pairing, receipts, economics, governance).

## 1) Model Context Protocol (MCP)

What it is:
- A client-server protocol for connecting models/agents to tools, resources, and
  prompts with structured inputs/outputs.

What works well:
- Clear separation of capabilities (tools/resources/prompts).
- JSON-RPC based messaging with multiple transports.
- Good developer ergonomics for tool providers.

Gaps vs AgentNet requirements:
- No native identity layer (DIDs), pairing, or receipts.
- No built-in economy (postage/bonds/escrow).
- No governance or community policy enforcement.
- Security relies heavily on host-side controls and user consent.

AgentNet inclusion plan:
- Treat MCP servers as ServiceRecord providers.
- Use MCP for tool execution inside the agent runtime only (local or remote).
- Record all MCP invocations as AgentNet receipts.
- Apply Policy Gate rules before any MCP tool call.

## 2) Agent2Agent (A2A)

What it is:
- A protocol for agent-to-agent communication over HTTP, with discovery via
  Agent Cards and support for task lifecycles, streaming, and async updates.

What works well:
- Task lifecycle modeling for long-running or multi-step work.
- Streaming updates and async delivery patterns.
- Simple HTTP+JSON-RPC transport that is easy to bridge to web systems.
- Agent Cards with name, description, url, capabilities, and skills; descriptions
  may use CommonMark/Markdown.

Gaps vs AgentNet requirements:
- No canonical CBOR schema layer.
- No native receipts or hash chaining.
- No economy or governance primitives.
- No pairing model for delegated authority.

AgentNet inclusion plan:
- Provide an AgentNet gateway that exposes a read-only A2A facade for
  web-facing interoperability.
- Map A2A Agent Cards to AgentNet AgentRecord/ServiceRecord (read-only).
- Map A2A tasks to AgentNet TaskOffer/TaskUpdate with receipts for every state
  transition.
- Ensure pairing and policy gate checks happen before any on-behalf-of action.

## 3) Markdown as agent-readable content

- AgentNet on-wire objects remain CBOR.
- Any human/agent-readable content SHOULD be Markdown (UTF-8).
- Structured machine-critical data stays in CBOR/JSON fields; Markdown is used
  for summaries, instructions, and explanations.

## 4) Interop principle

- MCP and A2A are complements, not substitutes.
- AgentNet remains the canonical, signed, economic layer.
- Bridges provide visibility and compatibility without weakening core security
  and accountability.
