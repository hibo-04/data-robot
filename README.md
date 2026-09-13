# analytics

Local proof of concept for a source-agnostic analytics engine. No auth or cloud. A minimal local review UI lives in `web/`.

The engine infers schema, profiles, relationships, numeric column identities, a semantic model, KPI candidates, SQL-shaped queries, and report suggestions. It must work without an LLM.

KPIs are **domain-agnostic** (entity + column + aggregation). Optional dictionary packs in `dictionaries/` add business labels (`generic`, `commerce`). They are never required.

V1 fixtures stay at **≤ 10 tables and ≤ 10_000 rows**. Do not grow them for performance tests.

Numeric **identities** (`a ≈ b + c`, `a ≈ k * b`, parent ≈ SUM(child)) are found from values only — not from column names.

User confirmation of relationships/KPIs/identities: local UI (`cargo run -p analytics-web`) or later overlay JSON. See [docs/model-review.md](docs/model-review.md). When tests find a missing formula or label, extend those catalogs (templates, dictionaries) instead of hardcoding names in the core.

## Run

```bash
python3 scripts/generate_fixtures.py   # already committed after first generation
cargo test
cargo run -- inspect --fixture ./fixtures/ecommerce_dirty
cargo run -- inspect --fixture ./fixtures/ecommerce_dirty --packs generic
cargo run -- detect-identities --fixture ./fixtures/ecommerce_dirty
cargo run -- export-model --fixture ./fixtures/ecommerce_dirty --out ./out
cargo run -- generate-query --fixture ./fixtures/ecommerce_dirty --query-json ./fixtures/examples/revenue_by_region.json --execute
cargo run -- benchmark --fixture ./fixtures/ecommerce_dirty
cargo run -p analytics-web   # http://127.0.0.1:3000
```

PostgreSQL (optional):

```bash
cargo run -- inspect --database-url "postgres://user:pass@localhost/dbname"
```

## Layout

- `src/connector` — `DataSource` trait, `FixtureConnector`, `PostgreSQLConnector`
- `src/profiling`, `relationships`, `identities`, `semantic`, `kpi`, `query`, `reports` — analytics core
- `src/dictionary` — optional label packs (`dictionaries/generic.json`, `dictionaries/commerce.json`)
- `web/` — local Axum + HTMX review UI (overlay JSON under `out/overlays/`)
- `fixtures/` — small CSV models plus `ground_truth.json` (benchmark only; never read during analysis)
  - shop: `ecommerce_clean`, `ecommerce_dirty`
  - CRM: `crm`; edge cases: `edge_cases`
  - B2B/ERP: `erp_clean`, `erp_dirty`
  - production: `production_clean`, `production_dirty`
  - logistics: `logistics_clean`, `logistics_dirty`
  - SaaS: `saas_clean`, `saas_dirty`
  - professional services: `projects_clean`, `projects_dirty`
- `docs/model-review.md` — later user accept/reject overlay (no UI yet)
