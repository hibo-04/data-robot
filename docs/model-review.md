# Model review (relationships, KPIs, UI)

The analytics core proposes a model. A person must be able to **accept, reject, or edit** those proposals. That confirmed model is what queries and reports should use. The user can change it again later.

This document is the product contract. The local UI in `web/` (`cargo run -p analytics-web`) is the first screen for that overlay. Overlay JSON is stored under `out/overlays/`. The engine still treats confirmation as a first-class idea when no overlay exists: analysis output is used directly.

## What the user should see

Every suggestion is shown with the **analysis evidence**, not just a yes/no:

### Relationships

- from / to columns
- type (`many_to_one`, …)
- band: declared / inferred / uncertain
- confidence
- evidence: datatype match, name similarity, unique/PK, value coverage, notes
- reason string (why this edge exists)

Declared database foreign keys are pre-accepted (confidence 1.0) but remain visible and **can still be disabled** if the catalog is wrong.

Inferred and uncertain edges start as *suggested*. The user confirms or rejects them.

### KPIs

- structural name (entity + column, domain-agnostic), e.g. `Order Item Quantity`
- optional business label from a dictionary pack, e.g. `Units Sold`
- aggregation, source column, confidence, reason
- which dictionary pack/entry produced the label (or `none`)

The user can keep the structural name, take the dictionary label, or type their own label. They can drop a KPI from the published model.

### Semantic roles

Column roles (measure, dimension, time, identifier, FK) should be overridable the same way. A “quantity” on a production table stays a measure even if the commerce pack is off.

### Numeric identities

Suggestions come from **values**, never from column names (`tax`, `gross`, `amount`, …). Show:

- expression (`invoices.gross_amount ≈ orders.net_amount + invoices.tax_amount`)
- template (`sum`, `rate`, `product`, `scaled_product`, `grain_sum`, …)
- match ratio, MAE, sample size
- coefficient `k` when the model is `y ≈ k · x` or `y ≈ k · a · b`
- scope: same table / join / grain (`SUM` of children)

The user can accept, reject, or edit the formula. Accepted identities belong in the published model (derived measures, “do not double-count”).

## Decisions are stored and mutable

User choices are a **versioned overlay** on top of the last analysis, not a rewrite of the source database.

Intended record (later JSON, not implemented yet):

```json
{
  "source_id": "ecommerce_dirty",
  "updated_at": "...",
  "relationships": [
    {
      "from": "orders.customer_nr",
      "to": "customers.id",
      "status": "accepted",
      "user_note": "legacy customer number"
    },
    {
      "from": "returns.return_id",
      "to": "orders.order_id",
      "status": "rejected"
    }
  ],
  "kpis": [
    {
      "id": "sum:order_items.quantity",
      "status": "accepted",
      "label": "Units Sold"
    },
    {
      "id": "sum:products.list_price",
      "status": "rejected"
    }
  ],
  "identities": [
    {
      "id": "sum:invoices.gross_amount~orders.net_amount,invoices.tax_amount",
      "status": "accepted"
    },
    {
      "id": "rate:products.list_price~products.unit_cost",
      "status": "rejected"
    }
  ],
  "packs": ["generic", "commerce"]
}
```

Status values: `accepted` | `rejected` | `edited` | `pending`.

Re-running discovery must **not wipe** these decisions. New suggestions appear as `pending`; previous accepts/rejects stay until the user changes them.

Changing a setting later is the same store: edit overlay, recompute queries/reports from the published model.

## Engine vs. published model

```text
analysis (suggestions + evidence)
        ↓
user overlay (accept / reject / rename)
        ↓
published semantic model  ← query planner and reports use this
```

Until an overlay exists, V1 uses the analysis output directly so the CLI keeps working.

## Out of scope for now

- Wizards, drag-and-drop, and chart builders
- Auth, sharing, multi-user edit
- Automatic overwrite of user decisions

The local UI follows this file: show evidence, let the user finalize relationships, KPIs, and numeric identities, keep those choices editable.

## Catalogs grow with tests

The engine uses **fixed catalogs**, not an LLM: identity templates, dictionary packs, name/synonym lists, and similar lookup tables.

When a test or fixture shows a real pattern that the catalog does not cover yet (for example `weighted_amount ≈ amount × probability / 100`, or a missing commerce label):

1. **Add the missing entry** to the catalog (new identity template, dictionary term, synonym, …).
2. **Add or extend a small fixture/test** that asserts the pattern is now found.
3. Do **not** hardcode table or column names in the core to make that one case pass.
4. Do **not** skip the gap because “we already have a few examples.”

This applies to every template- or dictionary-like store we keep, including:

- identity templates in `src/identities.rs`
- KPI/label packs in `dictionaries/`
- any future synonym lists, role hints, or report templates of the same kind

If the catalog already has an equivalent entry, reuse it. Only add when the pattern is genuinely new. Keep fixtures small (see the test-data budget); extra coverage means another compact schema, not more rows.
