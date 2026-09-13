#!/usr/bin/env python3
"""Generate small V1 fixtures. Do not raise the budgets below without an explicit user request."""

from __future__ import annotations

import csv
import json
import random
from datetime import date, timedelta
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / "fixtures"
MAX_TABLES = 10
MAX_ROWS = 10_000
SEED = 42

REGIONS = ["Nord", "Süd", "West", "Ost", "Export"]
CATEGORIES = {
    "Electronics": ["Phones", "Laptops", "Accessories"],
    "Furniture": ["Desks", "Chairs", "Storage"],
    "Office": ["Paper", "Toner", "Stationery"],
    "Food": ["Snacks", "Drinks", "Coffee"],
}
AFFINITY = {
    "Nord": {"Electronics": 0.45, "Office": 0.25, "Furniture": 0.15, "Food": 0.15},
    "Süd": {"Furniture": 0.4, "Food": 0.25, "Electronics": 0.2, "Office": 0.15},
    "West": {"Office": 0.4, "Electronics": 0.25, "Furniture": 0.2, "Food": 0.15},
    "Ost": {"Food": 0.35, "Furniture": 0.25, "Office": 0.2, "Electronics": 0.2},
    "Export": {"Electronics": 0.55, "Furniture": 0.2, "Office": 0.15, "Food": 0.1},
}
MONTH_FACTOR = [0.8, 0.7, 0.9, 0.95, 1.0, 1.05, 0.9, 0.85, 1.1, 1.2, 1.35, 1.5]
FIRST = ["Anna", "Ben", "Clara", "David", "Elena", "Felix", "Greta", "Hans", "Ines", "Jonas"]
LAST = ["Müller", "Schmidt", "Weber", "Fischer", "Meyer", "Wagner", "Becker", "Hoffmann", "Schäfer", "Koch"]
COMPANIES = ["Nordwerk", "Südhandel", "Westlog", "Ostbau", "Alpin", "Hanse", "Rhein", "Main", "Elbe", "Donau"]
CITIES = ["Hamburg", "München", "Köln", "Berlin", "Stuttgart", "Leipzig", "Frankfurt"]
CHANNELS = ["web", "phone", "partner", "store", "edi"]
PAYMENTS = ["invoice", "card", "sepa", "paypal", "prepay"]
CURRENCIES = ["EUR", "EUR", "EUR", "CHF", "USD"]
SEGMENTS = ["smb", "midmarket", "enterprise", "public"]
BRANDS = ["Nordic", "Helix", "Contour", "Plain", "Orbit"]
COLORS = ["black", "white", "grey", "blue", "oak"]
WAREHOUSES = ["HH1", "MUC2", "CGN1", "BER3"]
LANGUAGES = ["de", "de", "de", "en", "fr"]
INDUSTRIES = ["Manufacturing", "Retail", "Software", "Logistics", "Healthcare", "Energy"]
TEAMS = ["Hunter", "Farmer", "Key Account"]


def write_csv(path: Path, rows: list[dict], fieldnames: list[str]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as fh:
        writer = csv.DictWriter(fh, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(rows)


def write_json(path: Path, data) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def pick_weighted(rng: random.Random, weights: dict[str, float]) -> str:
    names = list(weights)
    totals = [weights[n] for n in names]
    return rng.choices(names, weights=totals, k=1)[0]


def assert_budget(tables: dict[str, int], label: str) -> None:
    if len(tables) > MAX_TABLES:
        raise SystemExit(f"{label}: {len(tables)} tables exceeds {MAX_TABLES}")
    total = sum(tables.values())
    if total > MAX_ROWS:
        raise SystemExit(f"{label}: {total} rows exceeds {MAX_ROWS}")
    print(f"{label}: {len(tables)} tables, {total} rows {tables}")


def generate_ecommerce_dirty() -> None:
    rng = random.Random(SEED)
    dest = ROOT / "ecommerce_dirty"

    sales_reps = []
    for i in range(1, 13):
        region = REGIONS[(i - 1) % len(REGIONS)]
        sales_reps.append(
            {
                "id": i,
                "name": f"{FIRST[i % len(FIRST)]} {LAST[i % len(LAST)]}",
                "email": f"rep{i}@sales.example",
                "phone": f"+49-40-10{i:04d}",
                "region": region,
                "office_city": CITIES[(i - 1) % len(CITIES)],
                "team": TEAMS[i % len(TEAMS)],
                "hire_date": date(2018, 1, 1) + timedelta(days=i * 80),
                "last_review": date(2018, 1, 1) + timedelta(days=i * 80 + 365),
                "active": "true" if i != 11 else "false",
                "quota": 400000 + i * 25000,
                "commission_pct": round(0.04 + (i % 5) * 0.01, 2),
                "manager_id": "" if i <= 3 else 1 + ((i - 1) % 3),
            }
        )

    suppliers = []
    for i in range(1, 21):
        suppliers.append(
            {
                "supplier_id": i,
                "name": f"Supplier {i:02d}",
                "country": rng.choice(["DE", "AT", "PL", "NL", "CN"]),
                "city": rng.choice(CITIES + ["Shanghai", "Rotterdam", "Vienna"]),
                "postal_code": f"{10000 + i * 37}",
                "lead_time_days": rng.choice([7, 10, 14, 21, 30]),
                "rating": rng.choice([2, 3, 3, 4, 4, 5]),
                "currency": rng.choice(["EUR", "EUR", "USD", "CNY"]),
                "payment_terms": rng.choice(["NET30", "NET45", "NET60", "PREPAY"]),
                "is_preferred": "true" if i % 4 == 0 else "false",
                "min_order_value": rng.choice([0, 250, 500, 1000]),
                "contact_email": f"purchasing{i}@supplier.example",
            }
        )

    products = []
    sku_list = []
    n = 1
    for category, subs in CATEGORIES.items():
        for _ in range(30):
            sku = f"SKU-{n:04d}"
            sku_list.append(sku)
            cost = round(rng.uniform(4, 240), 2)
            products.append(
                {
                    "sku": sku,
                    "ean": f"400{n:010d}",
                    "product_name": f"{category} Item {n}",
                    "brand": rng.choice(BRANDS),
                    "category": category,
                    "subcategory": rng.choice(subs),
                    "color": rng.choice(COLORS),
                    "supplier_id": rng.choice(suppliers)["supplier_id"],
                    "origin_country": rng.choice(["DE", "CN", "PL", "IT"]),
                    "unit_cost": cost,
                    "list_price": round(cost * rng.uniform(1.25, 1.9), 2),
                    "vat_rate": rng.choice([0.07, 0.19, 0.19, 0.19]),
                    "weight_kg": round(rng.uniform(0.05, 28.0), 3),
                    "stock_qty": rng.randint(0, 800),
                    "reorder_level": rng.choice([10, 25, 50, 100]),
                    "warranty_months": rng.choice([0, 12, 24, 36]),
                    "discontinued": "true" if rng.random() < 0.08 else "false",
                }
            )
            n += 1

    customers = []
    for i in range(1, 351):
        region = rng.choices(REGIONS, weights=[0.3, 0.25, 0.2, 0.15, 0.1])[0]
        email = f"c{i}@example.com"
        if i in (10, 11):
            email = "dup-a@example.com"
        if i in (20, 21):
            email = "dup-b@example.com"
        sales_rep = rng.choice([r["id"] for r in sales_reps if r["region"] == region] or [None])
        if rng.random() < 0.05:
            sales_rep = ""
        created = date(2022, 1, 1) + timedelta(days=rng.randint(0, 800))
        customers.append(
            {
                "id": i,
                "customer_name": f"{rng.choice(COMPANIES)} {i:03d} GmbH",
                "legal_form": rng.choice(["GmbH", "AG", "KG", "e.K."]),
                "region": region,
                "country": "DE" if region != "Export" else rng.choice(["FR", "IT", "US", "UK"]),
                "city": rng.choice(CITIES),
                "postal_code": f"{20000 + (i * 17) % 70000}",
                "segment": rng.choice(SEGMENTS),
                "industry": rng.choice(INDUSTRIES),
                "language": rng.choice(LANGUAGES),
                "sales_rep_id": sales_rep,
                "created_at": created,
                "last_contact_at": created + timedelta(days=rng.randint(0, 400)),
                "is_active": "false" if rng.random() < 0.12 else "true",
                "credit_limit": rng.choice([5000, 15000, 50000, 150000, 500000]),
                "employee_count": rng.choice([4, 12, 35, 80, 220, 900]),
                "vat_id": f"DE{100000000 + i}",
                "email": email,
                "phone": f"+49-{40 + (i % 50)}-{100000 + i}",
            }
        )

    start = date(2024, 1, 1)
    end = date(2026, 8, 1)
    span = (end - start).days
    orders = []
    active_customers = [c for c in customers if c["is_active"] == "true"]
    inactive_customers = [c for c in customers if c["is_active"] == "false"]

    for i in range(1, 2201):
        if rng.random() < 0.08 and inactive_customers:
            cust = rng.choice(inactive_customers)
            order_date = start + timedelta(days=rng.randint(0, 400))
        else:
            cust = rng.choice(active_customers)
            order_date = start + timedelta(days=rng.randint(0, span - 1))
        status = rng.choices(["open", "shipped", "delivered", "cancelled"], weights=[0.08, 0.22, 0.64, 0.06])[0]
        sales_rep = cust["sales_rep_id"]
        customer_nr = cust["id"]
        roll = rng.random()
        if roll < 0.02:
            customer_nr = ""
        elif roll < 0.03:
            customer_nr = 999999
        discount = rng.choice([0, 0, 0, 0, 15, 25, 40, 80])
        shipping = round(rng.choice([0, 4.9, 7.9, 12.9]), 2)
        packing = round(rng.choice([0, 0, 1.5, 2.9]), 2)
        promised = order_date + timedelta(days=rng.randint(2, 14))
        shipped = "" if status in ("open", "cancelled") else promised - timedelta(days=rng.randint(0, 3))
        orders.append(
            {
                "order_id": i,
                "customer_nr": customer_nr,
                "order_date": order_date,
                "promised_date": promised,
                "shipped_date": shipped,
                "status": status,
                "channel": rng.choice(CHANNELS),
                "priority": rng.choice(["normal", "normal", "high", "rush"]),
                "currency": rng.choice(CURRENCIES),
                "payment_method": rng.choice(PAYMENTS),
                "warehouse": rng.choice(WAREHOUSES),
                "billing_city": cust["city"],
                "shipping_city": cust["city"] if rng.random() < 0.85 else rng.choice(CITIES),
                "net_amount": 0.0,
                "discount_amount": discount,
                "shipping_cost": shipping,
                "packing_cost": packing,
                "sales_rep_id": sales_rep if sales_rep != "" else rng.choice([r["id"] for r in sales_reps]),
                "po_number": f"PO-{2024 + (i % 3)}-{i:05d}" if rng.random() < 0.55 else "",
                "notes": rng.choice(["", "", "call before delivery", "leave at gate"]),
                "_region": cust["region"],
                "_month": order_date.month,
            }
        )

    products_by_cat = {}
    for p in products:
        products_by_cat.setdefault(p["category"], []).append(p)

    order_items = []
    item_id = 1
    target_items = 4800
    for idx, order in enumerate(orders):
        # 400 orders with 3 lines, 1800 with 2 lines → 4800 items.
        n_items = 3 if idx < 400 else 2
        region = order["_region"]
        line_sum = 0.0
        for _ in range(n_items):
            category = pick_weighted(rng, AFFINITY[region])
            product = rng.choice(products_by_cat[category])
            qty = rng.randint(1, 6)
            unit_price = float(product["list_price"]) * rng.uniform(0.92, 1.0)
            line_discount = round(rng.choice([0, 0, 0, 2.5, 5.0]), 2)
            line_total = round(qty * unit_price - line_discount, 2)
            line_sum += line_total
            order_items.append(
                {
                    "item_id": item_id,
                    "order_id": order["order_id"],
                    "product_sku": product["sku"],
                    "quantity": qty,
                    "unit_price": round(unit_price, 2),
                    "line_discount": line_discount,
                    "line_total": line_total,
                    "warehouse": order["warehouse"],
                    "line_status": rng.choice(["open", "picked", "shipped", "invoiced"]),
                    "gift_wrap": "true" if rng.random() < 0.04 else "false",
                    "weight_g": round(float(product["weight_kg"]) * qty * 1000, 1),
                }
            )
            item_id += 1
            if item_id - 1 >= target_items:
                break
        factor = MONTH_FACTOR[order["_month"] - 1]
        rep = next(r for r in sales_reps if r["id"] == order["sales_rep_id"])
        perf = 0.7 + (rep["id"] % 12) * 0.05
        net = round((line_sum - float(order["discount_amount"])) * factor * perf, 2)
        order["net_amount"] = max(net, 1.0)
        if item_id - 1 >= target_items:
            # fill remaining orders with dummy 0-item skip; they already have net from lines
            pass

    # Guarantee remaining orders still have at least the net computed; if some have 0 items, copy last net.
    for order in orders:
        if order["net_amount"] == 0:
            order["net_amount"] = 25.0

    outlier_ids = {50, 120, 800, 1400, 2000}
    for order in orders:
        if order["order_id"] in outlier_ids:
            order["net_amount"] = round(float(order["net_amount"]) * 15, 2)

    invoices = []
    invoice_id = 1
    for order in orders:
        if invoice_id > 1600:
            break
        if rng.random() < 0.25:
            continue
        status = rng.choices(["open", "paid", "cancelled"], weights=[0.18, 0.76, 0.06])[0]
        inv_date = order["order_date"] + timedelta(days=rng.randint(0, 14))
        tax = round(float(order["net_amount"]) * 0.19, 2)
        gross = round(float(order["net_amount"]) + tax, 2)
        if status == "paid":
            paid = gross
        elif status == "cancelled":
            paid = 0.0
        else:
            paid = round(gross * rng.choice([0.0, 0.0, 0.4, 0.5]), 2)
        remaining = round(gross - paid, 2)
        invoices.append(
            {
                "invoice_id": invoice_id,
                "order_id": order["order_id"],
                "invoice_date": inv_date,
                "due_date": inv_date + timedelta(days=rng.choice([14, 30, 45])),
                "currency": order["currency"],
                "payment_method": order["payment_method"],
                "gross_amount": gross,
                "tax_amount": tax,
                "paid_amount": paid,
                "remaining_amount": remaining,
                "status": status,
                "dunning_level": 0 if status == "paid" else rng.choice([0, 0, 1, 2]),
                "cancelled_at": inv_date + timedelta(days=3) if status == "cancelled" else "",
            }
        )
        invoice_id += 1

    returns = []
    for i in range(1, 101):
        item = rng.choice(order_items)
        qty = max(1, min(item["quantity"], rng.randint(1, 2)))
        refund = round(qty * float(item["unit_price"]) * 0.9, 2)
        restock_fee = round(rng.choice([0, 0, 2.5, 5.0]), 2)
        returns.append(
            {
                "return_id": i,
                "order_item_id": item["item_id"],
                "return_date": date(2024, 6, 1) + timedelta(days=rng.randint(0, 700)),
                "quantity": qty,
                "reason": rng.choice(["damaged", "wrong_item", "late", "changed_mind"]),
                "condition": rng.choice(["unopened", "used", "damaged"]),
                "warehouse": rng.choice(WAREHOUSES),
                "carrier": rng.choice(["dhl", "ups", "dpd", "self"]),
                "tracking_nr": f"RR{i:010d}" if rng.random() < 0.7 else "",
                "restocked": "true" if rng.random() < 0.65 else "false",
                "refund_amount": refund,
                "restock_fee": restock_fee,
                "net_refund": round(refund - restock_fee, 2),
            }
        )

    for order in orders:
        order.pop("_region", None)
        order.pop("_month", None)

    schema = {
        "name": "ecommerce_dirty",
        "tables": {
            "sales_reps": {
                "columns": {
                    "id": {"type": "integer"},
                    "name": {"type": "text"},
                    "email": {"type": "text"},
                    "phone": {"type": "text"},
                    "region": {"type": "text"},
                    "office_city": {"type": "text"},
                    "team": {"type": "text"},
                    "hire_date": {"type": "date"},
                    "last_review": {"type": "date"},
                    "active": {"type": "boolean"},
                    "quota": {"type": "decimal"},
                    "commission_pct": {"type": "decimal"},
                    "manager_id": {"type": "integer", "nullable": True},
                },
                "primary_key": ["id"],
                "foreign_keys": [],
            },
            "suppliers": {
                "columns": {
                    "supplier_id": {"type": "integer"},
                    "name": {"type": "text"},
                    "country": {"type": "text"},
                    "city": {"type": "text"},
                    "postal_code": {"type": "text"},
                    "lead_time_days": {"type": "integer"},
                    "rating": {"type": "integer"},
                    "currency": {"type": "text"},
                    "payment_terms": {"type": "text"},
                    "is_preferred": {"type": "boolean"},
                    "min_order_value": {"type": "decimal"},
                    "contact_email": {"type": "text"},
                },
                "primary_key": ["supplier_id"],
            },
            "customers": {
                "columns": {
                    "id": {"type": "integer"},
                    "customer_name": {"type": "text"},
                    "legal_form": {"type": "text"},
                    "region": {"type": "text"},
                    "country": {"type": "text"},
                    "city": {"type": "text"},
                    "postal_code": {"type": "text"},
                    "segment": {"type": "text"},
                    "industry": {"type": "text"},
                    "language": {"type": "text"},
                    "sales_rep_id": {"type": "integer", "nullable": True},
                    "created_at": {"type": "date"},
                    "last_contact_at": {"type": "date"},
                    "is_active": {"type": "boolean"},
                    "credit_limit": {"type": "decimal"},
                    "employee_count": {"type": "integer"},
                    "vat_id": {"type": "text"},
                    "email": {"type": "text"},
                    "phone": {"type": "text"},
                },
                "primary_key": ["id"],
                "foreign_keys": [
                    {
                        "columns": ["sales_rep_id"],
                        "ref_table": "sales_reps",
                        "ref_columns": ["id"],
                    }
                ],
            },
            "products": {
                "columns": {
                    "sku": {"type": "text"},
                    "ean": {"type": "text"},
                    "product_name": {"type": "text"},
                    "brand": {"type": "text"},
                    "category": {"type": "text"},
                    "subcategory": {"type": "text"},
                    "color": {"type": "text"},
                    "supplier_id": {"type": "integer"},
                    "origin_country": {"type": "text"},
                    "unit_cost": {"type": "decimal"},
                    "list_price": {"type": "decimal"},
                    "vat_rate": {"type": "decimal"},
                    "weight_kg": {"type": "decimal"},
                    "stock_qty": {"type": "integer"},
                    "reorder_level": {"type": "integer"},
                    "warranty_months": {"type": "integer"},
                    "discontinued": {"type": "boolean"},
                },
                "primary_key": ["sku"],
                "foreign_keys": [
                    {
                        "columns": ["supplier_id"],
                        "ref_table": "suppliers",
                        "ref_columns": ["supplier_id"],
                    }
                ],
            },
            "orders": {
                "columns": {
                    "order_id": {"type": "integer"},
                    "customer_nr": {"type": "integer", "nullable": True},
                    "order_date": {"type": "date"},
                    "promised_date": {"type": "date"},
                    "shipped_date": {"type": "date", "nullable": True},
                    "status": {"type": "text"},
                    "channel": {"type": "text"},
                    "priority": {"type": "text"},
                    "currency": {"type": "text"},
                    "payment_method": {"type": "text"},
                    "warehouse": {"type": "text"},
                    "billing_city": {"type": "text"},
                    "shipping_city": {"type": "text"},
                    "net_amount": {"type": "decimal"},
                    "discount_amount": {"type": "decimal"},
                    "shipping_cost": {"type": "decimal"},
                    "packing_cost": {"type": "decimal"},
                    "sales_rep_id": {"type": "integer", "nullable": True},
                    "po_number": {"type": "text", "nullable": True},
                    "notes": {"type": "text", "nullable": True},
                },
                "primary_key": ["order_id"],
                "foreign_keys": [],
            },
            "order_items": {
                "columns": {
                    "item_id": {"type": "integer"},
                    "order_id": {"type": "integer"},
                    "product_sku": {"type": "text"},
                    "quantity": {"type": "integer"},
                    "unit_price": {"type": "decimal"},
                    "line_discount": {"type": "decimal"},
                    "line_total": {"type": "decimal"},
                    "warehouse": {"type": "text"},
                    "line_status": {"type": "text"},
                    "gift_wrap": {"type": "boolean"},
                    "weight_g": {"type": "decimal"},
                },
                "primary_key": ["item_id"],
                "foreign_keys": [
                    {
                        "columns": ["order_id"],
                        "ref_table": "orders",
                        "ref_columns": ["order_id"],
                    }
                ],
            },
            "invoices": {
                "columns": {
                    "invoice_id": {"type": "integer"},
                    "order_id": {"type": "integer"},
                    "invoice_date": {"type": "date"},
                    "due_date": {"type": "date"},
                    "currency": {"type": "text"},
                    "payment_method": {"type": "text"},
                    "gross_amount": {"type": "decimal"},
                    "tax_amount": {"type": "decimal"},
                    "paid_amount": {"type": "decimal"},
                    "remaining_amount": {"type": "decimal"},
                    "status": {"type": "text"},
                    "dunning_level": {"type": "integer"},
                    "cancelled_at": {"type": "date", "nullable": True},
                },
                "primary_key": ["invoice_id"],
                "foreign_keys": [
                    {
                        "columns": ["order_id"],
                        "ref_table": "orders",
                        "ref_columns": ["order_id"],
                    }
                ],
            },
            "returns": {
                "columns": {
                    "return_id": {"type": "integer"},
                    "order_item_id": {"type": "integer"},
                    "return_date": {"type": "date"},
                    "quantity": {"type": "integer"},
                    "reason": {"type": "text"},
                    "condition": {"type": "text"},
                    "warehouse": {"type": "text"},
                    "carrier": {"type": "text"},
                    "tracking_nr": {"type": "text", "nullable": True},
                    "restocked": {"type": "boolean"},
                    "refund_amount": {"type": "decimal"},
                    "restock_fee": {"type": "decimal"},
                    "net_refund": {"type": "decimal"},
                },
                "primary_key": ["return_id"],
                "foreign_keys": [],
            },
        },
    }
    ground_truth = {
        "relationships": [
            {"from": "customers.sales_rep_id", "to": "sales_reps.id"},
            {"from": "products.supplier_id", "to": "suppliers.supplier_id"},
            {"from": "order_items.order_id", "to": "orders.order_id"},
            {"from": "invoices.order_id", "to": "orders.order_id"},
            {"from": "orders.customer_nr", "to": "customers.id"},
            {"from": "orders.sales_rep_id", "to": "sales_reps.id"},
            {"from": "order_items.product_sku", "to": "products.sku"},
            {"from": "returns.order_item_id", "to": "order_items.item_id"},
        ],
        "kpis": [
            {"name": "Revenue", "definition": "SUM(orders.net_amount)"},
            {"name": "Order Count", "definition": "COUNT(DISTINCT orders.order_id)"},
            {"name": "Customer Count", "definition": "COUNT(DISTINCT orders.customer_nr)"},
            {"name": "Units Sold", "definition": "SUM(order_items.quantity)"},
        ],
    }

    write_json(dest / "schema.json", schema)
    write_json(dest / "ground_truth.json", ground_truth)
    write_csv(dest / "sales_reps.csv", sales_reps, list(sales_reps[0]))
    write_csv(dest / "suppliers.csv", suppliers, list(suppliers[0]))
    write_csv(dest / "customers.csv", customers, list(customers[0]))
    write_csv(dest / "products.csv", products, list(products[0]))
    write_csv(dest / "orders.csv", [{k: v for k, v in o.items()} for o in orders], [
        "order_id", "customer_nr", "order_date", "promised_date", "shipped_date", "status",
        "channel", "priority", "currency", "payment_method", "warehouse", "billing_city",
        "shipping_city", "net_amount", "discount_amount", "shipping_cost", "packing_cost",
        "sales_rep_id", "po_number", "notes",
    ])
    write_csv(dest / "order_items.csv", order_items, list(order_items[0]))
    write_csv(dest / "invoices.csv", invoices, list(invoices[0]))
    write_csv(dest / "returns.csv", returns, list(returns[0]))
    counts = {
        "sales_reps": len(sales_reps),
        "suppliers": len(suppliers),
        "customers": len(customers),
        "products": len(products),
        "orders": len(orders),
        "order_items": len(order_items),
        "invoices": len(invoices),
        "returns": len(returns),
    }
    assert_budget(counts, "ecommerce_dirty")


def generate_ecommerce_clean() -> None:
    rng = random.Random(SEED + 1)
    dest = ROOT / "ecommerce_clean"
    customers = [
        {
            "customer_id": i,
            "name": f"Customer {i}",
            "email": f"c{i}@clean.example",
            "region": REGIONS[i % 5],
            "country": "DE",
            "city": CITIES[i % len(CITIES)],
            "segment": SEGMENTS[i % len(SEGMENTS)],
            "is_active": "true" if i % 11 else "false",
            "created_at": date(2025, 1, 1) + timedelta(days=i),
        }
        for i in range(1, 81)
    ]
    products = [
        {
            "product_id": i,
            "sku": f"C-{i:04d}",
            "name": f"Product {i}",
            "brand": BRANDS[i % len(BRANDS)],
            "category": list(CATEGORIES)[i % 4],
            "subcategory": "Standard",
            "cost": round(6 + i * 0.8, 2),
            "price": round(10 + i * 1.5, 2),
            "in_stock": rng.randint(5, 200),
            "discontinued": "false",
        }
        for i in range(1, 41)
    ]
    orders = []
    items = []
    item_id = 1
    for i in range(1, 201):
        cust = rng.choice(customers)
        order_date = date(2025, 1, 1) + timedelta(days=rng.randint(0, 400))
        net = 0.0
        warehouse = rng.choice(WAREHOUSES)
        for _ in range(rng.randint(1, 3)):
            prod = rng.choice(products)
            qty = rng.randint(1, 4)
            unit_price = float(prod["price"])
            line = round(qty * unit_price, 2)
            net += line
            items.append(
                {
                    "item_id": item_id,
                    "order_id": i,
                    "product_id": prod["product_id"],
                    "quantity": qty,
                    "unit_price": unit_price,
                    "line_amount": line,
                    "warehouse": warehouse,
                }
            )
            item_id += 1
        orders.append(
            {
                "order_id": i,
                "customer_id": cust["customer_id"],
                "order_date": order_date,
                "shipped_date": order_date + timedelta(days=rng.randint(1, 8)),
                "status": rng.choice(["open", "paid", "shipped"]),
                "channel": rng.choice(CHANNELS),
                "currency": "EUR",
                "payment_method": rng.choice(PAYMENTS),
                "warehouse": warehouse,
                "net_amount": round(net, 2),
            }
        )
    schema = {
        "name": "ecommerce_clean",
        "tables": {
            "customers": {
                "columns": {
                    "customer_id": {"type": "integer"},
                    "name": {"type": "text"},
                    "email": {"type": "text"},
                    "region": {"type": "text"},
                    "country": {"type": "text"},
                    "city": {"type": "text"},
                    "segment": {"type": "text"},
                    "is_active": {"type": "boolean"},
                    "created_at": {"type": "date"},
                },
                "primary_key": ["customer_id"],
            },
            "products": {
                "columns": {
                    "product_id": {"type": "integer"},
                    "sku": {"type": "text"},
                    "name": {"type": "text"},
                    "brand": {"type": "text"},
                    "category": {"type": "text"},
                    "subcategory": {"type": "text"},
                    "cost": {"type": "decimal"},
                    "price": {"type": "decimal"},
                    "in_stock": {"type": "integer"},
                    "discontinued": {"type": "boolean"},
                },
                "primary_key": ["product_id"],
            },
            "orders": {
                "columns": {
                    "order_id": {"type": "integer"},
                    "customer_id": {"type": "integer"},
                    "order_date": {"type": "date"},
                    "shipped_date": {"type": "date"},
                    "status": {"type": "text"},
                    "channel": {"type": "text"},
                    "currency": {"type": "text"},
                    "payment_method": {"type": "text"},
                    "warehouse": {"type": "text"},
                    "net_amount": {"type": "decimal"},
                },
                "primary_key": ["order_id"],
                "foreign_keys": [
                    {"columns": ["customer_id"], "ref_table": "customers", "ref_columns": ["customer_id"]}
                ],
            },
            "order_items": {
                "columns": {
                    "item_id": {"type": "integer"},
                    "order_id": {"type": "integer"},
                    "product_id": {"type": "integer"},
                    "quantity": {"type": "integer"},
                    "unit_price": {"type": "decimal"},
                    "line_amount": {"type": "decimal"},
                    "warehouse": {"type": "text"},
                },
                "primary_key": ["item_id"],
                "foreign_keys": [
                    {"columns": ["order_id"], "ref_table": "orders", "ref_columns": ["order_id"]},
                    {"columns": ["product_id"], "ref_table": "products", "ref_columns": ["product_id"]},
                ],
            },
        },
    }
    ground_truth = {
        "relationships": [
            {"from": "orders.customer_id", "to": "customers.customer_id"},
            {"from": "order_items.order_id", "to": "orders.order_id"},
            {"from": "order_items.product_id", "to": "products.product_id"},
        ],
        "kpis": [
            {"name": "Revenue", "definition": "SUM(orders.net_amount)"},
            {"name": "Order Count", "definition": "COUNT(DISTINCT orders.order_id)"},
            {"name": "Units Sold", "definition": "SUM(order_items.quantity)"},
        ],
    }
    write_json(dest / "schema.json", schema)
    write_json(dest / "ground_truth.json", ground_truth)
    write_csv(dest / "customers.csv", customers, list(customers[0]))
    write_csv(dest / "products.csv", products, list(products[0]))
    write_csv(dest / "orders.csv", orders, list(orders[0]))
    write_csv(dest / "order_items.csv", items, list(items[0]))
    assert_budget(
        {
            "customers": len(customers),
            "products": len(products),
            "orders": len(orders),
            "order_items": len(items),
        },
        "ecommerce_clean",
    )


def generate_crm() -> None:
    rng = random.Random(SEED + 2)
    dest = ROOT / "crm"
    accounts = [
        {
            "account_id": i,
            "name": f"Account {i}",
            "industry": rng.choice(INDUSTRIES),
            "region": rng.choice(REGIONS[:4]),
            "country": "DE",
            "city": rng.choice(CITIES),
            "employees": rng.choice([8, 25, 80, 250, 1200]),
            "annual_revenue": rng.randint(200_000, 8_000_000),
            "status": rng.choice(["active", "active", "active", "churn_risk"]),
            "owner": rng.choice(FIRST),
            "website": f"https://account{i}.example",
            "created_at": date(2023, 1, 1) + timedelta(days=rng.randint(0, 900)),
        }
        for i in range(1, 41)
    ]
    contacts = [
        {
            "id": i,
            "account_nr": rng.choice(accounts)["account_id"],
            "first_name": rng.choice(FIRST),
            "last_name": rng.choice(LAST),
            "email": f"p{i}@crm.test",
            "phone": f"+49-30-{200000 + i}",
            "mobile": f"+49-170-{300000 + i}" if rng.random() < 0.7 else "",
            "role": rng.choice(["buyer", "champion", "user", "blocker"]),
            "department": rng.choice(["Purchasing", "IT", "Finance", "Ops"]),
            "title": rng.choice(["Manager", "Director", "Specialist", "VP"]),
            "city": rng.choice(CITIES),
            "language": rng.choice(LANGUAGES),
            "is_primary": "true" if i % 5 == 0 else "false",
            "last_seen": date(2026, 1, 1) + timedelta(days=rng.randint(0, 180)),
        }
        for i in range(1, 81)
    ]
    opps = []
    for i in range(1, 101):
        amount = rng.randint(5_000, 250_000)
        probability = rng.choice([10, 20, 40, 60, 80, 100])
        opps.append(
            {
                "opp_id": i,
                "account_id": rng.choice(accounts)["account_id"],
                "name": f"Opportunity {i}",
                "stage": rng.choice(["prospect", "qualify", "proposal", "won", "lost"]),
                "type": rng.choice(["new", "upsell", "renewal"]),
                "source": rng.choice(["inbound", "outbound", "partner", "event"]),
                "amount": amount,
                "probability": probability,
                "weighted_amount": round(amount * probability / 100.0, 2),
                "close_date": date(2026, 1, 1) + timedelta(days=rng.randint(0, 200)),
                "created_at": date(2025, 6, 1) + timedelta(days=rng.randint(0, 200)),
                "owner": rng.choice(FIRST),
                "competitor": rng.choice(["", "", "Helix", "Northwind"]),
            }
        )
    activities = [
        {
            "activity_id": i,
            "contact_id": rng.choice(contacts)["id"],
            "type": rng.choice(["call", "email", "meeting", "demo"]),
            "activity_date": date(2026, 1, 1) + timedelta(days=rng.randint(0, 180)),
            "duration_min": rng.choice([15, 30, 45, 60, 90]),
            "outcome": rng.choice(["completed", "no_show", "rescheduled"]),
            "completed": "true" if rng.random() < 0.8 else "false",
            "notes": rng.choice(["", "follow up", "send quote", "intro call"]),
        }
        for i in range(1, 201)
    ]
    schema = {
        "name": "crm",
        "tables": {
            "accounts": {
                "columns": {
                    "account_id": {"type": "integer"},
                    "name": {"type": "text"},
                    "industry": {"type": "text"},
                    "region": {"type": "text"},
                    "country": {"type": "text"},
                    "city": {"type": "text"},
                    "employees": {"type": "integer"},
                    "annual_revenue": {"type": "decimal"},
                    "status": {"type": "text"},
                    "owner": {"type": "text"},
                    "website": {"type": "text"},
                    "created_at": {"type": "date"},
                },
                "primary_key": ["account_id"],
            },
            "contacts": {
                "columns": {
                    "id": {"type": "integer"},
                    "account_nr": {"type": "integer"},
                    "first_name": {"type": "text"},
                    "last_name": {"type": "text"},
                    "email": {"type": "text"},
                    "phone": {"type": "text"},
                    "mobile": {"type": "text", "nullable": True},
                    "role": {"type": "text"},
                    "department": {"type": "text"},
                    "title": {"type": "text"},
                    "city": {"type": "text"},
                    "language": {"type": "text"},
                    "is_primary": {"type": "boolean"},
                    "last_seen": {"type": "date"},
                },
                "primary_key": ["id"],
                "foreign_keys": [],
            },
            "opportunities": {
                "columns": {
                    "opp_id": {"type": "integer"},
                    "account_id": {"type": "integer"},
                    "name": {"type": "text"},
                    "stage": {"type": "text"},
                    "type": {"type": "text"},
                    "source": {"type": "text"},
                    "amount": {"type": "decimal"},
                    "probability": {"type": "integer"},
                    "weighted_amount": {"type": "decimal"},
                    "close_date": {"type": "date"},
                    "created_at": {"type": "date"},
                    "owner": {"type": "text"},
                    "competitor": {"type": "text", "nullable": True},
                },
                "primary_key": ["opp_id"],
                "foreign_keys": [],
            },
            "activities": {
                "columns": {
                    "activity_id": {"type": "integer"},
                    "contact_id": {"type": "integer"},
                    "type": {"type": "text"},
                    "activity_date": {"type": "date"},
                    "duration_min": {"type": "integer"},
                    "outcome": {"type": "text"},
                    "completed": {"type": "boolean"},
                    "notes": {"type": "text", "nullable": True},
                },
                "primary_key": ["activity_id"],
                "foreign_keys": [
                    {"columns": ["contact_id"], "ref_table": "contacts", "ref_columns": ["id"]}
                ],
            },
        },
    }
    ground_truth = {
        "relationships": [
            {"from": "contacts.account_nr", "to": "accounts.account_id"},
            {"from": "opportunities.account_id", "to": "accounts.account_id"},
            {"from": "activities.contact_id", "to": "contacts.id"},
        ],
        "kpis": [
            {"name": "Revenue", "definition": "SUM(opportunities.amount)"},
        ],
    }
    write_json(dest / "schema.json", schema)
    write_json(dest / "ground_truth.json", ground_truth)
    write_csv(dest / "accounts.csv", accounts, list(accounts[0]))
    write_csv(dest / "contacts.csv", contacts, list(contacts[0]))
    write_csv(dest / "opportunities.csv", opps, list(opps[0]))
    write_csv(dest / "activities.csv", activities, list(activities[0]))
    assert_budget(
        {
            "accounts": len(accounts),
            "contacts": len(contacts),
            "opportunities": len(opps),
            "activities": len(activities),
        },
        "crm",
    )


def generate_edge_cases() -> None:
    rng = random.Random(SEED + 3)
    dest = ROOT / "edge_cases"
    parents = [
        {
            "parent_id": i,
            "name": f"P{i}",
            "city": CITIES[i % len(CITIES)],
            "opened_on": date(2020, 1, 1) + timedelta(days=i * 40),
            "active": "true" if i != 4 else "false",
        }
        for i in range(1, 6)
    ]
    children = [
        {
            "child_id": i,
            "parent_id": 1 + (i % 5),
            "name": f"C{i}",
            "note": rng.choice(["ok", "hold", "review"]),
            "weight": round(rng.uniform(0.2, 4.5), 2),
        }
        for i in range(1, 13)
    ]
    decoys = [
        {"parent_id": f"X{i}", "label": f"decoy {i}", "comment": "not a parent key"}
        for i in range(1, 4)
    ]
    messy = [
        {"id": 1, "maybe_value": "", "status": "open", "flag": "true", "comment": "all nulls"},
        {"id": 2, "maybe_value": "", "status": "open", "flag": "false", "comment": ""},
        {"id": 3, "maybe_value": "", "status": "closed", "flag": "true", "comment": "done"},
    ]
    schema = {
        "name": "edge_cases",
        "tables": {
            "parents": {
                "columns": {
                    "parent_id": {"type": "integer"},
                    "name": {"type": "text"},
                    "city": {"type": "text"},
                    "opened_on": {"type": "date"},
                    "active": {"type": "boolean"},
                },
                "primary_key": ["parent_id"],
            },
            "children": {
                "columns": {
                    "child_id": {"type": "integer"},
                    "parent_id": {"type": "integer"},
                    "name": {"type": "text"},
                    "note": {"type": "text"},
                    "weight": {"type": "decimal"},
                },
                "primary_key": ["child_id"],
                "foreign_keys": [],
            },
            "decoys": {
                "columns": {
                    "parent_id": {"type": "text"},
                    "label": {"type": "text"},
                    "comment": {"type": "text"},
                },
                "primary_key": ["parent_id"],
            },
            "empty_things": {
                "columns": {
                    "id": {"type": "integer"},
                    "label": {"type": "text"},
                    "extra": {"type": "text", "nullable": True},
                },
                "primary_key": ["id"],
            },
            "messy": {
                "columns": {
                    "id": {"type": "integer"},
                    "maybe_value": {"type": "decimal", "nullable": True},
                    "status": {"type": "text"},
                    "flag": {"type": "boolean"},
                    "comment": {"type": "text", "nullable": True},
                },
                "primary_key": ["id"],
            },
            "computed": {
                "columns": {
                    "row_id": {"type": "integer"},
                    "alpha": {"type": "decimal"},
                    "beta": {"type": "decimal"},
                    "gamma": {"type": "decimal"},
                    "delta": {"type": "decimal"},
                    "eps": {"type": "decimal"},
                    "eta": {"type": "decimal"},
                    "label": {"type": "text"},
                },
                "primary_key": ["row_id"],
            },
            "bundles": {
                "columns": {
                    "bundle_id": {"type": "integer"},
                    "span": {"type": "decimal"},
                    "color": {"type": "text"},
                    "sealed": {"type": "boolean"},
                },
                "primary_key": ["bundle_id"],
            },
            "pieces": {
                "columns": {
                    "piece_id": {"type": "integer"},
                    "bundle_id": {"type": "integer"},
                    "width": {"type": "decimal"},
                    "material": {"type": "text"},
                    "scrap": {"type": "decimal"},
                },
                "primary_key": ["piece_id"],
                "foreign_keys": [
                    {
                        "columns": ["bundle_id"],
                        "ref_table": "bundles",
                        "ref_columns": ["bundle_id"],
                    }
                ],
            },
        },
    }
    computed = []
    for i in range(1, 13):
        alpha = float(i * 2)
        beta = round(i * 0.5 + 1.0, 2)
        computed.append(
            {
                "row_id": i,
                "alpha": alpha,
                "beta": beta,
                "gamma": round(alpha + beta, 2),
                "delta": round(alpha * beta, 2),
                "eps": round(alpha * 0.19, 2),
                "eta": round(rng.uniform(3, 40), 2),
                "label": rng.choice(["red", "blue", "green"]),
            }
        )
    bundles = []
    pieces = []
    piece_id = 1
    widths = [
        [2, 3],
        [4, 1],
        [10],
        [1, 2, 3],
        [7],
        [2, 2, 2],
        [5, 5],
        [1, 9],
        [3, 3, 3],
        [4],
    ]
    for bundle_id, group in enumerate(widths, start=1):
        bundles.append(
            {
                "bundle_id": bundle_id,
                "span": round(sum(group), 2),
                "color": rng.choice(COLORS),
                "sealed": "true" if bundle_id % 2 else "false",
            }
        )
        for w in group:
            pieces.append(
                {
                    "piece_id": piece_id,
                    "bundle_id": bundle_id,
                    "width": w,
                    "material": rng.choice(["oak", "steel", "plastic"]),
                    "scrap": round(rng.uniform(0.0, 0.4), 2),
                }
            )
            piece_id += 1
    ground_truth = {
        "relationships": [
            {"from": "children.parent_id", "to": "parents.parent_id"},
            {"from": "pieces.bundle_id", "to": "bundles.bundle_id"},
        ],
        "kpis": [],
    }
    write_json(dest / "schema.json", schema)
    write_json(dest / "ground_truth.json", ground_truth)
    write_csv(dest / "parents.csv", parents, list(parents[0]))
    write_csv(dest / "children.csv", children, list(children[0]))
    write_json(dest / "decoys.json", decoys)
    write_csv(dest / "empty_things.csv", [], ["id", "label", "extra"])
    write_csv(dest / "messy.csv", messy, list(messy[0]))
    write_csv(dest / "computed.csv", computed, list(computed[0]))
    write_csv(dest / "bundles.csv", bundles, list(bundles[0]))
    write_csv(dest / "pieces.csv", pieces, list(pieces[0]))
    assert_budget(
        {
            "parents": len(parents),
            "children": len(children),
            "decoys": len(decoys),
            "empty_things": 0,
            "messy": len(messy),
            "computed": len(computed),
            "bundles": len(bundles),
            "pieces": len(pieces),
        },
        "edge_cases",
    )


def write_example_query() -> None:
    dest = ROOT / "examples"
    write_json(
        dest / "revenue_by_region.json",
        {
            "measures": ["revenue"],
            "dimensions": ["region"],
            "filters": [
                {"field": "order_date", "operator": "year_equals", "value": 2025}
            ],
            "limit": 10,
        },
    )


def main() -> None:
    generate_ecommerce_dirty()
    generate_ecommerce_clean()
    generate_crm()
    generate_edge_cases()
    write_example_query()
    from domain_fixtures import generate_all as generate_domain_fixtures

    generate_domain_fixtures()
    print("fixtures written under", ROOT)


if __name__ == "__main__":
    main()
