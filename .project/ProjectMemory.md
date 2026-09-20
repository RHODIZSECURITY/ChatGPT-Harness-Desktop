# RHODIZ Harness Desktop Project Memory

## Durable authority decisions

- Desktop is a client of Harness Core contract v1, never a second coding/runtime authority.
- Core retains Project/Session, signed workspace lease, capability, Landlock and Resource Governor authority.
- RHODIZ MCP Route owns transport/provider routing and provider/OAuth secrets, not coding authority.
- RHODIZ Memory MCP owns durable memory data, not coding authority.
- Route, Memory MCP and provider sidecars remain private to the managed WSL/Docker network by default.

## Durable Windows decisions

- Host shell: Tauri 2 with React/TypeScript.
- Host privilege boundary: narrow Rust broker with enumerated commands.
- Renderer gets no generic shell, PowerShell, WSL, Docker or process capability.
- Renderer does not persist backend bearer credentials.
- Desktop networking is loopback-only by default.
- Managed distro name for new work: `RHODIZ-Harness`.
- Docker/Compose runs inside the managed WSL2 distro; Harness containers never receive `docker.sock`.
- Runtime images/config require signed release manifests and exact digests; no floating `:latest`.
- Runtime updates require anti-rollback sequencing and bounded rollback.

## Source reuse decisions

- OpenHands is a coding-UI engineering source, not an alternate agent/runtime backend.
- Onyx is the primary Tauri/window/build engineering source.
- LibreChat is a chat/MCP/artifact UX engineering source, not an alternate provider/API backend.
- Every imported upstream module requires exact commit provenance, license record, local destination, modification summary, security review and tests.
- Source branches are never build inputs; exact commits are provenance anchors.

Pins:
- OpenHands `a0403035a20c91decadd011b907ee5b489f6788b`
- Onyx `e11c874019dbf04032cfc3476d82eba1d069a3d8`
- LibreChat `2452f499a86ae215146d988e9418a481616238ad`

## Quality decisions

- GitHub CI is certification only; defect discovery and fixing happen locally first.
- Test-Automator coverage is part of implementation, not a later phase.
- Native Windows claims require native Windows evidence; Linux cross-checks do not substitute for Windows E2E.
- Security advisories and warnings are recorded precisely and are not described as resolved without evidence.
