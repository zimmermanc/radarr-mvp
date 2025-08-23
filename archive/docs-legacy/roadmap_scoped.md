# Roadmap — Torrent-Only (qBittorrent), No Usenet

## Milestones & deliverables
### M0 — Foundation & Observability
- Structured JSON logs, correlation IDs spanning search→download→import.
- Prometheus `/metrics`: counters (searches, grabs, imports ok/fail), gauges (queue len), histograms (search latency, import time).
- Compose e2e harness (app + Postgres + qBittorrent + fake Prowlarr + fake HDBits).

### M1 — Indexers (HDBits + Prowlarr/Torrent)
- Harden HDBits: category mapping, freeleech bias, backoff on 429/5xx.
- Prowlarr client (torrent-only): API key auth; caps/indexers sync; aggregated search; health/backoff.
- Selection policy: infohash de‑dupe; timeout racing; per-source weighting.
- UI: indexer health/backoff dashboard.

### M2 — Quality Engine
- Quality Profile CRUD + defaulting; Custom Formats registry and scoring; Cutoff & upgrade loop.
- Import acceptance rules; rename templates; **recycle bin**.
- Decision explainers with reasons in UI.

### M3 — Lists & Discovery
- Trakt device OAuth importer; IMDb & TMDb importers (torrent-only).
- Scheduled sync; provenance on each candidate; Discover UI with “why included”.

### M4 — Failure Handling & Blocklist
- Canonical failure taxonomy; per-release blocklist with TTL; exponential backoff; manual override.
- E2E fault‑injection suite (429, 5xx, stalled downloads, bad hash, missing files).

### M5 — Integrations & Notifications
- Plex refresh webhook after successful import; Discord/webhook test-send.

### M6 — API & SDKs
- OpenAPI spec; Swagger UI; generate TypeScript client consumed by web/.
