#!/usr/bin/env bash
Proof: CI artifact with sample logs.'


create_issue "#0.2 Prometheus /metrics + health endpoints" "backend,infra,perf" "M0 — Foundation & Observability" $'Expose counters/gauges; liveness/readiness.


Acceptance: curl /metrics non-empty; readiness gates start.
Proof: scrape dump.'


create_issue "#0.3 Docker compose e2e harness" "infra,test" "M0 — Foundation & Observability" $'Compose stack: app+PG+qBittorrent+fake-indexer+seed media; GA job stores logs.


Acceptance: make e2e green locally & CI.'


create_issue "#1.1 HDBits provider hardening" "indexer,backend,test" "M1 — Indexers (HDBits + Prowlarr)" $'Cred validation, category mapping, freeleech pref, throttle/backoff.


Acceptance: 429 triggers backoff; UI shows rejection reasons.'


create_issue "#1.2 Prowlarr API client & discovery" "indexer,backend,api,test" "M1 — Indexers (HDBits + Prowlarr)" $'Add Prowlarr connection; sync indexers; fetch releases via aggregated Torznab endpoints.


Acceptance: UI shows indexers; search returns releases.'


create_issue "#1.3 Multi-indexer selection policy" "indexer,backend,perf" "M1 — Indexers (HDBits + Prowlarr)" $'Timeout racing, dedupe, weighting; per-source toggle.


Acceptance: p95 < 1.5s; duplicates collapsed.'


create_issue "#1.4 Indexer health & backoff dashboard" "frontend,backend,indexer,ux" "M1 — Indexers (HDBits + Prowlarr)" $'Status with last error & countdown.


Acceptance: induced 429 → Degraded + countdown.'


create_issue "#2.1 Quality Profile CRUD (API+UI)" "backend,frontend,api,ux" "M2 — Quality Engine" $'Define qualities/res; CRUD + cutoff; default profile.


Acceptance: cutoff change triggers upgrade loop.'


create_issue "#2.2 Custom Formats scoring" "backend,api,test" "M2 — Quality Engine" $'CF store (regex/globs), scoring, explainers.


Acceptance: CF raises preferred release.'


create_issue "#2.3 Import acceptance + rename + recycle bin" "backend,frontend,ux" "M2 — Quality Engine" $'Accept/reject with reason; rename templates; soft-delete.


Acceptance: rename preview; rejected list shows reason.'


create_issue "#3.1 Trakt list importer" "backend,api,test" "M3 — Lists & Discovery" $'Device OAuth; scheduled sync; de-dupe; monitoring.


Acceptance: Trakt list creates monitored movies.'


create_issue "#3.2 IMDb list importer" "backend,api,test" "M3 — Lists & Discovery" $'API/HTML fallback; schedule; throttling.


Acceptance: IMDb list adds N with provenance.'


create_issue "#3.3 TMDb list importer + Discover UI" "backend,frontend,ux" "M3 — Lists & Discovery" $'TMDb list import; Discover page; explanations.


Acceptance: UI shows candidates with reasons.'


create_issue "#4.1 Failure taxonomy & reason codes" "backend,api,test" "M4 — Failure Handling & Blocklist" $'Canonical reasons (timeout/auth/bad hash/DMCA).


Acceptance: Failures tagged; visible in UI.'


create_issue "#4.2 Blocklist & retry/backoff" "backend,frontend" "M4 — Failure Handling & Blocklist" $'Per-release blocklist with TTL; backoff; manual override.


Acceptance: Blocklisted never retried within TTL; override works.'


create_issue "#5.1 Plex library refresh" "backend,api,ux" "M5 — Integrations & Notifications" $'Token config; refresh on import; test hook.


Acceptance: Import triggers refresh (mocked in CI).'


create_issue "#5.2 Discord/webhook notifications" "backend,api" "M5 — Integrations & Notifications" $'Generic webhooks; Discord formatter; test-send.


Acceptance: Test message success from settings.'


create_issue "#6.1 OpenAPI spec + Swagger UI + TS client" "api,docs" "M6 — API Compatibility & SDKs" $'OpenAPI authoring; Swagger UI; generate TS client; wire into web/.


Acceptance: UI compiles using generated types.'


create_issue "#6.2 Compatibility façade (subset)" "api,backend" "M6 — API Compatibility & SDKs" $'Provide minimal subset compatible with common Radarr endpoints.


Acceptance: cassette tests pass for subset.'


create_issue "#7.1 Cross-platform builds + Docker images" "infra,release" "M7 — Packaging & Release" $'Cross-compile, multi-arch images, release notes.


Acceptance: Tagged release publishes binaries & images.'


create_issue "#7.2 SBOM + cosign signing" "security,release" "M7 — Packaging & Release" $'SBOM generation and image signing; verify in CI.


Acceptance: cosign verify succeeds; SBOM attached.'
