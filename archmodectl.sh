#!/usr/bin/env bash
# archmodectl.sh — Architecture Mode controller (ARR + Feature presets)
set -Eeuo pipefail
VERSION="2.0.0+arr+features"

# ---------- utils ----------
color() { printf "\033[%sm" "$1"; }
reset() { printf "\033[0m"; }
ok()    { color 0; color 32; printf "✅ %s\n" "$*"; reset; }
warn()  { color 1; color 33; printf "⚠️  %s\n" "$*"; reset; }
err()   { color 31; printf "❌ %s\n" "$*"; reset; }
info()  { color 36; printf "🔧 %s\n" "$*"; reset; }

require() { command -v "$1" >/dev/null 2>&1 || { err "Missing dependency: $1"; exit 1; }; }
timestamp() { date +"%Y-%m-%d %H:%M:%S"; }
ts_slug() { date +"%Y%m%d-%H%M%S"; }
slugify() { echo "$1" | tr '[:upper:]' '[:lower:]' | sed -E 's/[^a-z0-9]+/-/g;s/^-+|-+$//g'; }
ensure() { mkdir -p "$@"; }

ROOT="$(pwd)"
REPO="$(basename "$ROOT")"

STUDIO_URL_DEFAULT="https://github.com/arnaldo-delisio/claude-code-studio"
SUBAGENTS_URL_DEFAULT="https://github.com/VoltAgent/awesome-claude-code-subagents"

FORCE="${FORCE:-0}"

usage() {
  cat <<EOF
archmodectl v$VERSION

Usage:
  archmodectl init [--preset <name>] [flags*]
  archmodectl adopt
  archmodectl organize [--apply] [--archive-unused] [--show]
  archmodectl agents vendor|add <url> [--name NAME]|list
  archmodectl style install
  archmodectl mcp add <url> [--name NAME]
  archmodectl adr new|list|link
  archmodectl docs lint|ci|serve|build
  archmodectl diagrams build
  archmodectl guard ci
  archmodectl index|snapshot
  archmodectl status add <file.md>
  archmodectl house install [--solo] [--org ORG] [--owners ""] [--strict]
  archmodectl solo install|journal|hooks install|remove
  archmodectl go on|off|status

Presets:
  rust-service | rust-cli | go-service | py-api | python-lib | webapp-nextjs | data-workflows | infra-terraform
  # ARR service presets:
  arr-core-service | arr-plex-ingest | arr-metadata-tmdb | arr-graph-arango | arr-ml-reco | arr-indexers |
  arr-download-clients | arr-jobs | arr-observability | arr-admin-ui | arr-e2e-tests | arr-deploy
  # Feature presets:
  arr-feat-scoring | arr-feat-release-prediction | arr-feat-relationship-mapping | arr-feat-content-analysis

Flags for ARR core:
  --db postgres|arangodb|both
  --openapi
  --jobs redis|amqp|inline
  --vector-search
  --plex
  --tmdb
  --ci
  --obs
  --e2e

Env:
  FORCE=1  overwrite existing files

Examples:
  archmodectl init --preset arr-core-service --db both --openapi --jobs redis --plex --tmdb --obs --ci --e2e
  archmodectl init --preset arr-feat-relationship-mapping
EOF
}

write_docs() {
  ensure ".architecture/research" ".architecture/analysis" ".architecture/options" ".architecture/decisions" ".architecture/execution_logs" ".architecture/templates"
  ensure ".agents/custom" ".agents/workflows" ".agents/configs"
  ensure "vendor/studio" "vendor/subagents" "prompts" "scripts" "docs/architecture-mode" "context/mimic" "context/notes" ".claude" ".github/workflows"

  if [[ ! -f docs/architecture-mode/ARCHITECTURE_MODE.md ]]; then
    cat > docs/architecture-mode/ARCHITECTURE_MODE.md <<'EOF'
# Architecture Mode Guardrails (Solo)

**Planning Mode (default)** — research → analyze → questions → options → warnings → decision.  
**Execution Mode** — only after explicit "go".

Rules:
- Planning is read-only. No code changes until "go".
- Always do quick research + repo scan.
- Offer 4–8 simple options with risks/assumptions.
- Switch back to planning after each execution chunk.
EOF
  fi

  if [[ ! -f prompts/CLAUDE_KICKOFF.md ]]; then
    cat > prompts/CLAUDE_KICKOFF.md <<EOF
You are in **Architecture Mode (Solo)** for: $REPO

Start in **PLANNING MODE (read-only)**:
1) Research with WebSearch/WebFetch + scan the repo.
2) Propose **4–8 simple options** for the first improvement.
3) Ask clarifying questions and wait for **"go"** before code changes.

When I say **"go"**, execute only the chosen option. Then return to planning.
EOF
  fi

  if [[ ! -x scripts/claude-start.sh ]]; then
    cat > scripts/claude-start.sh <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo
echo "──────── Claude Kickoff Prompt (copy into Claude Code) ────────"
echo
cat prompts/CLAUDE_KICKOFF.md
echo
echo "────────────────────────── End ────────────────────────────────"
echo
EOF
    chmod +x scripts/claude-start.sh
  fi

  if [[ ! -x scripts/arch-status.sh ]]; then
    cat > scripts/arch-status.sh <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo "Architecture Mode status"
echo "------------------------"
echo "Generated: $(date +"%Y-%m-%d %H:%M:%S")"
echo "Repo: $(basename "$(git rev-parse --show-toplevel 2>/dev/null || echo ".")")"
echo "Planning dirs:"
find .architecture -maxdepth 1 -type d -print 2>/dev/null | sed 's/^/  - /'
echo "Agents:"
[ -d ".agents" ] && find .agents -maxdepth 2 -type d -print | sed 's/^/  - /'
echo "Vendor:"
[ -d "vendor" ] && find vendor -maxdepth 2 -type d -print | sed 's/^/  - /'
EOF
    chmod +x scripts/arch-status.sh
  fi

  if [[ ! -f .claude/config.json ]]; then
    cat > .claude/config.json <<'EOF'
{
  "architecture_mode": {
    "enabled": true,
    "default_mode": "PLANNING",
    "execution_triggers": ["g", "go"],
    "min_options": 4,
    "max_options": 8
  },
  "agents": {
    "enabled": true,
    "sources": [".agents/custom", ".agents/workflows", ".agents/configs", "vendor/studio", "vendor/subagents"]
  }
}
EOF
  fi
}

git_init_if_needed() {
  if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    git init -b main >/dev/null 2>&1 || git init >/dev/null 2>&1
    git config --local init.defaultBranch main || true
    ok "Initialized Git repository."
  fi
}

# ---------- ARR helpers ----------
_append_line() {
  local path="$1"; local line="$2"
  ensure "$(dirname "$path")"
  if [[ ! -f "$path" ]]; then
    echo "$line" > "$path"; info "created: $path"
  else
    if ! grep -Fqx "$line" "$path"; then
      echo "$line" >> "$path"; info "appended: $path"
    else warn "already has line: $path"; fi
  fi
}

_add_dep() {
  local cargo="$1"; local section="$2"; local dep="$3"
  ensure "$(dirname "$cargo")"
  [[ -f "$cargo" ]] || { err "Cargo.toml not found at $cargo"; return 1; }
  if grep -Fq "^\[$section\]" "$cargo" && grep -Fq "^$dep" "$cargo"; then
    warn "Cargo: $dep exists in [$section]"; return 0
  fi
  if ! grep -Fq "^\[$section\]" "$cargo"; then
    printf "\n[%s]\n%s\n" "$section" "$dep" >> "$cargo"
  else
    awk -v dep="$dep" '{print} END{print dep}' "$cargo" > "$cargo.tmp" && mv "$cargo.tmp" "$cargo"
  fi
  info "Cargo: added $dep under [$section]"
}

arr_add_feature_charter() { # slug title priority body_md
  local slug="$1" title="$2" prio="$3" body="$4"
  ensure ".architecture/features/$slug" ".architecture/research/$slug" ".architecture/analysis/$slug" ".architecture/options/$slug" ".architecture/decisions" ".claude/features"
  cat > ".architecture/features/$slug/charter.md" <<EOF
# Feature Charter: $title

- **Slug**: $slug
- **Priority**: $prio
- **Goal**: (fill in)
- **Non-goals**: (fill in)
- **Inputs/Outputs**: (fill in)
- **Integration Points**: (fill in)
- **Risks/Assumptions**: (fill in)
- **Success Criteria**: (fill in)

## Context
$body
EOF
  cat > ".claude/features/$slug.json" <<'EOF'
{
  "planning_model": "opus-4.1",
  "execution_model": "sonnet-4",
  "agents": {
    "research": ["web.search", "web.fetch"],
    "analysis": ["repo.scan", "grep", "read"],
    "code": ["rust", "shell"]
  }
}
EOF
  ok "Created feature charter + models for '$slug'"
}

arr_scaffold_workspace() {
  cat > Cargo.toml <<'EOF'
[workspace]
members = ["service","domain","infra"]
resolver = "2"
EOF

  ensure "service/src" "domain/src" "infra/src"

  cat > service/Cargo.toml <<'EOF'
[package]
name = "service"
version = "0.1.0"
edition = "2021"

[dependencies]
axum = "0.7"
tower = "0.5"
tower-http = { version = "0.5", features = ["trace","cors","compression-full"] }
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter","fmt"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
thiserror = "1"
EOF

  cat > service/src/main.rs <<'EOF'
use axum::{routing::get, Router};
use std::net::SocketAddr;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let app = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/readyz", get(|| async { "ready" }));

    let addr = SocketAddr::from(([0,0,0,0], 3000));
    tracing::info!("listening on {}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app)
        .await.unwrap();
}
EOF

  cat > domain/Cargo.toml <<'EOF'
[package]
name = "domain"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1", features = ["derive"] }
thiserror = "1"
EOF

  cat > domain/src/lib.rs <<'EOF'
pub mod movies {
    #[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
    pub struct Movie { pub id: i64, pub title: String }
}
EOF

  cat > infra/Cargo.toml <<'EOF'
[package]
name = "infra"
version = "0.1.0"
edition = "2021"

[dependencies]
tracing = "0.1"
anyhow = "1"
domain = { path = "../domain" }
EOF

  cat > infra/src/lib.rs <<'EOF'
pub mod hello {
    pub fn ping() -> &'static str { "pong" }
}
EOF

  cat > .gitignore <<'EOF'
/target
**/*.rs.bk
.env
.idea
.vscode
**/.DS_Store
EOF

  cat > .env.example <<'EOF'
RUST_LOG=info
PORT=3000
EOF

  cat > README.md <<'EOF'
# arr preset workspace
Quickstart:
1) cp .env.example .env
2) cargo run -p service
EOF
}

arr_add_postgres() {
  _add_dep "infra/Cargo.toml" "dependencies" 'sqlx = { version = "0.7", features = ["postgres","runtime-tokio","macros","uuid","chrono","migrate"] }'
  _append_line ".env.example" "DATABASE_URL=postgres://postgres:postgres@localhost:5432/app"
  if [[ ! -f docker-compose.yml ]]; then
    cat > docker-compose.yml <<'EOF'
version: "3.9"
services:
  db:
    image: postgres:16
    environment:
      POSTGRES_PASSWORD: postgres
      POSTGRES_DB: app
    ports: ["5432:5432"]
    healthcheck:
      test: ["CMD-SHELL","pg_isready -U postgres"]
      interval: 5s
      timeout: 3s
      retries: 20
EOF
  else
    if ! grep -q 'image: postgres' docker-compose.yml 2>/dev/null; then
      cat >> docker-compose.yml <<'EOF'
  db:
    image: postgres:16
    environment:
      POSTGRES_PASSWORD: postgres
      POSTGRES_DB: app
    ports: ["5432:5432"]
    healthcheck:
      test: ["CMD-SHELL","pg_isready -U postgres"]
      interval: 5s
      timeout: 3s
      retries: 20
EOF
    fi
  fi
  ensure "migrations"
  cat > migrations/20250101000000_init.sql <<'EOF'
-- sqlx migration
CREATE TABLE IF NOT EXISTS movies (
  id BIGSERIAL PRIMARY KEY,
  title TEXT NOT NULL
);
EOF
}

arr_add_openapi() {
  _add_dep "service/Cargo.toml" "dependencies" 'utoipa = "5"'
  _add_dep "service/Cargo.toml" "dependencies" 'utoipa-axum = "0.1"'
  if ! grep -q "utoipa" "service/src/main.rs" 2>/dev/null; then
    cat >> "service/src/main.rs" <<'EOF'

// --- OpenAPI (minimal sample) ---
use utoipa::OpenApi;
#[derive(OpenApi)]
#[openapi(paths(), components(schemas()))]
struct ApiDoc;
EOF
    info "patched: service/src/main.rs (+OpenAPI stub)"
  fi
}

arr_add_jobs() {
  local kind="${1:-redis}"
  _add_dep "service/Cargo.toml" "dependencies" 'apalis = { version = "0.6", default-features = false, features = ["cron"] }'
  if [[ "$kind" == "redis" ]]; then
    _add_dep "service/Cargo.toml" "dependencies" 'apalis-redis = "0.10"'
    _append_line ".env.example" "REDIS_URL=redis://127.0.0.1:6379"
    if [[ -f "docker-compose.yml" ]]; then
      if ! grep -q "redis:" docker-compose.yml; then
        cat >> docker-compose.yml <<'EOF'
  redis:
    image: redis:7
    ports: ["6379:6379"]
EOF
        info "patched: docker-compose.yml (+redis)"
      fi
    else
      cat > docker-compose.yml <<'EOF'
version: "3.9"
services:
  redis:
    image: redis:7
    ports: ["6379:6379"]
EOF
    fi
  elif [[ "$kind" == "amqp" ]]; then
    _add_dep "service/Cargo.toml" "dependencies" 'apalis-amqp = "0.5"'
    _append_line ".env.example" "AMQP_URL=amqp://guest:guest@127.0.0.1:5672/%2f"
  fi
  cat > service/src/jobs.rs <<'EOF'
pub async fn start_workers(){ /* TODO: wire apalis */ }
EOF
  if ! grep -q "mod jobs" service/src/main.rs 2>/dev/null; then
    sed -i.bak '1s;^;mod jobs;\n;' service/src/main.rs || true
  fi
}

arr_add_arangodb() {
  _add_dep "infra/Cargo.toml" "dependencies" 'arangors = "0.5"'
  _append_line ".env.example" "ARANGO_URL=http://127.0.0.1:8529"
  _append_line ".env.example" "ARANGO_USER=root"
  _append_line ".env.example" "ARANGO_PASS=password"
  cat > infra/src/arangodb.rs <<'EOF'
use arangors::{Connection, ClientError};
pub type Conn = arangors::connection::GenericConnection<arangors::client::reqwest::ReqwestClient>;
pub async fn connect(url:&str,user:&str,pass:&str)->Result<Conn,ClientError>{
    Connection::establish_basic_auth(url,user,pass).await
}
EOF
  if [[ -f "docker-compose.yml" ]]; then
cat >> docker-compose.yml <<'EOF'
  arangodb:
    image: arangodb:3.11
    environment:
      ARANGO_ROOT_PASSWORD: password
    ports: ["8529:8529"]
EOF
    info "patched: docker-compose.yml (+arangodb)"
  else
    cat > docker-compose.yml <<'EOF'
version: "3.9"
services:
  arangodb:
    image: arangodb:3.11
    environment:
      ARANGO_ROOT_PASSWORD: password
    ports: ["8529:8529"]
EOF
  fi
}

arr_add_vector_search() {
  ensure "infra/aql"
  cat > infra/aql/create_view_movies.aql <<'EOF'
/* Example ArangoSearch View with vector + text fields (manual run) */
// CREATE VIEW movies_search OPTIONS { "links": { "movies": { "includeAllFields": true } } };
// NOTE: For vectors: use 3.11+ enterprise features or model vectors as arrays and compute similarity in app layer.
EOF
}

arr_add_plex() {
  _append_line ".env.example" "PLEX_URL=http://127.0.0.1:32400"
  _append_line ".env.example" "PLEX_TOKEN=changeme"
  ensure "service/src/routes"
  cat > service/src/routes/plex.rs <<'EOF'
use axum::{routing::post, Router, extract::RawBody};
pub fn routes() -> Router {
    Router::new().route("/webhooks/plex", post(receive))
}
async fn receive(_body: RawBody) -> &'static str {
    // TODO: verify signature, parse payload, enqueue a job
    "ok"
}
EOF
  if ! grep -q 'routes::plex' service/src/main.rs 2>/dev/null; then
    sed -i.bak '1s;^;mod routes;\n;' service/src/main.rs || true
    awk '/Router::new\(\)/ && !done {print; print "        .merge(routes::plex::routes())"; done=1; next}1' service/src/main.rs > service/src/main.rs.tmp && mv service/src/main.rs.tmp service/src/main.rs
    info "patched: service/src/main.rs (+/webhooks/plex)"
  fi
}

arr_add_tmdb() {
  _append_line ".env.example" "TMDB_API_KEY=changeme"
  _add_dep "infra/Cargo.toml" "dependencies" 'reqwest = { version = "0.12", features = ["json","gzip","brotli","deflate","rustls-tls"] }'
  cat > infra/src/tmdb.rs <<'EOF'
use anyhow::Result;
pub async fn fetch_movie(_id:u32)->Result<()>{
    // TODO: call TMDb v3 using reqwest
    Ok(())
}
EOF
}

arr_add_observability() {
  _add_dep "service/Cargo.toml" "dependencies" 'metrics = "0.22"'
  _add_dep "service/Cargo.toml" "dependencies" 'metrics-exporter-prometheus = "0.14"'
  if ! grep -q "metrics_exporter_prometheus" service/src/main.rs 2>/dev/null; then
    cat >> service/src/main.rs <<'EOF'

// --- Prometheus exporter (basic) ---
use metrics_exporter_prometheus::PrometheusBuilder;
fn init_metrics() {
    let _ = PrometheusBuilder::new().install();
}
EOF
    sed -i.bak 's/tracing_subscriber::fmt().*/tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).init();\n    init_metrics();/' service/src/main.rs || true
    info "patched: service/src/main.rs (+metrics exporter)"
  fi
}

arr_add_ci() {
  ensure ".github/workflows"
  cat > .github/workflows/ci.yml <<'EOF'
name: CI
on:
  push: { branches: ["**"] }
  pull_request:
jobs:
  rust:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
        with: { toolchain: stable, profile: minimal, override: true }
      - run: sudo apt-get update && sudo apt-get install -y libssl-dev pkg-config
      - run: cargo fmt --all -- --check
      - run: cargo clippy --all-targets -- -D warnings
      - run: cargo test --all --all-features
EOF
}

arr_add_e2e() {
  ensure "tests"
  _add_dep "service/Cargo.toml" "dev-dependencies" 'testcontainers = "0.19"'
  cat > tests/smoke.rs <<'EOF'
#[test] fn smoke(){ assert_eq!(2+2,4); }
EOF
}

arr_add_indexers_stub() {
  ensure "infra/src/indexers"
  cat > infra/src/indexers/mod.rs <<'EOF'
pub trait Indexer { fn name(&self)->&str; } // TODO: add Torznab/Prowlarr adapters
EOF
}

arr_add_download_clients_stub() {
  ensure "infra/src/download"
  cat > infra/src/download/mod.rs <<'EOF'
pub trait DownloadClient { fn name(&self)->&str; } // TODO: add qBittorrent/Transmission adapters
EOF
}

arr_add_admin_ui() {
  ensure "admin/ui"
  cat > admin/ui/README.md <<'EOF'
# Admin UI
This is a placeholder for a future React/Vite app.
EOF
}

arr_add_deploy() {
  ensure "deploy/helm/chart/templates"
  cat > deploy/helm/chart/Chart.yaml <<'EOF'
apiVersion: v2
name: arr-service
version: 0.1.0
type: application
EOF
  cat > deploy/helm/chart/values.yaml <<'EOF'
replicaCount: 1
image:
  repository: ghcr.io/you/arr-service
  tag: latest
service:
  port: 3000
EOF
  cat > deploy/helm/chart/templates/deployment.yaml <<'EOF'
{{- /* minimal template */ -}}
EOF
}

preset_scaffold() {
  local preset="${1:-}"; shift || true
  case "$preset" in
    rust-service)
      ensure src
      cat > Cargo.toml <<'EOF'
[package]
name = "service"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["full"] }
axum = "0.7"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["fmt","env-filter"] }
EOF
      cat > src/main.rs <<'EOF'
use axum::{routing::get, Router};
use std::net::SocketAddr;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).init();
    let app = Router::new().route("/", get(|| async {"ok"}));
    let addr = SocketAddr::from(([127,0,0,1], 3000));
    tracing::info!("listening on {}", addr);
    axum::Server::bind(&addr).serve(app.into_make_service()).await.unwrap();
}
EOF
      ;;
    rust-cli)
      ensure src
      cat > Cargo.toml <<'EOF'
[package]
name = "cli"
version = "0.1.0"
edition = "2021"

[dependencies]
clap = { version = "4", features = ["derive"] }
anyhow = "1"
EOF
      cat > src/main.rs <<'EOF'
use clap::Parser;
#[derive(Parser)]
struct Args { #[arg(short, long, default_value_t = 1)] times: u8 }
fn main() { for _ in 0..Args::parse().times { println!("ok"); } }
EOF
      ;;
    go-service)
      cat > go.mod <<'EOF'
module service

go 1.22
EOF
      ensure cmd/service
      cat > cmd/service/main.go <<'EOF'
package main

import (
  "fmt"
  "log"
  "net/http"
)

func main() {
  http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) { fmt.Fprintln(w, "ok") })
  log.Println("listening on :3000")
  log.Fatal(http.ListenAndServe(":3000", nil))
}
EOF
      ;;
    py-api)
      cat > pyproject.toml <<'EOF'
[project]
name = "api"
version = "0.1.0"
dependencies = ["fastapi","uvicorn[standard]","pydantic"]
EOF
      ensure app
      cat > app/main.py <<'EOF'
from fastapi import FastAPI
app = FastAPI()

@app.get("/")
def read_root():
    return {"status": "ok"}
EOF
      ;;
    python-lib)
      cat > pyproject.toml <<'EOF'
[project]
name = "lib"
version = "0.1.0"
readme = "README.md"
requires-python = ">=3.10"
dependencies = []
[project.urls]
homepage = "https://example.com"
EOF
      ensure lib tests
      cat > lib/__init__.py <<'EOF'
__all__ = ["add"]
def add(a, b): return a + b
EOF
      cat > tests/test_add.py <<'EOF'
from lib import add
def test_add():
    assert add(1,2) == 3
EOF
      ;;
    webapp-nextjs)
      cat > package.json <<'EOF'
{
  "name": "webapp",
  "private": true,
  "scripts": {
    "dev": "next dev",
    "build": "next build",
    "start": "next start"
  },
  "dependencies": {
    "next": "14",
    "react": "18",
    "react-dom": "18"
  }
}
EOF
      ensure app
      cat > app/page.tsx <<'EOF'
export default function Home() {
  return <main style={{padding: 24}}><h1>ok</h1></main>
}
EOF
      ;;
    data-workflows)
      cat > pyproject.toml <<'EOF'
[project]
name = "data-workflows"
version = "0.1.0"
dependencies = ["duckdb","polars","pyarrow"]
EOF
      ensure workflows
      echo "# place your .sql/.py workflows here" > workflows/README.md
      ;;
    infra-terraform)
      ensure infra
      cat > infra/main.tf <<'EOF'
terraform {
  required_version = ">= 1.6.0"
  required_providers { aws = { source = "hashicorp/aws" version = "~> 5.0" } }
}
provider "aws" { region = var.aws_region }
variable "aws_region" { default = "us-east-1" }
EOF
      ;;

    # ---------- ARR presets ----------
    arr-core-service)
      # Parse flags
      local db="postgres"; local openapi="0"; local jobs=""; local vector="0"
      local plex="0"; local tmdb="0"; local ci="0"; local obs="0"; local e2e="0"
      while [[ $# -gt 0 ]]; do
        case "$1" in
          --db) db="${2:-postgres}"; shift 2;;
          --openapi) openapi="1"; shift;;
          --jobs) jobs="${2:-redis}"; shift 2;;
          --vector-search) vector="1"; shift;;
          --plex) plex="1"; shift;;
          --tmdb) tmdb="1"; shift;;
          --ci) ci="1"; shift;;
          --obs) obs="1"; shift;;
          --e2e) e2e="1"; shift;;
          *) warn "unknown flag for arr-core-service: $1"; shift;;
        esac
      done
      arr_scaffold_workspace
      case "$db" in
        postgres) arr_add_postgres ;;
        arangodb) arr_add_arangodb ;;
        both) arr_add_postgres; arr_add_arangodb ;;
        *) warn "unknown --db '$db'";;
      esac
      [[ "$openapi" == "1" ]] && arr_add_openapi
      [[ -n "$jobs" ]] && arr_add_jobs "$jobs"
      [[ "$vector" == "1" ]] && arr_add_vector_search
      [[ "$plex" == "1" ]] && arr_add_plex
      [[ "$tmdb" == "1" ]] && arr_add_tmdb
      [[ "$obs"  == "1" ]] && arr_add_observability
      [[ "$ci"   == "1" ]] && arr_add_ci
      [[ "$e2e"  == "1" ]] && arr_add_e2e
      ;;
    arr-plex-ingest)
      preset_scaffold arr-core-service --db postgres --openapi --jobs redis --plex --obs --e2e
      ;;
    arr-metadata-tmdb)
      preset_scaffold arr-core-service --db postgres --openapi --jobs redis --tmdb --obs --e2e
      ;;
    arr-graph-arango)
      preset_scaffold arr-core-service --db arangodb --openapi --vector-search --obs --e2e
      ;;
    arr-ml-reco)
      preset_scaffold arr-core-service --db both --openapi --vector-search --obs --e2e
      ;;
    arr-indexers)
      preset_scaffold arr-core-service --db postgres --openapi --obs --e2e
      arr_add_indexers_stub
      ;;
    arr-download-clients)
      preset_scaffold arr-core-service --db postgres --openapi --obs --e2e
      arr_add_download_clients_stub
      ;;
    arr-jobs)
      preset_scaffold arr-core-service --db postgres --openapi --jobs redis --obs --e2e
      ;;
    arr-observability)
      preset_scaffold arr-core-service --db postgres --openapi --obs
      ;;
    arr-admin-ui)
      preset_scaffold arr-core-service --db postgres --openapi --obs
      arr_add_admin_ui
      ;;
    arr-e2e-tests)
      arr_scaffold_workspace
      arr_add_e2e
      ;;
    arr-deploy)
      arr_scaffold_workspace
      arr_add_deploy
      ;;

    # ---------- Feature presets ----------
    arr-feat-scoring)
      preset_scaffold arr-core-service --db both --openapi --jobs redis --tmdb --obs --e2e
      arr_add_feature_charter "scoring" "Recommendation Scoring Engine" "P2" \
"**Scoring factors:** graph-based similarity (actors/directors), content similarity (plot), user preference, external/trending, temporal/recency.

Deliver P50 <100ms; batch/offline refreshing via jobs."
      ;;
    arr-feat-release-prediction)
      preset_scaffold arr-core-service --db postgres --openapi --jobs redis --tmdb --obs --e2e
      arr_add_feature_charter "release-prediction" "Release Prediction" "P4" \
"Predict: theatrical→digital timeline, streaming availability windows, and expected quality of releases. Provide explainable outputs and confidence."
      ;;
    arr-feat-relationship-mapping)
      preset_scaffold arr-core-service --db arangodb --openapi --vector-search --obs --e2e
      arr_add_feature_charter "relationship-mapping" "Movie Relationship Mapping" "P2" \
"Graph structure: Movies↔Actors (ACTED_IN), Movies↔Directors (DIRECTED), Movies↔Genres (BELONGS_TO), Movies↔Movies (SIMILAR_TO).
Features: connection analysis, genre clustering, franchise detection, collaboration networks."
      ;;
    arr-feat-content-analysis)
      preset_scaffold arr-core-service --db postgres --openapi --jobs redis --tmdb --obs --e2e
      arr_add_feature_charter "content-analysis" "ML-Powered Content Analysis" "P3" \
"Poster mood/theme cues, plot-summary NLP, review sentiment, genre auto-classification. Start with service stubs; consider vector/embeddings later."
      ;;

    ""|*)
      ;;
  esac
}

cmd_init() {
  require git
  local PRESET=""
  local -a PRESET_ARGS=()
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --preset) PRESET="$2"; shift 2;;
      *) PRESET_ARGS+=("$1"); shift;;
    esac
  done
  read -r -p "Project name [$REPO]: " NAME; NAME="${NAME:-$REPO}"
  local SLUG; SLUG="$(slugify "$NAME")"
  read -r -p "Directory [./$SLUG]: " DIR; DIR="${DIR:-./$SLUG}"
  read -r -p "Description [Research-first dev with Architecture Mode]: " DESC; DESC="${DESC:-Research-first dev with Architecture Mode}"
  mkdir -p "$DIR" && cd "$DIR"
  git_init_if_needed
  write_docs
  preset_scaffold "$PRESET" "${PRESET_ARGS[@]}"
  cat > README.md <<EOF
# $NAME

$DESC

## Quickstart
\`\`\`bash
./scripts/claude-start.sh
./scripts/arch-status.sh
\`\`\`

## Next steps
\`\`\`bash
./archmodectl.sh agents vendor
./archmodectl.sh style install
./archmodectl.sh solo install
\`\`\`
EOF
  git add . >/dev/null 2>&1 || true
  git commit -m "chore: bootstrap Architecture Mode ($(timestamp)) [preset: ${PRESET:-none}]" >/dev/null 2>&1 || true
  ok "New project created at: $DIR (preset: ${PRESET:-none})"
}

cmd_adopt() {
  require git
  git_init_if_needed
  write_docs
  ensure .architecture/analysis
  {
    echo "# Repository Inventory"
    echo "- Generated: $(timestamp)"
    compgen -G "package.json" >/dev/null && echo "- Node: package.json"
    compgen -G "pyproject.toml" >/dev/null && echo "- Python: pyproject.toml"
    compgen -G "Cargo.toml" >/dev/null && echo "- Rust: Cargo.toml"
    compgen -G "go.mod" >/dev/null && echo "- Go: go.mod"
  } > .architecture/analysis/inventory.md
  git add . >/dev/null 2>&1 || true
  git commit -m "chore: adopt Architecture Mode ($(timestamp))" >/dev/null 2>&1 || true
  ok "Adopted current repo into Architecture Mode."
}

cmd_style_install() {
  ensure ".claude/output-styles"
  local file=".claude/output-styles/Architecture Mode.md"
  cat > "$file" <<'EOF'
---
name: Architecture Mode
description: >
  Research-first development with structured planning phases and gated execution.
  Planning is read-only and execution requires an explicit "go" trigger.
---

# Architecture Mode — Output Style

**Non-negotiables**
- Start in PLANNING mode (read-only).
- Perform external research & local analysis before proposing code changes.
- Present 4–8 simple options with explicit risks and assumptions.
- Ask clarifying questions when constraints are unclear.
- Enter EXECUTION only after an explicit "go".
EOF
  ok "Installed Output Style at: $file"
  echo "In Claude Code, run: /output-style \"Architecture Mode\""
}

cmd_agents_vendor() {
  local studio="${1:-$STUDIO_URL_DEFAULT}"
  local subs="${2:-$SUBAGENTS_URL_DEFAULT}"
  ensure vendor
  if [[ ! -d vendor/studio/.git ]]; then
    git clone --depth 1 "$studio" vendor/studio || warn "Clone failed: $studio"
  else
    (cd vendor/studio && git pull --ff-only || true)
  fi
  if [[ ! -d vendor/subagents/.git ]]; then
    git clone --depth 1 "$subs" vendor/subagents || warn "Clone failed: $subs"
  else
    (cd vendor/subagents && git pull --ff-only || true)
  fi
  {
    echo "# vendor.lock — records the commits vendored"
    [ -d vendor/studio/.git ] && (cd vendor/studio && printf "studio %s\n" "$(git rev-parse --short HEAD 2>/dev/null || echo '?')")
    [ -d vendor/subagents/.git ] && (cd vendor/subagents && printf "subagents %s\n" "$(git rev-parse --short HEAD 2>/dev/null || echo '?')")
  } > vendor/vendor.lock || true
  ok "Vendored default agent repos."
}

cmd_agents_add() {
  if [[ $# -lt 1 ]]; then err "Usage: archmodectl agents add <url> [--name NAME]"; exit 1; fi
  local url="$1"; shift
  local name=""
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --name) name="$2"; shift 2;;
      *) err "Unknown arg: $1"; exit 1;;
    esac
  done
  ensure vendor
  local repo_name; repo_name="$(basename -s .git "$url")"
  [[ -n "$name" ]] && repo_name="$name"
  local dest="vendor/$repo_name"
  if [[ -d "$dest/.git" ]]; then
    warn "$dest already exists; pulling latest"
    (cd "$dest" && git pull --ff-only || true)
  else
    git clone --depth 1 "$url" "$dest" || { err "Clone failed: $url"; exit 1; }
  fi
  ok "Added agent repo at $dest"
}

scan_agent_files() {
  find "$1" -type f \( -iname "*agent*.yml" -o -iname "*agent*.yaml" -o -iname "*workflow*.yml" -o -iname "*workflow*.yaml" -o -iname "*.json" \) 2>/dev/null
}

cmd_agents_list() {
  local total=0
  echo "Sources scanned:"
  for p in ".agents" "vendor"; do
    [[ -d "$p" ]] || continue
    echo " - $p"
    while read -r f; do
      [[ -n "$f" ]] || continue
      total=$((total+1))
    done < <(scan_agent_files "$p")
  done
  echo
  echo "Approx agent/workflow config files discovered: $total"
  echo "(Heuristic; runtime agents depend on your Claude Code setup.)"
}

cmd_mcp_add() {
  if [[ $# -lt 1 ]]; then err "Usage: archmodectl mcp add <url> [--name NAME]"; exit 1; fi
  local url="$1"; shift
  local name=""
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --name) name="$2"; shift 2;;
      *) err "Unknown arg: $1"; exit 1;;
    esac
  done
  [[ -z "$name" ]] && name="$(basename "$url")"
  ensure ".claude"
  local file=".claude/mcp.json"
  [[ -f "$file" ]] || echo "[]" > "$file"
  if grep -q "\"$url\"" "$file"; then
    warn "MCP server already present: $url"
  else
    local tmp; tmp="$(mktemp)"
    awk -v u="$url" -v n="$name" '
      BEGIN{added=0}
      {
        if ($0 ~ /\]$/ && added==0) {
          sub(/\]$/, "", $0);
          if (length($0) > 1) {
            print $0 ", {\"name\":\"" n "\",\"url\":\"" u "\"}]"
          } else {
            print "[{\"name\":\"" n "\",\"url\":\"" u "\"}]"
          }
          added=1
        } else print $0
      }' "$file" > "$tmp"
    mv "$tmp" "$file"
    ok "Added MCP server: $name -> $url"
  fi
}

adr_dir=".architecture/decisions"
adr_template() {
cat <<'EOF'
# {TITLE}

* Status: proposed
* Deciders: <list>
* Date: {DATE}

## Context and Problem Statement
<what is the issue? who is impacted?>

## Decision Drivers
- <driver 1>
- <driver 2>

## Considered Options
- <option A>
- <option B>

## Decision Outcome
Chosen option: "<option>", because <reasons>.

## Pros and Cons of the Options
### <option A>
- Good, because ...
- Bad, because ...

### <option B>
- Good, because ...
- Bad, because ...

## Links
- Proposal / Options: ../options/
- Status / Tracking: ../../docs/architecture-mode/STATUS.md
- Related ADRs:
EOF
}

cmd_adr_new() {
  if [[ $# -lt 1 ]]; then err "Usage: archmodectl adr new \"Decision title\""; exit 1; fi
  ensure "$adr_dir"
  local title="$*"
  local date="$(date +%Y-%m-%d)"
  local slug="$(slugify "$title")"
  local file="$adr_dir/${date}-adr-${slug}.md"
  if [[ -f "$file" ]]; then err "ADR exists: $file"; exit 1; fi
  adr_template | sed -e "s/{TITLE}/$title/g" -e "s/{DATE}/$date/g" > "$file"
  ok "Created ADR: $file"
  cmd_index >/dev/null || true
}

cmd_adr_list() {
  ensure "$adr_dir"
  ls -1 "$adr_dir"/*.md 2>/dev/null | sed 's/^/ - /' || echo "No ADRs found."
}

cmd_adr_link() {
  if [[ $# -lt 2 ]]; then err "Usage: archmodectl adr link <adr-file> <PR#>"; exit 1; fi
  local file="$1"; local pr="$2"
  [[ -f "$file" ]] || { err "ADR not found: $file"; exit 1; }
  echo "- PR: #$pr" >> "$file"
  ok "Linked PR #$pr in $file"
}

cmd_docs_lint() {
  [[ -f .markdownlint.jsonc ]] || cat > .markdownlint.jsonc <<'EOF'
{
  "default": true,
  "MD013": false,
  "MD033": { "allowed_elements": ["br","sub","sup"] }
}
EOF
  ensure ".vale/styles/ArchMode"
  [[ -f .vale.ini ]] || cat > .vale.ini <<'EOF'
StylesPath = .vale/styles
MinAlertLevel = suggestion

[*]
BasedOnStyles = Vale, ArchMode
EOF
  [[ -f .vale/styles/ArchMode/Terms.yml ]] || cat > .vale/styles/ArchMode/Terms.yml <<'EOF'
extends: substitution
message: "Prefer '%s'"
level: suggestion
ignorecase: true
swap:
  utilize: use
  leverage: use
EOF

  if command -v markdownlint >/dev/null 2>&1; then
    markdownlint "**/*.md" "**/*.mdx" || true
  else
    warn "markdownlint not installed. npm i -g markdownlint-cli2"
  fi
  if command -v vale >/dev/null 2>&1; then
    vale . || true
  else
    warn "Vale not installed. brew install vale"
  fi
  ok "Docs lint scaffolding complete."
}

cmd_docs_ci() {
  ensure ".github/workflows"
  cat > .github/workflows/docs-lint.yml <<'EOF'
name: Docs Lint
on:
  pull_request:
    paths:
      - "**/*.md"
      - "**/*.mdx"
jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          node-version: "20"
      - name: Install markdownlint-cli2
        run: npm i -g markdownlint-cli2
      - name: Run markdownlint-cli2
        run: markdownlint-cli2 "**/*.md" "**/*.mdx"
      - name: Install Vale
        run: |
          curl -fsSL https://raw.githubusercontent.com/errata-ai/vale/master/scripts/install.sh | sh
          echo "$HOME/.local/bin" >> $GITHUB_PATH
      - name: Run Vale
        run: vale .
EOF
  ok "Created CI workflow: .github/workflows/docs-lint.yml"
}

cmd_docs_serve_build() {
  local action="$1"
  [[ -f mkdocs.yml ]] || cat > mkdocs.yml <<EOF
site_name: $REPO Docs
theme:
  name: material
nav:
  - Home: index.md
  - Architecture:
      - Guardrails: docs/architecture-mode/ARCHITECTURE_MODE.md
      - Status: docs/architecture-mode/STATUS.md
      - Index: docs/architecture-mode/INDEX.md
EOF
  [[ -f index.md ]] || echo "# $REPO" > index.md
  if ! command -v mkdocs >/dev/null 2>&1; then
    warn "mkdocs not installed. pip install mkdocs-material"
    return 0
  fi
  case "$action" in
    serve) mkdocs serve ;;
    build) mkdocs build ;;
  esac
}

cmd_diagrams_build() {
  if ! command -v mmdc >/dev/null 2>&1; then
    warn "mermaid-cli (mmdc) not found. npm i -g @mermaid-js/mermaid-cli"
    return 0
  fi
  ensure ".architecture/diagrams"
  local count=0
  while IFS= read -r -d '' f; do
    awk '
      BEGIN{inm=0; idx=0}
      /^```mermaid/ {inm=1; next}
      /^```/ && inm==1 {inm=0; idx++}
      inm==1 {print > sprintf(".architecture/diagrams/%s-%d.mmd", envfile, idx)}
    ' envfile="$(basename "${f%.*}")" "$f"
  done < <(find . -type f \( -iname "*.md" -o -iname "*.mdx" \) -print0)
  for m in .architecture/diagrams/*.mmd; do
    [[ -f "$m" ]] || continue
    mmdc -i "$m" -o "${m%.mmd}.svg" || true
    count=$((count+1))
  done
  ok "Rendered $count Mermaid diagram(s)."
}

cmd_guard_ci() {
  ensure ".github/workflows"
  cat > .github/workflows/archmode-guard.yml <<'EOF'
name: Architecture Mode Guard
on:
  pull_request:
    types: [opened, edited, synchronize, labeled, unlabeled]

jobs:
  guard:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Check labels/body for go/ADR
        env:
          PR_TITLE: ${{ github.event.pull_request.title }}
          PR_BODY: ${{ github.event.pull_request.body }}
          PR_LABELS: ${{ toJson(github.event.pull_request.labels) }}
        run: |
          echo "Labels: $PR_LABELS"
          HAS_GO=$(echo "$PR_LABELS" | grep -Eo '"name":\s*"go"' || true)
          HAS_ADR=$(printf "%s\n%s" "$PR_TITLE" "$PR_BODY" | grep -E 'ADR[- ]?[0-9]+' || true)
          CHANGED_ADR=$(git diff --name-only HEAD^ HEAD 2>/dev/null | grep -E '^\.?\.?/.architecture/decisions/.*\.md$' || true)
          if [[ -z "$HAS_GO" && -z "$HAS_ADR" && -z "$CHANGED_ADR" ]]; then
            echo "::error ::PR must have label 'go' OR reference an ADR (e.g., ADR-001) OR include an ADR change."
            exit 1
          fi
          echo "Guard passed."
EOF
  ok "Created guard workflow: .github/workflows/archmode-guard.yml"
}

cmd_organize() {
  local APPLY=0 ARCH=0 SHOW=0
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --apply) APPLY=1 ;;
      --archive-unused) ARCH=1 ;;
      --show) SHOW=1 ;;
      *) err "Unknown arg: $1"; exit 1 ;;
    esac
    shift
  done

  ensure ".architecture/analysis" "docs/architecture-mode" "context/notes" ".architecture/execution_logs/status-archive"

  local TS; TS="$(ts_slug)"
  local MANIFEST=".architecture/analysis/organize-manifest-$TS.csv"
  echo "from_path,category,matched_tokens,action,to_path" > "$MANIFEST"

  local EXCL="(^|/)(\.git|node_modules|vendor|target|dist|build|\.venv|\.claude|\.idea|\.vscode|coverage|.cache|\.architecture/.*|context/archive)($|/)"
  MATCH() { grep -Eiq "$1" <<<"$2"; }
  date_pref() { local f="$1"; local d; d="$(git log --diff-filter=A --follow --format='%cs' -- "$f" 2>/dev/null | tail -n1)"; [[ -z "$d" ]] && d="$(date -r "$f" +%Y-%m-%d 2>/dev/null || date +%Y-%m-%d)"; echo "$d"; }
  slug() { echo "$1" | tr '[:upper:]' '[:lower:]' | sed -E 's/[^a-z0-9]+/-/g;s/^-+|-+$//g'; }

  categorize() {
    local f="$1" base path content
    base="$(basename "$f")"; path="$f"; content="$(sed -n '1,100p' "$f" 2>/dev/null || true)"
    [[ "$path" =~ \.architecture/(research|analysis|options|decisions|execution_logs|templates|snapshots)|context/(notes|archive)|docs/architecture-mode/STATUS\.md ]] && { echo "skip|dest"; return; }
    MATCH "(research|investigation|literature|benchmarks|spike)" "$path$base" && { echo "research|path"; return; }
    MATCH "(analysis|assessment|trade[- ]?offs|findings)" "$path$base" && { echo "analysis|path"; return; }
    MATCH "(options?|alternatives?|proposal|design-options?)" "$path$base" && { echo "options|path"; return; }
    MATCH "(decision|adr-|adr_|^adr|rationale|go-no-go|go_no_go)" "$path$base" && { echo "decisions|path"; return; }
    MATCH "(status|roadmap|milestone|progress|changelog|weekly|standup)" "$path$base" && { echo "status|path"; return; }
    MATCH "(note|notes|journal|scratch|context)" "$path$base" && { echo "notes|path"; return; }
    MATCH "^# +(research|investigation|background|prior art)" "$content" && { echo "research|content"; return; }
    MATCH "^# +(analysis|assessment|findings|trade[ -]?offs)" "$content" && { echo "analysis|content"; return; }
    MATCH "## +options?|### +options?|^# +options?" "$content" && { echo "options|content"; return; }
    MATCH "(adr|architectural decision record|decision)" "$content" && { echo "decisions|content"; return; }
    MATCH "(status|progress|roadmap|milestone|weekly|standup)" "$content" && { echo "status|content"; return; }
    echo "notes|default"
  }

  local scanned=0 matched=0 moved=0 archived=0 skipped=0

  while IFS= read -r -d '' f; do
    [[ "$f" =~ $EXCL ]] && continue
    [[ "$f" =~ \.([Mm][Dd]|[Mm][Dd][Xx]|[Mm][Aa][Rr][Kk][Dd][Oo][Ww][Nn])$ ]] || continue
    scanned=$((scanned+1))
    local cat_out; cat_out="$(categorize "$f")"
    local category="${cat_out%%|*}"; local reason="${cat_out##*|}"
    [[ "$category" != "skip" ]] && matched=$((matched+1)) || skipped=$((skipped+1))

    case "$category" in
      skip)
        echo "$f,skip,$reason,skip," >> "$MANIFEST"
        [[ $SHOW -eq 1 ]] && echo "skip: $f ($reason)"
        ;;
      research|analysis|options|decisions)
        local d; d="$(date_pref "$f")"; local bn; bn="$(basename "$f")"
        local dest=".architecture/$category/${d}-$(slug "${bn%.*}").md"
        if [[ $APPLY -eq 1 ]]; then
          ensure "$(dirname "$dest")"; mv "$f" "$dest"; moved=$((moved+1)); action="plan-move"
        else action="plan-move(dry)"; fi
        echo "$f,$category,$reason,$action,$dest" >> "$MANIFEST"
        [[ $SHOW -eq 1 ]] && echo "$action: $f -> $dest"
        ;;
      status)
        if [[ $APPLY -eq 1 ]]; then
          ensure "docs/architecture-mode"
          {
            echo; echo "<!-- imported from: $f on $(timestamp) -->"; echo
            echo "### Imported: $(basename "$f")"; echo
            sed $'s/\r$//' "$f"; echo
          } >> "docs/architecture-mode/STATUS.md"
          local arch=".architecture/execution_logs/status-archive/$(date_pref "$f")-$(slug "${f%.*}")-original.md"
          ensure "$(dirname "$arch")"; cp "$f" "$arch"; moved=$((moved+1)); action="status-append"
        else action="status-append(dry)"; fi
        echo "$f,status,$reason,$action,docs/architecture-mode/STATUS.md" >> "$MANIFEST"
        [[ $SHOW -eq 1 ]] && echo "$action: $f -> docs/architecture-mode/STATUS.md"
        ;;
      notes|*)
        local dest="context/notes/$(basename "$f")"
        if [[ $APPLY -eq 1 && $ARCH -eq 1 ]]; then
          ensure "context/notes"; mv "$f" "$dest"; archived=$((archived+1)); action="notes-archive"
        else action="notes-archive(dry)"; fi
        echo "$f,notes,$reason,$action,$dest" >> "$MANIFEST"
        [[ $SHOW -eq 1 ]] && echo "$action: $f -> $dest"
        ;;
    esac
  done < <(find . -type f -print0)

  echo
  if [[ $APPLY -eq 1 ]]; then echo "✅ Applied organization."; else echo "ℹ️  Dry-run only. No files moved."; fi
  echo "Manifest: $MANIFEST"
  echo "Scanned: $scanned  Matched: $matched  Skipped: $skipped  Moved: $moved  Notes archived: $archived"
}

cmd_index() {
  ensure "docs/architecture-mode"
  local out="docs/architecture-mode/INDEX.md"
  echo "# Planning Index" > "$out"
  for d in research analysis options decisions; do
    echo -e "\n## ${d^}" >> "$out"
    if compgen -G ".architecture/$d/*.md" >/dev/null; then
      ls -1 ".architecture/$d"/*.md | sed 's/^/ - /' >> "$out"
    else
      echo " - (none)" >> "$out"
    fi
  done
  ok "Wrote index: $out"
}

cmd_snapshot() {
  ensure ".architecture/snapshots"
  local t; t="$(ts_slug)"
  local out=".architecture/snapshots/planning-$t.tar.gz"
  tar -czf "$out" .architecture docs/architecture-mode 2>/dev/null || true
  ok "Snapshot: $out"
}

cmd_status_add() {
  if [[ $# -lt 1 ]]; then err "Usage: archmodectl status add <file.md>"; exit 1; fi
  local src="$1"
  [[ -f "$src" ]] || { err "File not found: $src"; exit 1; }
  ensure "docs/architecture-mode"
  {
    echo; echo "<!-- imported from: $src on $(timestamp) -->"; echo
    echo "### Imported: $(basename "$src")"; echo
    sed $'s/\r$//' "$src"; echo
  } >> "docs/architecture-mode/STATUS.md"
  local arch=".architecture/execution_logs/status-archive/$(date +%Y-%m-%d)-$(slugify "${src%.*}")-original.md"
  ensure "$(dirname "$arch")"; cp "$src" "$arch"
  ok "Appended to STATUS.md and archived original."
}

cmd_safe_commit() {
  require git
  local t; t="$(ts_slug)"
  git add -A >/dev/null 2>&1 || true
  git commit -m "chore: safe-commit snapshot ($t)" >/dev/null 2>&1 || true
  git tag "archmode-snap-$t" >/dev/null 2>&1 || true
  ensure ".architecture/snapshots"
  git archive --format=tar.gz -o ".architecture/snapshots/repo-$t.tar.gz" HEAD || true
  ok "Created tag archmode-snap-$t and snapshot archive."
}

cmd_house_install() {
  local ORG=""
  local OWNERS=""
  local STRICT=0
  local SOLO=0
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --org) ORG="$2"; shift 2;;
      --owners) OWNERS="$2"; shift 2;;
      --strict) STRICT=1; shift 1;;
      --solo) SOLO=1; shift 1;;
      *) err "Unknown arg: $1"; exit 1;;
    esac
  done

  cat > .editorconfig <<'EOF'
root = true
[*]
charset = utf-8
end_of_line = lf
insert_final_newline = true
trim_trailing_whitespace = true
indent_style = space
indent_size = 2
[*.rs]
indent_size = 4
EOF

  ensure ".github/ISSUE_TEMPLATE" "docs/architecture-mode"

  if [[ $SOLO -eq 1 ]]; then
    cat > .github/pull_request_template.md <<'EOF'
## Summary
Why and what changed?

## Notes
- Link to ADR (optional): 
- Risk/rollback: 
- Checklist: tests/docs updated if needed
EOF
    cat > docs/architecture-mode/HOUSE-STYLE.md <<'EOF'
# Solo House Style
- Keep changes small; PRs optional.
- Gate code changes with a personal "go" decision (see `.architecture/GO` and `archmodectl go on|off`).
- Keep planning docs in `.architecture/` and daily logs in `docs/architecture-mode/daily/`.
EOF
    ok "Installed SOLO house-style pack (minimal templates)."
  else
    if [[ -n "$ORG" || -n "$OWNERS" ]]; then
      cat > .github/CODEOWNERS <<EOF
* ${ORG:+@$ORG}
EOF
      if [[ -n "$OWNERS" ]]; then
        for u in $OWNERS; do echo "* @$u" >> .github/CODEOWNERS; done
      fi
    fi

    cat > .github/pull_request_template.md <<'EOF'
## Summary
What changed and why

## Plan / Decision
- [ ] ADR reference or 'go' approval

## Checklist
- [ ] Tests/docs updated
- [ ] Risk/rollback noted
EOF

    if [[ $STRICT -eq 1 ]]; then
      cat > commitlint.config.js <<'EOF'
module.exports = { extends: ['@commitlint/config-conventional'] };
EOF
      ensure ".github/workflows"
      cat > .github/workflows/commitlint.yml <<'EOF'
name: Commitlint
on:
  pull_request:
jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: wagoid/commitlint-github-action@v6
EOF
      ok "Enabled strict commitlint CI."
    fi
    ok "Installed house-style pack."
  fi
}

solo_journal() {
  ensure "docs/architecture-mode/daily"
  local d; d="$(date +%Y-%m-%d)"
  local f="docs/architecture-mode/daily/$d.md"
  if [[ ! -f "$f" ]]; then
    cat > "$f" <<EOF
# $d — Daily

## Status
-

## Today
-

## Risks/Assumptions
-

## Next
-

EOF
    ok "Created $f"
  else
    warn "Daily already exists: $f"
  fi

  ensure "docs/architecture-mode"
  if [[ ! -f docs/architecture-mode/STATUS.md ]] || ! grep -q "$d — Daily" docs/architecture-mode/STATUS.md 2>/dev/null; then
    {
      echo; echo "### $d — Daily"
      echo "- See: [$d](daily/$d.md)"
    } >> docs/architecture-mode/STATUS.md
  fi
}

solo_hooks_install() {
  require git
  ensure ".git/hooks"
  cat > .git/hooks/pre-commit <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
# If GO flag exists, allow all changes
if [[ -f ".architecture/GO" ]]; then
  exit 0
fi
# Only allow planning/doc-only commits without GO
CHANGED=$(git diff --cached --name-only)
if [[ -z "$CHANGED" ]]; then exit 0; fi
echo "$CHANGED" | grep -Ev '^(\.architecture/(research|analysis|options|decisions|execution_logs|templates|snapshots)|docs/architecture-mode/|context/notes/|README\.md|ARCHMODE-README\.md|\.claude/|prompts/|scripts/claude-start\.sh)$' >/dev/null && {
  echo "Architecture Mode (solo): No .architecture/GO flag; committing code changes is blocked."
  echo "Run:   ./archmodectl.sh go on      # enable execution (creates .architecture/GO)"
  echo "Then:  git commit -m '...'"
  echo "After merge: ./archmodectl.sh go off"
  exit 1
}
exit 0
EOF
  chmod +x .git/hooks/pre-commit
  ok "Installed local pre-commit guard (solo)."
}

solo_hooks_remove() {
  if [[ -f .git/hooks/pre-commit ]]; then
    rm -f .git/hooks/pre-commit
    ok "Removed local pre-commit guard."
  else
    warn "No pre-commit hook found."
  fi
}

cmd_solo() {
  sub="${1:-}"; shift || true
  case "$sub" in
    install)
      cmd_house_install --solo
      ;;
    journal)
      solo_journal
      ;;
    hooks)
      sub2="${1:-}"; shift || true
      case "$sub2" in
        install) solo_hooks_install ;;
        remove)  solo_hooks_remove ;;
        *) err "Usage: archmodectl solo hooks install|remove"; exit 1 ;;
      esac
      ;;
    *)
      err "Usage: archmodectl solo install|journal|hooks install|remove"; exit 1 ;;
  esac
}

cmd_go() {
  sub="${1:-}"; shift || true
  ensure ".architecture"
  case "$sub" in
    on)
      : > .architecture/GO
      ok "GO enabled (.architecture/GO created). You may commit code changes."
      ;;
    off)
      rm -f .architecture/GO
      ok "GO disabled (.architecture/GO removed)."
      ;;
    status)
      if [[ -f .architecture/GO ]]; then
        echo "GO is ON"
      else
        echo "GO is OFF"
      fi
      ;;
    *)
      err "Usage: archmodectl go on|off|status"; exit 1 ;;
  esac
}

# ---------- router ----------
cmd="${1:-}"; shift || true
case "$cmd" in
  init) cmd_init "$@" ;;
  adopt) cmd_adopt "$@" ;;
  organize) cmd_organize "$@" ;;
  agents)
    sub="${1:-}"; shift || true
    case "$sub" in
      vendor) cmd_agents_vendor "$@" ;;
      add) cmd_agents_add "$@" ;;
      list) cmd_agents_list "$@" ;;
      *) usage; exit 1 ;;
    esac
    ;;
  style)
    sub="${1:-}"; shift || true
    case "$sub" in
      install) cmd_style_install "$@" ;;
      *) usage; exit 1 ;;
    esac
    ;;
  mcp)
    sub="${1:-}"; shift || true
    case "$sub" in
      add) cmd_mcp_add "$@" ;;
      *) usage; exit 1 ;;
    esac
    ;;
  adr)
    sub="${1:-}"; shift || true
    case "$sub" in
      new) shift || true; cmd_adr_new "$@" ;;
      list) cmd_adr_list "$@" ;;
      link) cmd_adr_link "$@" ;;
      *) usage; exit 1 ;;
    esac
    ;;
  docs)
    sub="${1:-}"; shift || true
    case "$sub" in
      lint) cmd_docs_lint "$@" ;;
      ci) cmd_docs_ci "$@" ;;
      serve) cmd_docs_serve_build serve ;;
      build) cmd_docs_serve_build build ;;
      *) usage; exit 1 ;;
    esac
    ;;
  diagrams)
    sub="${1:-}"; shift || true
    case "$sub" in
      build) cmd_diagrams_build "$@" ;;
      *) usage; exit 1 ;;
    esac
    ;;
  guard)
    sub="${1:-}"; shift || true
    case "$sub" in
      ci) cmd_guard_ci "$@" ;;
      *) usage; exit 1 ;;
    esac
    ;;
  index) cmd_index "$@" ;;
  snapshot) cmd_snapshot "$@" ;;
  status)
    sub="${1:-}"; shift || true
    case "$sub" in
      add) cmd_status_add "$@" ;;
      *) usage; exit 1 ;;
    esac
    ;;
  safe-commit) cmd_safe_commit "$@" ;;
  house)
    sub="${1:-}"; shift || true
    case "$sub" in
      install) cmd_house_install "$@" ;;
      *) usage; exit 1 ;;
    esac
    ;;
  solo) cmd_solo "$@" ;;
  go) cmd_go "$@" ;;
  ""|-h|--help|help) usage ;;
  *) usage; exit 1 ;;
esac
