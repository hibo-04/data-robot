# analytics

Source-agnostic analytics engine plus two adapters around it — a throwaway local PoC UI and the beginnings of the B2B product app.

| Path | Crate | World |
| --- | --- | --- |
| `src/` | `analytics` | The engine. No HTTP, HTML, auth, tenancy or LLM. Only `DataSource` in, suggestions with evidence out. |
| `web/` | `analytics-web` | Local, login-free PoC review page. Not extended into the product. |
| `app/` | `octa-app` | The product app (working title *Octa*). Accounts, organizations, sessions, locale and jobs live here, never in the engine. Built package by package along [docs/roadmap.md](docs/roadmap.md); today: shell (P0.1) plus locale stack with DE/EN catalogs (P0.2). |

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
cargo run -p analytics-web   # PoC UI, http://127.0.0.1:3000
cargo run -p octa-app        # product app, http://127.0.0.1:4000  (?lang=en / ?lang=de)
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
- `app/` — product app (`octa-app`): Axum, baseline security headers, boundary tests that keep the engine free of HTTP/HTML; `app/locales/*.json` message catalogs (ICU MessageFormat, DE + EN, parity enforced by tests), `app/src/locale.rs` language / formats / time-zone resolution
- `docs/glossary.md` — product terms: translated or deliberately not, state labels
- `fixtures/` — small CSV models plus `ground_truth.json` (benchmark only; never read during analysis)
  - shop: `ecommerce_clean`, `ecommerce_dirty`
  - CRM: `crm`; edge cases: `edge_cases`
  - B2B/ERP: `erp_clean`, `erp_dirty`
  - production: `production_clean`, `production_dirty`
  - logistics: `logistics_clean`, `logistics_dirty`
  - SaaS: `saas_clean`, `saas_dirty`
  - professional services: `projects_clean`, `projects_dirty`
- `docs/model-review.md` — later user accept/reject overlay (no UI yet)
- `docs/roadmap.md`, `docs/web-app-construct.md`, `docs/ui-ux-notes.md` — plan, platform construct and UI notes for the product app
