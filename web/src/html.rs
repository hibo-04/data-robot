use analytics::identities::ColumnIdentity;
use analytics::kpi::{Certainty, KpiCandidate};
use analytics::pipeline::Analysis;
use analytics::query::QueryResult;
use analytics::relationships::{Relationship, RelationshipBand};
use analytics::reports::Visualization;
use analytics::semantic::SemanticRole;

use crate::overlay::{display_kpi_name, Overlay, ReviewStatus};

pub struct ChartView {
    pub labels: Vec<String>,
    pub values: Vec<f64>,
}

pub struct KpiValue {
    pub title: String,
    pub value: String,
}

pub struct Dashboard {
    pub cards: Vec<KpiValue>,
    pub chart_title: Option<String>,
    pub chart: Option<ChartView>,
    pub table: Option<QueryResult>,
    pub sql: Option<String>,
}

pub struct ExploreView {
    pub sql: String,
    pub result: QueryResult,
    pub chart: Option<ChartView>,
}

pub struct SourceView {
    pub name: String,
    pub packs: String,
    pub analysis: Analysis,
    pub overlay: Overlay,
    pub dashboard: Dashboard,
    pub explore: Option<ExploreView>,
    pub error: Option<String>,
}

pub fn layout(title: &str, body: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<link rel="stylesheet" href="/style.css">
<script src="https://unpkg.com/htmx.org@2.0.4" defer></script>
</head>
<body>
{body}
</body>
</html>"#,
        title = esc(title),
        body = body
    )
}

pub fn error_page(status: u16, message: &str) -> String {
    layout(
        "analytics",
        &format!(
            r#"<header class="app"><a class="brand" href="/">analytics</a></header>
<main>
<h1>{status}</h1>
<p class="flash">{msg}</p>
<p><a href="/">Back to fixtures</a></p>
</main>"#,
            status = status,
            msg = esc(message)
        ),
    )
}

pub fn home_page(fixtures: &[String], packs_default: &str, error: Option<&str>) -> String {
    let mut items = String::new();
    if fixtures.is_empty() {
        items.push_str(r#"<p class="flash">No fixtures found. Point <code>--fixtures</code> at the CSV models.</p>"#);
    } else {
        items.push_str(r#"<ul class="fixture-list">"#);
        for name in fixtures {
            items.push_str(&format!(
                r#"<li><button class="pick" type="submit" name="fixture" value="{name}">{label}</button></li>"#,
                name = esc(name),
                label = esc(name)
            ));
        }
        items.push_str("</ul>");
    }
    let err = error
        .map(|e| format!(r#"<p class="flash">{}</p>"#, esc(e)))
        .unwrap_or_default();
    layout(
        "analytics",
        &format!(
            r#"<header class="app">
<a class="brand" href="/">analytics</a>
<span class="meta">local model review</span>
</header>
<main>
<h1>Choose a fixture</h1>
<p class="lede">The engine proposes relationships, KPIs, and identities. You accept or reject them. Queries and reports use the published overlay.</p>
{err}
<form method="post" action="/analyze">
<label class="field" for="packs">Dictionary packs</label>
<input id="packs" type="text" name="packs" value="{packs}">
<label class="field">Fixtures</label>
{items}
</form>
</main>"#,
            err = err,
            packs = esc(packs_default),
            items = items
        ),
    )
}

pub fn source_page(view: &SourceView) -> String {
    let a = &view.analysis;
    let (declared, inferred, uncertain) = a.relationships.counts();
    let body = format!(
        r#"<header class="app">
<a class="brand" href="/">analytics</a>
<span class="meta">{name} · {ms}ms · packs {packs}</span>
</header>
<main>
<h1>{name}</h1>
<p class="lede">Review suggestions with their evidence. Rejected items stay visible so you can restore them. Queries below use the published model.</p>
{flash}
<nav class="jump">
<a href='#overview'>Overview</a>
<a href='#relationships'>Relationships ({nrel})</a>
<a href='#kpis'>KPIs ({nkpi})</a>
<a href='#identities'>Identities ({nid})</a>
<a href='#reports'>Reports</a>
</nav>

<section id="overview">
<h2>Overview</h2>
<div class="stats">
<span>{tables} tables</span>
<span>{cols} columns</span>
<span>declared {declared} · inferred {inferred} · uncertain {uncertain}</span>
</div>
<table class="tables">
<thead><tr><th>Table</th><th>Columns</th><th>Rows</th></tr></thead>
<tbody>{table_rows}</tbody>
</table>
</section>

<section id="relationships">
<h2>Relationships</h2>
{rels}
</section>

<section id="kpis">
<h2>KPIs</h2>
{kpis}
</section>

<section id="identities">
<h2>Identities</h2>
{idents}
</section>

<section id="reports">
<h2>Reports</h2>
{dash}
<h3>Explore</h3>
{explore_form}
<div id='explore-result'>{explore}</div>
{suggestions}
</section>
</main>"#,
        name = esc(&view.name),
        ms = a.timings.total_ms,
        packs = esc(&view.packs),
        flash = view
            .error
            .as_deref()
            .map(|e| format!(r#"<p class="flash">{}</p>"#, esc(e)))
            .unwrap_or_default(),
        nrel = a.relationships.relationships.len(),
        nkpi = a.kpis.len(),
        nid = a.identities.len(),
        tables = a.schema.tables.len(),
        cols = a.schema.column_count(),
        declared = declared,
        inferred = inferred,
        uncertain = uncertain,
        table_rows = table_rows(a),
        rels = relationship_cards(view),
        kpis = kpi_cards(view),
        idents = identity_cards(view),
        dash = dashboard_html(&view.dashboard),
        explore_form = explore_form(view),
        explore = view
            .explore
            .as_ref()
            .map(explore_result)
            .unwrap_or_default(),
        suggestions = report_list(view),
    );
    layout(&format!("analytics · {}", view.name), &body)
}

pub fn relationship_card(
    name: &str,
    packs: &str,
    rel: &Relationship,
    status: ReviewStatus,
) -> String {
    let id = rel_dom_id(&rel.from.qualified(), &rel.to.qualified());
    let band = match rel.band {
        RelationshipBand::Declared => "declared",
        RelationshipBand::Inferred => "inferred",
        RelationshipBand::Uncertain => "uncertain",
    };
    let notes = rel.evidence.notes.join(" · ");
    format!(
        r#"<article class="card {rej}" id="{id}">
<div class="card-head">
<span class="pill">{band}</span>
<span class="pill {st}">{st}</span>
<span class="pill">{conf:.0}%</span>
<span class="pill">{ty:?}</span>
</div>
<h3>{from} → {to}</h3>
<p class="reason">{reason}</p>
<p class="evidence">types {dt:.0}% · names {ns:.0}% · coverage {cov:.0}%{pk}{notes}</p>
<div class="actions">
{accept}{reject}{pending}
</div>
</article>"#,
        rej = if status.is_rejected() { "rejected" } else { "" },
        id = esc(&id),
        band = band,
        st = status.as_str(),
        conf = rel.confidence * 100.0,
        ty = rel.relationship_type,
        from = esc(&rel.from.qualified()),
        to = esc(&rel.to.qualified()),
        reason = esc(&rel.reason),
        dt = rel.evidence.datatype_match * 100.0,
        ns = rel.evidence.name_similarity * 100.0,
        cov = rel.evidence.value_coverage * 100.0,
        pk = if rel.evidence.target_is_primary_key {
            " · target PK"
        } else {
            ""
        },
        notes = if notes.is_empty() {
            String::new()
        } else {
            format!(" · {}", esc(&notes))
        },
        accept = overlay_button(
            name,
            packs,
            "relationship",
            &id,
            ReviewStatus::Accepted,
            status,
            rel,
        ),
        reject = overlay_button(
            name,
            packs,
            "relationship",
            &id,
            ReviewStatus::Rejected,
            status,
            rel,
        ),
        pending = overlay_button(
            name,
            packs,
            "relationship",
            &id,
            ReviewStatus::Pending,
            status,
            rel,
        ),
    )
}

pub fn kpi_review_card(name: &str, packs: &str, kpi: &KpiCandidate, overlay: &Overlay) -> String {
    let status = overlay.kpi_status(&kpi.id);
    let label = display_kpi_name(kpi, overlay);
    let id = kpi_dom_id(&kpi.id);
    format!(
        r#"<article class="card {rej}" id="{id}">
<div class="card-head">
<span class="pill {st}">{st}</span>
<span class="pill">{conf:.0}%</span>
<span class="pill">{cert}</span>
{agg}
</div>
<h3>{label}</h3>
<p class="reason">{reason}</p>
<p class="evidence">{struct}{src}{pack}</p>
<div class="actions">
{accept}{reject}
<form method="post" action="{action}" hx-post="{action}" hx-target='#{id}' hx-swap="outerHTML">
<input type="hidden" name="kind" value="kpi">
<input type="hidden" name="id" value="{kid}">
<input type="hidden" name="status" value="edited">
<input type="hidden" name="packs" value="{packs}">
<input type="text" name="label" value="{label_attr}" placeholder='custom label'>
<button type="submit">Save label</button>
</form>
</div>
</article>"#,
        rej = if status.is_rejected() { "rejected" } else { "" },
        id = esc(&id),
        st = status.as_str(),
        conf = kpi.confidence * 100.0,
        cert = certainty(kpi.certainty),
        agg = kpi
            .aggregation
            .map(|a| format!(r#"<span class="pill">{:?}</span>"#, a))
            .unwrap_or_default(),
        label = esc(&label),
        reason = esc(&kpi.reason),
        struct = esc(&kpi.name),
        src = kpi
            .source
            .as_ref()
            .map(|s| format!(" · {}", esc(&s.qualified())))
            .unwrap_or_default(),
        pack = kpi
            .dictionary_pack
            .as_deref()
            .map(|p| format!(" · pack {}", esc(p)))
            .unwrap_or_else(|| " · no pack".into()),
        accept = kpi_button(name, packs, kpi, ReviewStatus::Accepted, status),
        reject = kpi_button(name, packs, kpi, ReviewStatus::Rejected, status),
        action = overlay_action(name),
        kid = esc(&kpi.id),
        packs = esc(packs),
        label_attr = esc(&label),
    )
}

pub fn identity_review_card(
    name: &str,
    packs: &str,
    idn: &ColumnIdentity,
    status: ReviewStatus,
) -> String {
    let id = ident_dom_id(&idn.id);
    format!(
        r#"<article class="card {rej}" id="{id}">
<div class="card-head">
<span class="pill">{scope:?}</span>
<span class="pill {st}">{st}</span>
<span class="pill">{conf:.0}%</span>
<span class="pill">{cert}</span>
<span class="pill">{tmpl:?}</span>
</div>
<h3>{expr}</h3>
<p class="reason">{reason}</p>
<p class="evidence">match {match_pct:.0}% of {pairs} rows · MAE {mae:.4}{k}</p>
<div class="actions">
{accept}{reject}{pending}
</div>
</article>"#,
        rej = if status.is_rejected() { "rejected" } else { "" },
        id = esc(&id),
        scope = idn.scope,
        st = status.as_str(),
        conf = idn.confidence * 100.0,
        cert = certainty(idn.certainty),
        tmpl = idn.template,
        expr = esc(&idn.expression),
        reason = esc(&idn.reason),
        match_pct = idn.match_ratio * 100.0,
        pairs = idn.pair_count,
        mae = idn.mae,
        k = idn
            .coefficient
            .map(|c| format!(" · k={c:.4}"))
            .unwrap_or_default(),
        accept = ident_button(name, packs, idn, ReviewStatus::Accepted, status),
        reject = ident_button(name, packs, idn, ReviewStatus::Rejected, status),
        pending = ident_button(name, packs, idn, ReviewStatus::Pending, status),
    )
}

pub fn explore_result(view: &ExploreView) -> String {
    let mut out = String::from(r#"<div id="explore-result">"#);
    out.push_str(&format!(r#"<pre class="sql">{}</pre>"#, esc(&view.sql)));
    if let Some(chart) = &view.chart {
        out.push_str(&bar_chart(chart));
    }
    out.push_str(&query_table(&view.result));
    out.push_str("</div>");
    out
}

pub fn chart_from_result(result: &QueryResult) -> Option<ChartView> {
    if result.columns.len() < 2 || result.rows.is_empty() {
        return None;
    }
    let mut labels = Vec::new();
    let mut values = Vec::new();
    for row in result.rows.iter().take(24) {
        if row.len() < 2 {
            continue;
        }
        let Some(v) = parse_number(&row[1]) else {
            continue;
        };
        labels.push(row[0].clone());
        values.push(v);
    }
    if values.is_empty() {
        None
    } else {
        Some(ChartView { labels, values })
    }
}

pub fn scalar_from_result(result: &QueryResult) -> Option<String> {
    result.rows.first().and_then(|r| r.first()).cloned()
}

fn relationship_cards(view: &SourceView) -> String {
    if view.analysis.relationships.relationships.is_empty() {
        return r#"<p class="muted">None found.</p>"#.into();
    }
    view.analysis
        .relationships
        .relationships
        .iter()
        .map(|rel| {
            relationship_card(
                &view.name,
                &view.packs,
                rel,
                view.overlay.relationship_status(rel),
            )
        })
        .collect()
}

fn kpi_cards(view: &SourceView) -> String {
    if view.analysis.kpis.is_empty() {
        return r#"<p class="muted">None found.</p>"#.into();
    }
    view.analysis
        .kpis
        .iter()
        .map(|kpi| kpi_review_card(&view.name, &view.packs, kpi, &view.overlay))
        .collect()
}

fn identity_cards(view: &SourceView) -> String {
    if view.analysis.identities.is_empty() {
        return r#"<p class="muted">None found.</p>"#.into();
    }
    view.analysis
        .identities
        .iter()
        .map(|id| {
            identity_review_card(
                &view.name,
                &view.packs,
                id,
                view.overlay.identity_status(&id.id),
            )
        })
        .collect()
}

fn table_rows(analysis: &Analysis) -> String {
    analysis
        .schema
        .tables
        .iter()
        .map(|t| {
            let rows = analysis
                .profiles
                .table(&t.name)
                .map(|p| p.row_count.to_string())
                .unwrap_or_else(|| "–".into());
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
                esc(&t.name),
                t.columns.len(),
                esc(&rows)
            )
        })
        .collect()
}

fn dashboard_html(dash: &Dashboard) -> String {
    let mut out = String::new();
    if !dash.cards.is_empty() {
        out.push_str(r#"<div class="kpis-dash">"#);
        for card in &dash.cards {
            out.push_str(&format!(
                r#"<div class="kpi-value"><div class="num">{v}</div><div class="lbl">{t}</div></div>"#,
                v = esc(&card.value),
                t = esc(&card.title)
            ));
        }
        out.push_str("</div>");
    }
    if let Some(title) = &dash.chart_title {
        out.push_str(&format!("<h3>{}</h3>", esc(title)));
    }
    if let Some(sql) = &dash.sql {
        out.push_str(&format!(r#"<pre class="sql">{}</pre>"#, esc(sql)));
    }
    if let Some(chart) = &dash.chart {
        out.push_str(&bar_chart(chart));
    }
    if let Some(table) = &dash.table {
        out.push_str(&query_table(table));
    }
    if out.is_empty() {
        out.push_str(r#"<p class="muted">No published KPI could be executed yet.</p>"#);
    }
    out
}

fn explore_form(view: &SourceView) -> String {
    let measures: Vec<_> = view
        .analysis
        .kpis
        .iter()
        .filter(|k| crate::overlay::queryable(k) && !view.overlay.kpi_status(&k.id).is_rejected())
        .collect();
    let mut measure_opts = String::new();
    for kpi in &measures {
        let label = display_kpi_name(kpi, &view.overlay);
        measure_opts.push_str(&format!(
            r#"<option value="{v}">{l}</option>"#,
            v = esc(&kpi.name),
            l = esc(&label)
        ));
    }
    let mut dim_opts = String::from(r#"<option value="">(no dimension)</option>"#);
    for col in view.analysis.semantic_model.columns.iter().filter(|c| {
        matches!(
            c.role,
            SemanticRole::Dimension
                | SemanticRole::CategoricalDimension
                | SemanticRole::TimeDimension
        )
    }) {
        dim_opts.push_str(&format!(
            r#"<option value="{v}">{l}</option>"#,
            v = esc(&col.ref_.qualified()),
            l = esc(&col.semantic_name)
        ));
    }
    if measures.is_empty() {
        return r#"<p class="muted">No queryable KPIs in the published model.</p>"#.into();
    }
    format!(
        r#"<form method="post" action="/source/{name}/query" hx-post="/source/{name}/query" hx-target='#explore-result' hx-swap="outerHTML">
<input type="hidden" name="packs" value="{packs}">
<label class="field" for="measure">Measure</label>
<select id="measure" name="measure">{measures}</select>
<label class="field" for="dimension">Dimension</label>
<select id="dimension" name="dimension">{dims}</select>
<div class="actions"><button class="primary" type="submit">Run query</button></div>
</form>"#,
        name = esc(&view.name),
        packs = esc(&view.packs),
        measures = measure_opts,
        dims = dim_opts
    )
}

fn report_list(view: &SourceView) -> String {
    if view.analysis.reports.is_empty() {
        return String::new();
    }
    let mut out = String::from("<h3>Suggestions</h3>");
    for report in &view.analysis.reports {
        let vis = match report.visualization {
            Visualization::KpiCard => "card",
            Visualization::LineChart => "line",
            Visualization::BarChart => "bar",
            Visualization::PieChart => "pie",
            Visualization::Table => "table",
        };
        let run = match (&report.measure, &report.dimension) {
            (Some(measure), dim) => format!(
                r#"<form method="post" action="/source/{name}/query" hx-post="/source/{name}/query" hx-target='#explore-result' hx-swap="outerHTML">
<input type="hidden" name="packs" value="{packs}">
<input type="hidden" name="measure" value="{m}">
<input type="hidden" name="dimension" value="{d}">
<button type="submit">Run</button>
</form>"#,
                name = esc(&view.name),
                packs = esc(&view.packs),
                m = esc(measure),
                d = esc(dim.as_deref().unwrap_or(""))
            ),
            _ => String::new(),
        };
        out.push_str(&format!(
            r#"<article class="card">
<div class="card-head"><span class="pill">{vis}</span><span class="pill">{conf:.0}%</span></div>
<h3>{title}</h3>
<p class="reason">{reason}</p>
<div class="actions">{run}</div>
</article>"#,
            vis = vis,
            conf = report.confidence * 100.0,
            title = esc(&report.title),
            reason = esc(&report.reason),
            run = run
        ));
    }
    out
}

fn query_table(result: &QueryResult) -> String {
    if result.columns.is_empty() {
        return r#"<p class="muted">Empty result.</p>"#.into();
    }
    let mut out = String::from(r#"<table class="data"><thead><tr>"#);
    for col in &result.columns {
        out.push_str(&format!("<th>{}</th>", esc(col)));
    }
    out.push_str("</tr></thead><tbody>");
    for row in result.rows.iter().take(80) {
        out.push_str("<tr>");
        for cell in row {
            out.push_str(&format!("<td>{}</td>", esc(cell)));
        }
        out.push_str("</tr>");
    }
    out.push_str("</tbody></table>");
    if result.rows.len() > 80 {
        out.push_str(&format!(
            r#"<p class="muted">Showing 80 of {} rows.</p>"#,
            result.rows.len()
        ));
    }
    out
}

fn bar_chart(chart: &ChartView) -> String {
    let w = 720.0;
    let h = 280.0;
    let pad_l = 12.0;
    let pad_r = 12.0;
    let pad_t = 12.0;
    let pad_b = 64.0;
    let plot_w = w - pad_l - pad_r;
    let plot_h = h - pad_t - pad_b;
    let n = chart.values.len().max(1) as f64;
    let max = chart
        .values
        .iter()
        .copied()
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let slot = plot_w / n;
    let bar_w = slot * 0.62;
    let mut bars = String::new();
    for (i, (label, val)) in chart.labels.iter().zip(&chart.values).enumerate() {
        let bh = (*val / max) * plot_h;
        let x = pad_l + i as f64 * slot + (slot - bar_w) / 2.0;
        let y = pad_t + plot_h - bh;
        let short = truncate(label, 12);
        bars.push_str(&format!(
            r#"<rect class="bar" x="{x:.1}" y="{y:.1}" width="{bar_w:.1}" height="{bh:.1}">
<title>{full}: {val}</title></rect>
<text class="axis" x="{cx:.1}" y="{ly:.1}" text-anchor="end" transform="rotate(-48 {cx:.1} {ly:.1})">{short}</text>"#,
            x = x,
            y = y,
            bar_w = bar_w,
            bh = bh.max(1.0),
            full = esc(label),
            val = val,
            cx = x + bar_w / 2.0,
            ly = h - 8.0,
            short = esc(&short)
        ));
    }
    format!(
        r#"<svg class="chart" viewBox="0 0 {w} {h}" role="img" aria-label="bar chart">{bars}</svg>"#,
        w = w,
        h = h,
        bars = bars
    )
}

fn overlay_action(name: &str) -> String {
    format!("/source/{}/overlay", name)
}

fn overlay_button(
    name: &str,
    packs: &str,
    kind: &str,
    target: &str,
    want: ReviewStatus,
    current: ReviewStatus,
    rel: &Relationship,
) -> String {
    if current == want {
        return String::new();
    }
    let label = match want {
        ReviewStatus::Accepted => "Accept",
        ReviewStatus::Rejected => "Reject",
        ReviewStatus::Pending => "Clear",
        ReviewStatus::Edited => "Save",
    };
    let class = match want {
        ReviewStatus::Accepted => "accept",
        ReviewStatus::Rejected => "reject",
        _ => "",
    };
    format!(
        r#"<form method="post" action="{action}" hx-post="{action}" hx-target='#{target}' hx-swap="outerHTML">
<input type="hidden" name="kind" value="{kind}">
<input type="hidden" name="from" value="{from}">
<input type="hidden" name="to" value="{to}">
<input type="hidden" name="status" value="{st}">
<input type="hidden" name="packs" value="{packs}">
<button class="{class}" type="submit">{label}</button>
</form>"#,
        action = overlay_action(name),
        target = esc(target),
        kind = kind,
        from = esc(&rel.from.qualified()),
        to = esc(&rel.to.qualified()),
        st = want.as_str(),
        packs = esc(packs),
        class = class,
        label = label
    )
}

fn kpi_button(
    name: &str,
    packs: &str,
    kpi: &KpiCandidate,
    want: ReviewStatus,
    current: ReviewStatus,
) -> String {
    if current == want {
        return String::new();
    }
    let (label, class) = match want {
        ReviewStatus::Accepted => ("Accept", "accept"),
        ReviewStatus::Rejected => ("Reject", "reject"),
        _ => ("Clear", ""),
    };
    let id = kpi_dom_id(&kpi.id);
    format!(
        r#"<form method="post" action="{action}" hx-post="{action}" hx-target='#{id}' hx-swap="outerHTML">
<input type="hidden" name="kind" value="kpi">
<input type="hidden" name="id" value="{kid}">
<input type="hidden" name="status" value="{st}">
<input type="hidden" name="packs" value="{packs}">
<button class="{class}" type="submit">{label}</button>
</form>"#,
        action = overlay_action(name),
        id = esc(&id),
        kid = esc(&kpi.id),
        st = want.as_str(),
        packs = esc(packs),
        class = class,
        label = label
    )
}

fn ident_button(
    name: &str,
    packs: &str,
    idn: &ColumnIdentity,
    want: ReviewStatus,
    current: ReviewStatus,
) -> String {
    if current == want {
        return String::new();
    }
    let (label, class) = match want {
        ReviewStatus::Accepted => ("Accept", "accept"),
        ReviewStatus::Rejected => ("Reject", "reject"),
        _ => ("Clear", ""),
    };
    let id = ident_dom_id(&idn.id);
    format!(
        r#"<form method="post" action="{action}" hx-post="{action}" hx-target='#{id}' hx-swap="outerHTML">
<input type="hidden" name="kind" value="identity">
<input type="hidden" name="id" value="{iid}">
<input type="hidden" name="status" value="{st}">
<input type="hidden" name="packs" value="{packs}">
<button class="{class}" type="submit">{label}</button>
</form>"#,
        action = overlay_action(name),
        id = esc(&id),
        iid = esc(&idn.id),
        st = want.as_str(),
        packs = esc(packs),
        class = class,
        label = label
    )
}

pub fn rel_dom_id(from: &str, to: &str) -> String {
    format!("rel-{}", slug(&format!("{from}_{to}")))
}

pub fn kpi_dom_id(id: &str) -> String {
    format!("kpi-{}", slug(id))
}

pub fn ident_dom_id(id: &str) -> String {
    format!("id-{}", slug(id))
}

fn slug(raw: &str) -> String {
    raw.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

fn certainty(c: Certainty) -> &'static str {
    match c {
        Certainty::Sicher => "certain",
        Certainty::Wahrscheinlich => "likely",
        Certainty::Unsicher => "uncertain",
    }
}

fn parse_number(raw: &str) -> Option<f64> {
    raw.trim().parse().ok()
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{t}…")
    }
}

pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}
