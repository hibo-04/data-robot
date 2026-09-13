#!/usr/bin/env python3
"""Five extra domain fixtures (clean + dirty). Called from generate_fixtures.py."""

from __future__ import annotations

import random
from datetime import date, datetime, timedelta
from pathlib import Path

from generate_fixtures import (
    CITIES,
    COMPANIES,
    FIRST,
    LAST,
    MAX_ROWS,
    MAX_TABLES,
    SEED,
    assert_budget,
    write_csv,
    write_json,
)

ROOT = Path(__file__).resolve().parents[1] / "fixtures"

INDUSTRIES = ["Manufacturing", "Retail", "Software", "Logistics", "Healthcare", "Energy", "Chemicals"]
CURRENCIES = ["EUR", "EUR", "EUR", "CHF"]
SITES = ["Hamburg", "München", "Köln", "Leipzig", "Stuttgart"]
MACHINE_TYPES = ["CNC", "Press", "Welder", "Packer", "Oven", "Mill"]
FAULT_CODES = ["E01", "E12", "E20", "E44", "E90"]
PLAN_NAMES = ["Starter", "Team", "Business", "Enterprise"]
TICKET_TYPES = ["bug", "billing", "how-to", "outage"]
PROJECT_TYPES = ["fixed", "time_material", "retainer"]


def emit(name: str, tables: dict[str, list[dict]], schema_tables: dict, truth: dict) -> None:
    dest = ROOT / name
    write_json(dest / "schema.json", {"name": name, "tables": schema_tables})
    write_json(dest / "ground_truth.json", truth)
    for table, rows in tables.items():
        fields = list(schema_tables[table]["columns"].keys())
        write_csv(dest / f"{table}.csv", rows, fields)
    counts = {k: len(v) for k, v in tables.items()}
    if len(counts) > MAX_TABLES or sum(counts.values()) > MAX_ROWS:
        raise SystemExit(f"{name} exceeds budget")
    assert_budget(counts, name)


def fk(columns: list[str], ref_table: str, ref_columns: list[str]) -> dict:
    return {"columns": columns, "ref_table": ref_table, "ref_columns": ref_columns}


def col(type_name: str, nullable: bool | None = None) -> dict:
    out = {"type": type_name}
    if nullable is True:
        out["nullable"] = True
    return out


def blank(rng: random.Random, value, p: float):
    return "" if rng.random() < p else value


def ts(day: date, hour: int, minute: int = 0) -> str:
    return datetime(day.year, day.month, day.day, hour, minute, 0).strftime("%Y-%m-%d %H:%M:%S")


def generate_all() -> None:
    generate_erp_clean()
    generate_erp_dirty()
    generate_production_clean()
    generate_production_dirty()
    from domain_more import generate_logistics_clean, generate_logistics_dirty
    from domain_saas import (
        generate_projects_clean,
        generate_projects_dirty,
        generate_saas_clean,
        generate_saas_dirty,
    )

    generate_logistics_clean()
    generate_logistics_dirty()
    generate_saas_clean()
    generate_saas_dirty()
    generate_projects_clean()
    generate_projects_dirty()


def generate_erp_clean() -> None:
    rng = random.Random(SEED + 10)
    sellers = [
        {
            "seller_id": i,
            "name": f"{FIRST[i % len(FIRST)]} {LAST[i % len(LAST)]}",
            "email": f"seller{i}@erp.example",
            "region": SITES[i % len(SITES)],
            "quota": 250000 + i * 15000,
            "is_active": "true" if i != 11 else "false",
        }
        for i in range(1, 13)
    ]
    suppliers = [
        {
            "supplier_id": i,
            "name": f"{COMPANIES[i % len(COMPANIES)]} Supply {i:02d}",
            "country": rng.choice(["DE", "AT", "NL", "PL"]),
            "city": rng.choice(CITIES),
            "lead_time_days": rng.choice([7, 14, 21, 28]),
            "rating": rng.choice([3, 4, 4, 5]),
        }
        for i in range(1, 21)
    ]
    customers = [
        {
            "customer_id": i,
            "name": f"{COMPANIES[i % len(COMPANIES)]} GmbH {i}",
            "industry": INDUSTRIES[i % len(INDUSTRIES)],
            "city": CITIES[i % len(CITIES)],
            "country": "DE",
            "seller_id": 1 + (i % len(sellers)),
            "payment_terms": rng.choice(["NET14", "NET30", "NET45"]),
            "is_active": "true" if i % 17 else "false",
        }
        for i in range(1, 81)
    ]
    products = []
    for i in range(1, 51):
        cost = round(12 + i * 3.4, 2)
        products.append(
            {
                "product_id": i,
                "sku": f"ERP-{i:04d}",
                "name": f"Industrial Part {i}",
                "category": rng.choice(["Fasteners", "Drives", "Seals", "Tools"]),
                "supplier_id": 1 + (i % len(suppliers)),
                "unit_cost": cost,
                "list_price": round(cost * 1.45, 2),
                "in_stock": rng.randint(4, 400),
            }
        )
    orders = []
    items = []
    invoices = []
    returns = []
    item_id = 1
    invoice_id = 1
    return_id = 1
    for oid in range(1, 251):
        cust = rng.choice(customers)
        order_date = date(2025, 1, 2) + timedelta(days=rng.randint(0, 400))
        net = 0.0
        n_lines = rng.randint(1, 3)
        for _ in range(n_lines):
            prod = rng.choice(products)
            qty = rng.randint(2, 40)
            price = float(prod["list_price"])
            line = round(qty * price, 2)
            net += line
            items.append(
                {
                    "item_id": item_id,
                    "order_id": oid,
                    "product_id": prod["product_id"],
                    "quantity": qty,
                    "unit_price": price,
                    "line_amount": line,
                }
            )
            if rng.random() < 0.08:
                rq = rng.randint(1, qty)
                refund = round(rq * price, 2)
                returns.append(
                    {
                        "return_id": return_id,
                        "item_id": item_id,
                        "return_date": order_date + timedelta(days=rng.randint(5, 40)),
                        "quantity": rq,
                        "reason": rng.choice(["damaged", "wrong_item", "overstock"]),
                        "refund_amount": refund,
                    }
                )
                return_id += 1
            item_id += 1
        net = round(net, 2)
        orders.append(
            {
                "order_id": oid,
                "customer_id": cust["customer_id"],
                "seller_id": cust["seller_id"],
                "order_date": order_date,
                "status": rng.choice(["open", "confirmed", "shipped", "invoiced"]),
                "currency": "EUR",
                "po_number": f"PO-{oid:05d}",
                "net_amount": net,
            }
        )
        tax = round(net * 0.19, 2)
        gross = round(net + tax, 2)
        paid = gross if rng.random() < 0.7 else round(gross * rng.choice([0.0, 0.5]), 2)
        invoices.append(
            {
                "invoice_id": invoice_id,
                "order_id": oid,
                "invoice_date": order_date + timedelta(days=2),
                "due_date": order_date + timedelta(days=32),
                "net_amount": net,
                "tax_amount": tax,
                "gross_amount": gross,
                "paid_amount": paid,
                "remaining_amount": round(gross - paid, 2),
                "status": "paid" if paid == gross else "open",
            }
        )
        invoice_id += 1
    schema = {
        "sellers": {
            "columns": {
                "seller_id": col("integer"),
                "name": col("text"),
                "email": col("text"),
                "region": col("text"),
                "quota": col("decimal"),
                "is_active": col("boolean"),
            },
            "primary_key": ["seller_id"],
        },
        "suppliers": {
            "columns": {
                "supplier_id": col("integer"),
                "name": col("text"),
                "country": col("text"),
                "city": col("text"),
                "lead_time_days": col("integer"),
                "rating": col("integer"),
            },
            "primary_key": ["supplier_id"],
        },
        "customers": {
            "columns": {
                "customer_id": col("integer"),
                "name": col("text"),
                "industry": col("text"),
                "city": col("text"),
                "country": col("text"),
                "seller_id": col("integer"),
                "payment_terms": col("text"),
                "is_active": col("boolean"),
            },
            "primary_key": ["customer_id"],
            "foreign_keys": [fk(["seller_id"], "sellers", ["seller_id"])],
        },
        "products": {
            "columns": {
                "product_id": col("integer"),
                "sku": col("text"),
                "name": col("text"),
                "category": col("text"),
                "supplier_id": col("integer"),
                "unit_cost": col("decimal"),
                "list_price": col("decimal"),
                "in_stock": col("integer"),
            },
            "primary_key": ["product_id"],
            "foreign_keys": [fk(["supplier_id"], "suppliers", ["supplier_id"])],
        },
        "orders": {
            "columns": {
                "order_id": col("integer"),
                "customer_id": col("integer"),
                "seller_id": col("integer"),
                "order_date": col("date"),
                "status": col("text"),
                "currency": col("text"),
                "po_number": col("text"),
                "net_amount": col("decimal"),
            },
            "primary_key": ["order_id"],
            "foreign_keys": [
                fk(["customer_id"], "customers", ["customer_id"]),
                fk(["seller_id"], "sellers", ["seller_id"]),
            ],
        },
        "order_items": {
            "columns": {
                "item_id": col("integer"),
                "order_id": col("integer"),
                "product_id": col("integer"),
                "quantity": col("integer"),
                "unit_price": col("decimal"),
                "line_amount": col("decimal"),
            },
            "primary_key": ["item_id"],
            "foreign_keys": [
                fk(["order_id"], "orders", ["order_id"]),
                fk(["product_id"], "products", ["product_id"]),
            ],
        },
        "invoices": {
            "columns": {
                "invoice_id": col("integer"),
                "order_id": col("integer"),
                "invoice_date": col("date"),
                "due_date": col("date"),
                "net_amount": col("decimal"),
                "tax_amount": col("decimal"),
                "gross_amount": col("decimal"),
                "paid_amount": col("decimal"),
                "remaining_amount": col("decimal"),
                "status": col("text"),
            },
            "primary_key": ["invoice_id"],
            "foreign_keys": [fk(["order_id"], "orders", ["order_id"])],
        },
        "returns": {
            "columns": {
                "return_id": col("integer"),
                "item_id": col("integer"),
                "return_date": col("date"),
                "quantity": col("integer"),
                "reason": col("text"),
                "refund_amount": col("decimal"),
            },
            "primary_key": ["return_id"],
            "foreign_keys": [fk(["item_id"], "order_items", ["item_id"])],
        },
    }
    truth = {
        "relationships": [
            {"from": "customers.seller_id", "to": "sellers.seller_id"},
            {"from": "products.supplier_id", "to": "suppliers.supplier_id"},
            {"from": "orders.customer_id", "to": "customers.customer_id"},
            {"from": "orders.seller_id", "to": "sellers.seller_id"},
            {"from": "order_items.order_id", "to": "orders.order_id"},
            {"from": "order_items.product_id", "to": "products.product_id"},
            {"from": "invoices.order_id", "to": "orders.order_id"},
            {"from": "returns.item_id", "to": "order_items.item_id"},
        ],
        "kpis": [
            {"name": "Revenue", "definition": "SUM(orders.net_amount)"},
            {"name": "Order Count", "definition": "COUNT(DISTINCT orders.order_id)"},
            {"name": "Units Sold", "definition": "SUM(order_items.quantity)"},
            {"name": "Refunds", "definition": "SUM(returns.refund_amount)"},
        ],
    }
    emit(
        "erp_clean",
        {
            "sellers": sellers,
            "suppliers": suppliers,
            "customers": customers,
            "products": products,
            "orders": orders,
            "order_items": items,
            "invoices": invoices,
            "returns": returns,
        },
        schema,
        truth,
    )


def generate_erp_dirty() -> None:
    rng = random.Random(SEED + 11)
    sellers = [
        {
            "vk_id": i,
            "verk_name": f"{FIRST[i % len(FIRST)]} {LAST[i % len(LAST)]}",
            "mail": f"vk{i}@handel.local",
            "gebiet": SITES[i % len(SITES)],
            "soll_umsatz": 180000 + i * 12000,
            "aktiv_kz": "true" if i != 16 else "false",
        }
        for i in range(1, 19)
    ]
    suppliers = [
        {
            "lief_nr": i,
            "lief_bez": f"Lief {COMPANIES[i % len(COMPANIES)]} {i}",
            "land": rng.choice(["DE", "AT", "CN", "PL"]),
            "ort": rng.choice(CITIES + ["Ningbo", "Katowice"]),
            "lieferzeit_t": rng.choice([10, 14, 21, 35]),
            "bewertung": rng.choice([2, 3, 4, 4, 5]),
        }
        for i in range(1, 31)
    ]
    customers = [
        {
            "kd_nr": i,
            "firmenname": f"{COMPANIES[i % len(COMPANIES)]} AG {i}",
            "branche": INDUSTRIES[i % len(INDUSTRIES)],
            "ort": CITIES[i % len(CITIES)],
            "vk_id": blank(rng, 1 + (i % len(sellers)), 0.04),
            "zahlziel": rng.choice(["14T", "30T", "45T", "60T"]),
            "kreditlimit": 20000 + i * 500,
            "angelegt": date(2023, 3, 1) + timedelta(days=i * 3),
        }
        for i in range(1, 151)
    ]
    products = []
    for i in range(1, 91):
        ek = round(8 + i * 2.15, 2)
        products.append(
            {
                "art_nr": f"A{i:05d}",
                "bezeichng": f"Bauteil {i}",
                "warengrp": rng.choice(["Schrauben", "Motoren", "Dichtungen", "Werkzeug"]),
                "lief_nr": 1 + (i % len(suppliers)),
                "ek_preis": ek,
                "vk_preis": round(ek * rng.uniform(1.3, 1.8), 2),
                "lager_stk": rng.randint(0, 600),
                "mwst_satz": rng.choice([0.19, 0.19, 0.19, 0.07]),
            }
        )
    orders = []
    items = []
    invoices = []
    returns = []
    pos_id = 1
    rg_nr = 1
    ret_id = 1
    for oid in range(1, 701):
        cust = rng.choice(customers)
        d = date(2024, 11, 1) + timedelta(days=rng.randint(0, 500))
        net = 0.0
        for _ in range(rng.randint(1, 4)):
            prod = rng.choice(products)
            qty = rng.randint(1, 50)
            price = float(prod["vk_preis"])
            rabatt = round(price * qty * rng.choice([0.0, 0.0, 0.0, 0.05, 0.1]), 2)
            pos_netto = round(qty * price - rabatt, 2)
            net += pos_netto
            gewicht = round(float(prod["ek_preis"]) * 0.02 * qty, 3)
            items.append(
                {
                    "pos_id": pos_id,
                    "auftr_nr": oid,
                    "art_nr": prod["art_nr"],
                    "menge": qty,
                    "einzelpr": price,
                    "pos_rabatt": rabatt,
                    "pos_netto": pos_netto,
                    "gewicht_kg": gewicht,
                }
            )
            if rng.random() < 0.07:
                rq = rng.randint(1, qty)
                erst = round(rq * price * 0.9, 2)
                fee = round(12.5 if rng.random() < 0.4 else 0.0, 2)
                returns.append(
                    {
                        "ret_id": ret_id,
                        "pos_id": pos_id,
                        "ret_dat": d + timedelta(days=rng.randint(4, 50)),
                        "ret_menge": rq,
                        "grund": rng.choice(["Reklamation", "Falschlieferung", "Überbestand"]),
                        "erstattung": erst,
                        "bearb_geb": fee,
                        "netto_erst": round(erst - fee, 2),
                    }
                )
                ret_id += 1
            pos_id += 1
        net = round(net, 2)
        orders.append(
            {
                "auftr_nr": oid,
                "kd_nr": blank(rng, cust["kd_nr"], 0.03),
                "vk_id": cust["vk_id"],
                "auftr_dat": d,
                "status_txt": rng.choice(["offen", "bestätigt", "geliefert", "fakturiert", "storno"]),
                "waehrung": rng.choice(CURRENCIES),
                "bst_nr": blank(rng, f"BST-{oid}", 0.12),
                "netto_betrag": net,
                "fracht": round(rng.choice([0, 29, 49, 89]), 2),
                "notiz": blank(rng, "Rush", 0.85),
            }
        )
        if rng.random() < 0.88:
            tax = round(net * 0.19, 2)
            brutto = round(net + tax, 2)
            gezahlt = brutto if rng.random() < 0.62 else round(brutto * rng.choice([0.0, 0.4, 0.5]), 2)
            invoices.append(
                {
                    "rg_nr": rg_nr,
                    "auftr_nr": oid,
                    "rg_dat": d + timedelta(days=3),
                    "faellig": d + timedelta(days=33),
                    "netto_betrag": net,
                    "mwst_betrag": tax,
                    "brutto": brutto,
                    "gezahlt": gezahlt,
                    "rest": round(brutto - gezahlt, 2),
                    "status_txt": "bezahlt" if gezahlt == brutto else "offen",
                    "mahnstufe": 0 if gezahlt == brutto else rng.choice([0, 1, 2]),
                }
            )
            rg_nr += 1
    schema = {
        "sellers": {
            "columns": {
                "vk_id": col("integer"),
                "verk_name": col("text"),
                "mail": col("text"),
                "gebiet": col("text"),
                "soll_umsatz": col("decimal"),
                "aktiv_kz": col("boolean"),
            },
            "primary_key": ["vk_id"],
        },
        "suppliers": {
            "columns": {
                "lief_nr": col("integer"),
                "lief_bez": col("text"),
                "land": col("text"),
                "ort": col("text"),
                "lieferzeit_t": col("integer"),
                "bewertung": col("integer"),
            },
            "primary_key": ["lief_nr"],
        },
        "customers": {
            "columns": {
                "kd_nr": col("integer"),
                "firmenname": col("text"),
                "branche": col("text"),
                "ort": col("text"),
                "vk_id": col("integer", True),
                "zahlziel": col("text"),
                "kreditlimit": col("decimal"),
                "angelegt": col("date"),
            },
            "primary_key": ["kd_nr"],
            "foreign_keys": [fk(["vk_id"], "sellers", ["vk_id"])],
        },
        "products": {
            "columns": {
                "art_nr": col("text"),
                "bezeichng": col("text"),
                "warengrp": col("text"),
                "lief_nr": col("integer"),
                "ek_preis": col("decimal"),
                "vk_preis": col("decimal"),
                "lager_stk": col("integer"),
                "mwst_satz": col("decimal"),
            },
            "primary_key": ["art_nr"],
            "foreign_keys": [fk(["lief_nr"], "suppliers", ["lief_nr"])],
        },
        "orders": {
            "columns": {
                "auftr_nr": col("integer"),
                "kd_nr": col("integer", True),
                "vk_id": col("integer", True),
                "auftr_dat": col("date"),
                "status_txt": col("text"),
                "waehrung": col("text"),
                "bst_nr": col("text", True),
                "netto_betrag": col("decimal"),
                "fracht": col("decimal"),
                "notiz": col("text", True),
            },
            "primary_key": ["auftr_nr"],
            "foreign_keys": [],
        },
        "order_items": {
            "columns": {
                "pos_id": col("integer"),
                "auftr_nr": col("integer"),
                "art_nr": col("text"),
                "menge": col("integer"),
                "einzelpr": col("decimal"),
                "pos_rabatt": col("decimal"),
                "pos_netto": col("decimal"),
                "gewicht_kg": col("decimal"),
            },
            "primary_key": ["pos_id"],
            "foreign_keys": [fk(["auftr_nr"], "orders", ["auftr_nr"])],
        },
        "invoices": {
            "columns": {
                "rg_nr": col("integer"),
                "auftr_nr": col("integer"),
                "rg_dat": col("date"),
                "faellig": col("date"),
                "netto_betrag": col("decimal"),
                "mwst_betrag": col("decimal"),
                "brutto": col("decimal"),
                "gezahlt": col("decimal"),
                "rest": col("decimal"),
                "status_txt": col("text"),
                "mahnstufe": col("integer"),
            },
            "primary_key": ["rg_nr"],
            "foreign_keys": [fk(["auftr_nr"], "orders", ["auftr_nr"])],
        },
        "returns": {
            "columns": {
                "ret_id": col("integer"),
                "pos_id": col("integer"),
                "ret_dat": col("date"),
                "ret_menge": col("integer"),
                "grund": col("text"),
                "erstattung": col("decimal"),
                "bearb_geb": col("decimal"),
                "netto_erst": col("decimal"),
            },
            "primary_key": ["ret_id"],
            "foreign_keys": [],
        },
    }
    truth = {
        "relationships": [
            {"from": "customers.vk_id", "to": "sellers.vk_id"},
            {"from": "products.lief_nr", "to": "suppliers.lief_nr"},
            {"from": "orders.kd_nr", "to": "customers.kd_nr"},
            {"from": "orders.vk_id", "to": "sellers.vk_id"},
            {"from": "order_items.auftr_nr", "to": "orders.auftr_nr"},
            {"from": "order_items.art_nr", "to": "products.art_nr"},
            {"from": "invoices.auftr_nr", "to": "orders.auftr_nr"},
            {"from": "returns.pos_id", "to": "order_items.pos_id"},
        ],
        "kpis": [
            {"name": "Revenue", "definition": "SUM(orders.netto_betrag)"},
            {"name": "Order Count", "definition": "COUNT(DISTINCT orders.auftr_nr)"},
            {"name": "Units Sold", "definition": "SUM(order_items.menge)"},
            {"name": "Refunds", "definition": "SUM(returns.netto_erst)"},
        ],
    }
    emit(
        "erp_dirty",
        {
            "sellers": sellers,
            "suppliers": suppliers,
            "customers": customers,
            "products": products,
            "orders": orders,
            "order_items": items,
            "invoices": invoices,
            "returns": returns,
        },
        schema,
        truth,
    )


def generate_production_clean() -> None:
    rng = random.Random(SEED + 20)
    machines = [
        {
            "machine_id": i,
            "name": f"{MACHINE_TYPES[i % len(MACHINE_TYPES)]}-{i:02d}",
            "type": MACHINE_TYPES[i % len(MACHINE_TYPES)],
            "site": SITES[i % len(SITES)],
            "install_date": date(2018, 1, 1) + timedelta(days=i * 40),
            "rated_power_kw": round(8 + i * 1.7, 1),
        }
        for i in range(1, 13)
    ]
    shifts = []
    for i in range(1, 41):
        day = date(2025, 3, 1) + timedelta(days=(i - 1) // 3)
        name = ["early", "late", "night"][(i - 1) % 3]
        shifts.append(
            {
                "shift_id": i,
                "shift_date": day,
                "name": name,
                "crew": rng.choice(["A", "B", "C"]),
                "planned_minutes": 480,
            }
        )
    orders = []
    workpieces = []
    wp_id = 1
    for oid in range(1, 201):
        machine = rng.choice(machines)
        planned = rng.randint(4, 12)
        completed = planned if rng.random() < 0.8 else rng.randint(1, planned)
        scrap_n = planned - completed
        start = date(2025, 3, 1) + timedelta(days=rng.randint(0, 70))
        orders.append(
            {
                "order_id": oid,
                "machine_id": machine["machine_id"],
                "shift_id": 1 + ((oid - 1) % len(shifts)),
                "part_no": f"P-{1000 + oid}",
                "planned_qty": planned,
                "completed_qty": completed,
                "scrap_qty": scrap_n,
                "started_on": start,
                "status": "done" if completed == planned else "running",
            }
        )
        for n in range(completed):
            net = round(rng.uniform(120, 480), 1)
            scrap = round(rng.uniform(0.2, 8.0), 1)
            workpieces.append(
                {
                    "workpiece_id": wp_id,
                    "order_id": oid,
                    "machine_id": machine["machine_id"],
                    "serial": f"WP-{oid:04d}-{n + 1:02d}",
                    "net_weight_g": net,
                    "scrap_g": scrap,
                    "gross_weight_g": round(net + scrap, 1),
                    "cycle_seconds": rng.randint(40, 180),
                    "ok": "true",
                }
            )
            wp_id += 1
    sensors = []
    sid = 1
    base = datetime(2025, 3, 1, 6, 0, 0)
    for i in range(3500):
        machine = machines[i % len(machines)]
        t = base + timedelta(minutes=15 * (i // len(machines)))
        power = round(float(machine["rated_power_kw"]) * rng.uniform(0.35, 0.95), 2)
        sensors.append(
            {
                "reading_id": sid,
                "machine_id": machine["machine_id"],
                "recorded_at": t.strftime("%Y-%m-%d %H:%M:%S"),
                "temperature_c": round(rng.uniform(38, 92), 1),
                "vibration_rms": round(rng.uniform(0.4, 6.5), 2),
                "power_kw": power,
                "energy_kwh": round(power * 0.25, 3),
            }
        )
        sid += 1
    faults = []
    for i in range(1, 251):
        machine = rng.choice(machines)
        dur = rng.randint(5, 240)
        faults.append(
            {
                "fault_id": i,
                "machine_id": machine["machine_id"],
                "opened_at": (base + timedelta(hours=i * 3)).strftime("%Y-%m-%d %H:%M:%S"),
                "code": rng.choice(FAULT_CODES),
                "severity": rng.choice(["low", "medium", "high"]),
                "duration_min": dur,
                "resolved": "true" if rng.random() < 0.9 else "false",
            }
        )
    maintenances = [
        {
            "maintenance_id": i,
            "machine_id": 1 + ((i - 1) % len(machines)),
            "performed_on": date(2025, 1, 5) + timedelta(days=i * 4),
            "kind": rng.choice(["inspection", "repair", "lubrication", "calibration"]),
            "duration_min": rng.randint(30, 360),
            "cost": round(rng.uniform(80, 2400), 2),
        }
        for i in range(1, 151)
    ]
    schema = {
        "machines": {
            "columns": {
                "machine_id": col("integer"),
                "name": col("text"),
                "type": col("text"),
                "site": col("text"),
                "install_date": col("date"),
                "rated_power_kw": col("decimal"),
            },
            "primary_key": ["machine_id"],
        },
        "shifts": {
            "columns": {
                "shift_id": col("integer"),
                "shift_date": col("date"),
                "name": col("text"),
                "crew": col("text"),
                "planned_minutes": col("integer"),
            },
            "primary_key": ["shift_id"],
        },
        "production_orders": {
            "columns": {
                "order_id": col("integer"),
                "machine_id": col("integer"),
                "shift_id": col("integer"),
                "part_no": col("text"),
                "planned_qty": col("integer"),
                "completed_qty": col("integer"),
                "scrap_qty": col("integer"),
                "started_on": col("date"),
                "status": col("text"),
            },
            "primary_key": ["order_id"],
            "foreign_keys": [
                fk(["machine_id"], "machines", ["machine_id"]),
                fk(["shift_id"], "shifts", ["shift_id"]),
            ],
        },
        "workpieces": {
            "columns": {
                "workpiece_id": col("integer"),
                "order_id": col("integer"),
                "machine_id": col("integer"),
                "serial": col("text"),
                "net_weight_g": col("decimal"),
                "scrap_g": col("decimal"),
                "gross_weight_g": col("decimal"),
                "cycle_seconds": col("integer"),
                "ok": col("boolean"),
            },
            "primary_key": ["workpiece_id"],
            "foreign_keys": [
                fk(["order_id"], "production_orders", ["order_id"]),
                fk(["machine_id"], "machines", ["machine_id"]),
            ],
        },
        "sensor_readings": {
            "columns": {
                "reading_id": col("integer"),
                "machine_id": col("integer"),
                "recorded_at": col("timestamp"),
                "temperature_c": col("decimal"),
                "vibration_rms": col("decimal"),
                "power_kw": col("decimal"),
                "energy_kwh": col("decimal"),
            },
            "primary_key": ["reading_id"],
            "foreign_keys": [fk(["machine_id"], "machines", ["machine_id"])],
        },
        "faults": {
            "columns": {
                "fault_id": col("integer"),
                "machine_id": col("integer"),
                "opened_at": col("timestamp"),
                "code": col("text"),
                "severity": col("text"),
                "duration_min": col("integer"),
                "resolved": col("boolean"),
            },
            "primary_key": ["fault_id"],
            "foreign_keys": [fk(["machine_id"], "machines", ["machine_id"])],
        },
        "maintenances": {
            "columns": {
                "maintenance_id": col("integer"),
                "machine_id": col("integer"),
                "performed_on": col("date"),
                "kind": col("text"),
                "duration_min": col("integer"),
                "cost": col("decimal"),
            },
            "primary_key": ["maintenance_id"],
            "foreign_keys": [fk(["machine_id"], "machines", ["machine_id"])],
        },
    }
    truth = {
        "relationships": [
            {"from": "production_orders.machine_id", "to": "machines.machine_id"},
            {"from": "production_orders.shift_id", "to": "shifts.shift_id"},
            {"from": "workpieces.order_id", "to": "production_orders.order_id"},
            {"from": "workpieces.machine_id", "to": "machines.machine_id"},
            {"from": "sensor_readings.machine_id", "to": "machines.machine_id"},
            {"from": "faults.machine_id", "to": "machines.machine_id"},
            {"from": "maintenances.machine_id", "to": "machines.machine_id"},
        ],
        "kpis": [
            {"name": "Produced Units", "definition": "SUM(production_orders.completed_qty)"},
            {"name": "Downtime", "definition": "SUM(faults.duration_min)"},
            {"name": "Energy", "definition": "SUM(sensor_readings.energy_kwh)"},
            {"name": "Maintenance Cost", "definition": "SUM(maintenances.cost)"},
        ],
    }
    emit(
        "production_clean",
        {
            "machines": machines,
            "shifts": shifts,
            "production_orders": orders,
            "workpieces": workpieces,
            "sensor_readings": sensors,
            "faults": faults,
            "maintenances": maintenances,
        },
        schema,
        truth,
    )


def generate_production_dirty() -> None:
    rng = random.Random(SEED + 21)
    machines = [
        {
            "masch_id": i,
            "bez": f"M{i:02d}-{MACHINE_TYPES[i % len(MACHINE_TYPES)][:3]}",
            "typ_txt": MACHINE_TYPES[i % len(MACHINE_TYPES)],
            "werk": SITES[i % len(SITES)],
            "inbetrieb": date(2017, 6, 1) + timedelta(days=i * 55),
            "nennleist_kw": round(10 + i * 1.3, 1),
        }
        for i in range(1, 17)
    ]
    shifts = [
        {
            "schicht_id": i,
            "tag": date(2025, 2, 1) + timedelta(days=(i - 1) // 3),
            "schicht_bez": ["F", "S", "N"][(i - 1) % 3],
            "kolonne": rng.choice(["K1", "K2", "K3"]),
            "plan_min": 480,
        }
        for i in range(1, 49)
    ]
    orders = []
    workpieces = []
    wp_id = 1
    for oid in range(1, 281):
        machine = rng.choice(machines)
        planned = rng.randint(3, 10)
        completed = planned if rng.random() < 0.75 else rng.randint(0, planned)
        yield_pct = 0 if planned == 0 else round(100.0 * completed / planned, 1)
        orders.append(
            {
                "pa_nr": oid,
                "masch_id": machine["masch_id"],
                "schicht_id": blank(rng, 1 + ((oid - 1) % len(shifts)), 0.05),
                "teil_nr": f"T{oid:04d}",
                "plan_stk": planned,
                "ist_stk": completed,
                "ausbeute_pct": yield_pct,
                "start_dat": date(2025, 2, 1) + timedelta(days=rng.randint(0, 90)),
                "stat": rng.choice(["läuft", "fertig", "abbruch"]),
            }
        )
        for n in range(max(completed, 0)):
            net = round(rng.uniform(90, 520), 1)
            scrap = round(rng.uniform(0.0, 12.0), 1)
            workpieces.append(
                {
                    "wt_id": wp_id,
                    "pa_nr": oid,
                    "masch_id": machine["masch_id"],
                    "snr": f"{oid}-{n + 1}",
                    "nettogew_g": net,
                    "ausschuss_g": scrap,
                    "bruttogew_g": round(net + scrap, 1),
                    "zyklus_s": rng.randint(35, 200),
                    "iO": "true" if rng.random() < 0.93 else "false",
                }
            )
            wp_id += 1
    sensors = []
    sid = 1
    base = datetime(2025, 2, 1, 5, 0, 0)
    for i in range(5200):
        machine = machines[i % len(machines)]
        t = base + timedelta(minutes=12 * (i // len(machines)))
        power = round(float(machine["nennleist_kw"]) * rng.uniform(0.3, 1.05), 2)
        sensors.append(
            {
                "mess_id": sid,
                "masch_id": machine["masch_id"],
                "ts": t.strftime("%Y-%m-%d %H:%M:%S"),
                "temp_c": round(rng.uniform(34, 98), 1),
                "vib_rms": round(rng.uniform(0.2, 8.1), 2),
                "leist_kw": power,
                "energie_kwh": round(power * 0.2, 3),
            }
        )
        sid += 1
    faults = [
        {
            "stoer_id": i,
            "masch_id": 1 + rng.randrange(len(machines)),
            "aufgetreten": (base + timedelta(hours=i * 2)).strftime("%Y-%m-%d %H:%M:%S"),
            "code": rng.choice(FAULT_CODES + ["??"]),
            "prio": rng.choice(["n", "h", "k"]),
            "dauer_min": rng.randint(3, 300),
            "behoben_kz": "true" if rng.random() < 0.86 else "false",
        }
        for i in range(1, 401)
    ]
    maintenances = [
        {
            "wart_id": i,
            "masch_id": 1 + ((i - 1) % len(machines)),
            "am": date(2024, 12, 1) + timedelta(days=i * 3),
            "art": rng.choice(["Inspektion", "Reparatur", "Schmierung"]),
            "dauer_min": rng.randint(20, 400),
            "kosten": round(rng.uniform(50, 3100), 2),
        }
        for i in range(1, 201)
    ]
    schema = {
        "machines": {
            "columns": {
                "masch_id": col("integer"),
                "bez": col("text"),
                "typ_txt": col("text"),
                "werk": col("text"),
                "inbetrieb": col("date"),
                "nennleist_kw": col("decimal"),
            },
            "primary_key": ["masch_id"],
        },
        "shifts": {
            "columns": {
                "schicht_id": col("integer"),
                "tag": col("date"),
                "schicht_bez": col("text"),
                "kolonne": col("text"),
                "plan_min": col("integer"),
            },
            "primary_key": ["schicht_id"],
        },
        "production_orders": {
            "columns": {
                "pa_nr": col("integer"),
                "masch_id": col("integer"),
                "schicht_id": col("integer", True),
                "teil_nr": col("text"),
                "plan_stk": col("integer"),
                "ist_stk": col("integer"),
                "ausbeute_pct": col("decimal"),
                "start_dat": col("date"),
                "stat": col("text"),
            },
            "primary_key": ["pa_nr"],
            "foreign_keys": [fk(["masch_id"], "machines", ["masch_id"])],
        },
        "workpieces": {
            "columns": {
                "wt_id": col("integer"),
                "pa_nr": col("integer"),
                "masch_id": col("integer"),
                "snr": col("text"),
                "nettogew_g": col("decimal"),
                "ausschuss_g": col("decimal"),
                "bruttogew_g": col("decimal"),
                "zyklus_s": col("integer"),
                "iO": col("boolean"),
            },
            "primary_key": ["wt_id"],
            "foreign_keys": [fk(["pa_nr"], "production_orders", ["pa_nr"])],
        },
        "sensor_readings": {
            "columns": {
                "mess_id": col("integer"),
                "masch_id": col("integer"),
                "ts": col("timestamp"),
                "temp_c": col("decimal"),
                "vib_rms": col("decimal"),
                "leist_kw": col("decimal"),
                "energie_kwh": col("decimal"),
            },
            "primary_key": ["mess_id"],
            "foreign_keys": [],
        },
        "faults": {
            "columns": {
                "stoer_id": col("integer"),
                "masch_id": col("integer"),
                "aufgetreten": col("timestamp"),
                "code": col("text"),
                "prio": col("text"),
                "dauer_min": col("integer"),
                "behoben_kz": col("boolean"),
            },
            "primary_key": ["stoer_id"],
            "foreign_keys": [],
        },
        "maintenances": {
            "columns": {
                "wart_id": col("integer"),
                "masch_id": col("integer"),
                "am": col("date"),
                "art": col("text"),
                "dauer_min": col("integer"),
                "kosten": col("decimal"),
            },
            "primary_key": ["wart_id"],
            "foreign_keys": [fk(["masch_id"], "machines", ["masch_id"])],
        },
    }
    truth = {
        "relationships": [
            {"from": "production_orders.masch_id", "to": "machines.masch_id"},
            {"from": "production_orders.schicht_id", "to": "shifts.schicht_id"},
            {"from": "workpieces.pa_nr", "to": "production_orders.pa_nr"},
            {"from": "workpieces.masch_id", "to": "machines.masch_id"},
            {"from": "sensor_readings.masch_id", "to": "machines.masch_id"},
            {"from": "faults.masch_id", "to": "machines.masch_id"},
            {"from": "maintenances.masch_id", "to": "machines.masch_id"},
        ],
        "kpis": [
            {"name": "Produced Units", "definition": "SUM(production_orders.ist_stk)"},
            {"name": "Downtime", "definition": "SUM(faults.dauer_min)"},
            {"name": "Energy", "definition": "SUM(sensor_readings.energie_kwh)"},
            {"name": "Maintenance Cost", "definition": "SUM(maintenances.kosten)"},
        ],
    }
    emit(
        "production_dirty",
        {
            "machines": machines,
            "shifts": shifts,
            "production_orders": orders,
            "workpieces": workpieces,
            "sensor_readings": sensors,
            "faults": faults,
            "maintenances": maintenances,
        },
        schema,
        truth,
    )

