# Claude Code Prompt Pack — Torrent-Only

## /plan Indexers (HDBits + Prowlarr/Torrent-only)
- Produce: architecture sketch; data models; backoff policy; selection (infohash de‑dupe, timeout racing, weighting); test plan; proof artifacts.
- Constraint: No Usenet. Prowlarr only queries torrent indexers.
- Exit: Offer 3 options + tradeoffs → wait for [[GO:OPTION N]].

## /plan Quality (Profiles/CF/Cutoff)
- DB schema; API; CF scoring examples; explainers; rename preview; recycle bin; unit+e2e tests.
- Exit gate [[GO:QUALITY]] after sample scoring table makes sense.

## /plan Failure (Taxonomy + Blocklist)
- Canonical reasons; blocklist with TTL; exponential backoff; override; UI surfacing; fault-injection test matrix.
- Exit gate [[GO:FAILURE]].

## /plan Lists (Trakt/IMDb/TMDb)
- Trakt device OAuth; IMDb & TMDb importers; scheduler; provenance model; Discover UI.
- Exit gate [[GO:LISTS]].

## /go
- Execute in tiny diffs; show git patches + test logs; stop on first red test.
