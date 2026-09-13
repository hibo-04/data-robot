"""SaaS and professional-services fixtures. Imported by domain_fixtures."""

from __future__ import annotations

import random
from datetime import date, timedelta

from domain_fixtures import (
    CITIES,
    COMPANIES,
    FIRST,
    LAST,
    PLAN_NAMES,
    PROJECT_TYPES,
    SEED,
    SITES,
    TICKET_TYPES,
    blank,
    col,
    emit,
    fk,
)


def generate_saas_clean() -> None:
    rng = random.Random(SEED + 40)
    plans = []
    for i, name in enumerate(PLAN_NAMES, start=1):
        price = [29, 79, 199, 499][i - 1]
        plans.append(
            {
                "plan_id": i,
                "name": name,
                "monthly_price": price,
                "included_seats": [3, 10, 25, 100][i - 1],
                "is_active": "true",
            }
        )
    companies = [
        {
            "company_id": i,
            "name": f"{COMPANIES[i % len(COMPANIES)]} Cloud {i}",
            "industry": rng.choice(["Software", "Retail", "Healthcare", "Finance"]),
            "country": rng.choice(["DE", "AT", "CH", "NL"]),
            "city": CITIES[i % len(CITIES)],
            "created_at": date(2023, 1, 1) + timedelta(days=i * 4),
        }
        for i in range(1, 81)
    ]
    users = [
        {
            "user_id": i,
            "company_id": 1 + ((i - 1) % len(companies)),
            "email": f"user{i}@saas.example",
            "role": rng.choice(["admin", "member", "billing"]),
            "is_active": "true" if i % 13 else "false",
            "created_at": date(2023, 2, 1) + timedelta(days=i),
        }
        for i in range(1, 401)
    ]
    subscriptions = []
    invoices = []
    payments = []
    usage = []
    tickets = []
    inv_id = 1
    pay_id = 1
    use_id = 1
    tic_id = 1
    for sid in range(1, 201):
        company = rng.choice(companies)
        plan = rng.choice(plans)
        seats = rng.randint(1, int(plan["included_seats"]))
        mrr = round(seats * float(plan["monthly_price"]), 2)
        start = date(2024, 1, 1) + timedelta(days=rng.randint(0, 400))
        churned = rng.random() < 0.18
        status = "churned" if churned else rng.choice(["trial", "active", "past_due"])
        subscriptions.append(
            {
                "subscription_id": sid,
                "company_id": company["company_id"],
                "plan_id": plan["plan_id"],
                "seats": seats,
                "mrr": mrr,
                "arr": round(mrr * 12, 2),
                "started_on": start,
                "ended_on": start + timedelta(days=rng.randint(40, 300)) if churned else "",
                "status": status,
            }
        )
        n_inv = rng.randint(1, 3)
        for k in range(n_inv):
            subtotal = mrr
            tax = round(subtotal * 0.19, 2)
            total = round(subtotal + tax, 2)
            paid = total if rng.random() < 0.82 else 0.0
            invoices.append(
                {
                    "invoice_id": inv_id,
                    "subscription_id": sid,
                    "company_id": company["company_id"],
                    "invoice_date": start + timedelta(days=30 * k),
                    "subtotal": subtotal,
                    "tax_amount": tax,
                    "total_amount": total,
                    "paid_amount": paid,
                    "remaining_amount": round(total - paid, 2),
                    "status": "paid" if paid == total else "open",
                }
            )
            if paid:
                payments.append(
                    {
                        "payment_id": pay_id,
                        "invoice_id": inv_id,
                        "paid_on": start + timedelta(days=30 * k + 3),
                        "amount": paid,
                        "method": rng.choice(["card", "sepa", "wire"]),
                    }
                )
                pay_id += 1
            inv_id += 1
        for _ in range(rng.randint(2, 6)):
            usage.append(
                {
                    "usage_id": use_id,
                    "company_id": company["company_id"],
                    "user_id": rng.choice(
                        [u["user_id"] for u in users if u["company_id"] == company["company_id"]]
                        or [users[0]["user_id"]]
                    ),
                    "feature": rng.choice(["export", "api", "sso", "audit_log", "dashboard"]),
                    "used_on": start + timedelta(days=rng.randint(0, 80)),
                    "events": rng.randint(1, 400),
                }
            )
            use_id += 1
            if use_id > 800:
                break
        if rng.random() < 0.55:
            tickets.append(
                {
                    "ticket_id": tic_id,
                    "company_id": company["company_id"],
                    "user_id": blank(
                        rng,
                        rng.choice(
                            [u["user_id"] for u in users if u["company_id"] == company["company_id"]]
                            or [1]
                        ),
                        0.1,
                    ),
                    "opened_on": start + timedelta(days=rng.randint(0, 90)),
                    "type": rng.choice(TICKET_TYPES),
                    "priority": rng.choice(["low", "normal", "high"]),
                    "handle_hours": round(rng.uniform(0.5, 48.0), 1),
                    "status": rng.choice(["open", "solved", "waiting"]),
                }
            )
            tic_id += 1
        if use_id > 800:
            break
    schema = {
        "plans": {
            "columns": {
                "plan_id": col("integer"),
                "name": col("text"),
                "monthly_price": col("decimal"),
                "included_seats": col("integer"),
                "is_active": col("boolean"),
            },
            "primary_key": ["plan_id"],
        },
        "companies": {
            "columns": {
                "company_id": col("integer"),
                "name": col("text"),
                "industry": col("text"),
                "country": col("text"),
                "city": col("text"),
                "created_at": col("date"),
            },
            "primary_key": ["company_id"],
        },
        "users": {
            "columns": {
                "user_id": col("integer"),
                "company_id": col("integer"),
                "email": col("text"),
                "role": col("text"),
                "is_active": col("boolean"),
                "created_at": col("date"),
            },
            "primary_key": ["user_id"],
            "foreign_keys": [fk(["company_id"], "companies", ["company_id"])],
        },
        "subscriptions": {
            "columns": {
                "subscription_id": col("integer"),
                "company_id": col("integer"),
                "plan_id": col("integer"),
                "seats": col("integer"),
                "mrr": col("decimal"),
                "arr": col("decimal"),
                "started_on": col("date"),
                "ended_on": col("date", True),
                "status": col("text"),
            },
            "primary_key": ["subscription_id"],
            "foreign_keys": [
                fk(["company_id"], "companies", ["company_id"]),
                fk(["plan_id"], "plans", ["plan_id"]),
            ],
        },
        "invoices": {
            "columns": {
                "invoice_id": col("integer"),
                "subscription_id": col("integer"),
                "company_id": col("integer"),
                "invoice_date": col("date"),
                "subtotal": col("decimal"),
                "tax_amount": col("decimal"),
                "total_amount": col("decimal"),
                "paid_amount": col("decimal"),
                "remaining_amount": col("decimal"),
                "status": col("text"),
            },
            "primary_key": ["invoice_id"],
            "foreign_keys": [
                fk(["subscription_id"], "subscriptions", ["subscription_id"]),
                fk(["company_id"], "companies", ["company_id"]),
            ],
        },
        "payments": {
            "columns": {
                "payment_id": col("integer"),
                "invoice_id": col("integer"),
                "paid_on": col("date"),
                "amount": col("decimal"),
                "method": col("text"),
            },
            "primary_key": ["payment_id"],
            "foreign_keys": [fk(["invoice_id"], "invoices", ["invoice_id"])],
        },
        "feature_usage": {
            "columns": {
                "usage_id": col("integer"),
                "company_id": col("integer"),
                "user_id": col("integer"),
                "feature": col("text"),
                "used_on": col("date"),
                "events": col("integer"),
            },
            "primary_key": ["usage_id"],
            "foreign_keys": [
                fk(["company_id"], "companies", ["company_id"]),
                fk(["user_id"], "users", ["user_id"]),
            ],
        },
        "tickets": {
            "columns": {
                "ticket_id": col("integer"),
                "company_id": col("integer"),
                "user_id": col("integer", True),
                "opened_on": col("date"),
                "type": col("text"),
                "priority": col("text"),
                "handle_hours": col("decimal"),
                "status": col("text"),
            },
            "primary_key": ["ticket_id"],
            "foreign_keys": [fk(["company_id"], "companies", ["company_id"])],
        },
    }
    truth = {
        "relationships": [
            {"from": "users.company_id", "to": "companies.company_id"},
            {"from": "subscriptions.company_id", "to": "companies.company_id"},
            {"from": "subscriptions.plan_id", "to": "plans.plan_id"},
            {"from": "invoices.subscription_id", "to": "subscriptions.subscription_id"},
            {"from": "invoices.company_id", "to": "companies.company_id"},
            {"from": "payments.invoice_id", "to": "invoices.invoice_id"},
            {"from": "feature_usage.company_id", "to": "companies.company_id"},
            {"from": "feature_usage.user_id", "to": "users.user_id"},
            {"from": "tickets.company_id", "to": "companies.company_id"},
            {"from": "tickets.user_id", "to": "users.user_id"},
        ],
        "kpis": [
            {"name": "MRR", "definition": "SUM(subscriptions.mrr)"},
            {"name": "ARR", "definition": "SUM(subscriptions.arr)"},
            {"name": "Invoice Amount", "definition": "SUM(invoices.total_amount)"},
            {"name": "Ticket Count", "definition": "COUNT(DISTINCT tickets.ticket_id)"},
        ],
    }
    emit(
        "saas_clean",
        {
            "plans": plans,
            "companies": companies,
            "users": users,
            "subscriptions": subscriptions,
            "invoices": invoices,
            "payments": payments,
            "feature_usage": usage,
            "tickets": tickets,
        },
        schema,
        truth,
    )


def generate_saas_dirty() -> None:
    rng = random.Random(SEED + 41)
    plans = []
    for i, name in enumerate(["Basic", "Pro", "Scale", "Ent"], start=1):
        plans.append(
            {
                "tarif_id": i,
                "tarif_bez": name,
                "preis_m": [39, 99, 249, 799][i - 1],
                "inkl_seats": [2, 8, 30, 150][i - 1],
                "aktiv_kz": "true" if i != 4 else "true",
            }
        )
    companies = [
        {
            "firma_nr": i,
            "firmenname": f"{COMPANIES[i % len(COMPANIES)]} SaaS {i}",
            "branche": rng.choice(["IT", "Handel", "Medizin", "Fin"]),
            "land": rng.choice(["DE", "AT", "CH"]),
            "ort": CITIES[i % len(CITIES)],
            "anlagedat": date(2022, 6, 1) + timedelta(days=i * 3),
        }
        for i in range(1, 121)
    ]
    users = [
        {
            "usr_id": i,
            "firma_nr": 1 + ((i - 1) % len(companies)),
            "mail": f"u{i}@cloud.local",
            "rolle_txt": rng.choice(["adm", "usr", "bill"]),
            "aktiv_kz": "true" if i % 11 else "false",
            "anlagedat": date(2022, 8, 1) + timedelta(days=i // 2),
        }
        for i in range(1, 701)
    ]
    subscriptions = []
    invoices = []
    payments = []
    usage = []
    tickets = []
    inv_id = 1
    pay_id = 1
    use_id = 1
    tic_id = 1
    for sid in range(1, 351):
        company = rng.choice(companies)
        plan = rng.choice(plans)
        seats = rng.randint(1, int(plan["inkl_seats"]))
        mrr = round(seats * float(plan["preis_m"]), 2)
        start = date(2023, 6, 1) + timedelta(days=rng.randint(0, 500))
        churned = rng.random() < 0.22
        subscriptions.append(
            {
                "abo_nr": sid,
                "firma_nr": company["firma_nr"],
                "tarif_id": plan["tarif_id"],
                "anz_seats": seats,
                "mrr_betrag": mrr,
                "arr_betrag": round(mrr * 12, 2),
                "start_dat": start,
                "ende_dat": start + timedelta(days=rng.randint(20, 280)) if churned else "",
                "stat_txt": "gekündigt" if churned else rng.choice(["trial", "aktiv", "mahnung"]),
            }
        )
        for k in range(rng.randint(1, 4)):
            netto = mrr
            mwst = round(netto * 0.19, 2)
            brutto = round(netto + mwst, 2)
            gezahlt = brutto if rng.random() < 0.78 else rng.choice([0.0, round(brutto * 0.5, 2)])
            invoices.append(
                {
                    "rg_nr": inv_id,
                    "abo_nr": sid,
                    "firma_nr": company["firma_nr"],
                    "rg_dat": start + timedelta(days=30 * k),
                    "netto_betrag": netto,
                    "mwst_betrag": mwst,
                    "brutto": brutto,
                    "gezahlt": gezahlt,
                    "rest": round(brutto - gezahlt, 2),
                    "stat_txt": "bezahlt" if gezahlt == brutto else "offen",
                }
            )
            if gezahlt:
                payments.append(
                    {
                        "zahl_id": pay_id,
                        "rg_nr": inv_id,
                        "zahl_dat": start + timedelta(days=30 * k + 4),
                        "betrag": gezahlt,
                        "weg": rng.choice(["KK", "SEPA", "ÜWZ"]),
                    }
                )
                pay_id += 1
            inv_id += 1
        company_users = [u["usr_id"] for u in users if u["firma_nr"] == company["firma_nr"]] or [1]
        for _ in range(rng.randint(3, 8)):
            if use_id > 2500:
                break
            usage.append(
                {
                    "nutz_id": use_id,
                    "firma_nr": company["firma_nr"],
                    "usr_id": rng.choice(company_users),
                    "feat": rng.choice(["export", "api", "sso", "audit", "dash"]),
                    "am": start + timedelta(days=rng.randint(0, 100)),
                    "anz_events": rng.randint(1, 800),
                }
            )
            use_id += 1
        if rng.random() < 0.6 and tic_id <= 500:
            tickets.append(
                {
                    "tix_nr": tic_id,
                    "firma_nr": company["firma_nr"],
                    "usr_id": blank(rng, rng.choice(company_users), 0.12),
                    "auf_dat": start + timedelta(days=rng.randint(0, 120)),
                    "typ_txt": rng.choice(["bug", "rechnung", "frage", "ausfall"]),
                    "prio": rng.choice(["n", "h", "k"]),
                    "bearb_h": round(rng.uniform(0.2, 60.0), 1),
                    "stat_txt": rng.choice(["offen", "erledigt", "warte"]),
                }
            )
            tic_id += 1
        if use_id > 2500:
            break
    schema = {
        "plans": {
            "columns": {
                "tarif_id": col("integer"),
                "tarif_bez": col("text"),
                "preis_m": col("decimal"),
                "inkl_seats": col("integer"),
                "aktiv_kz": col("boolean"),
            },
            "primary_key": ["tarif_id"],
        },
        "companies": {
            "columns": {
                "firma_nr": col("integer"),
                "firmenname": col("text"),
                "branche": col("text"),
                "land": col("text"),
                "ort": col("text"),
                "anlagedat": col("date"),
            },
            "primary_key": ["firma_nr"],
        },
        "users": {
            "columns": {
                "usr_id": col("integer"),
                "firma_nr": col("integer"),
                "mail": col("text"),
                "rolle_txt": col("text"),
                "aktiv_kz": col("boolean"),
                "anlagedat": col("date"),
            },
            "primary_key": ["usr_id"],
            "foreign_keys": [fk(["firma_nr"], "companies", ["firma_nr"])],
        },
        "subscriptions": {
            "columns": {
                "abo_nr": col("integer"),
                "firma_nr": col("integer"),
                "tarif_id": col("integer"),
                "anz_seats": col("integer"),
                "mrr_betrag": col("decimal"),
                "arr_betrag": col("decimal"),
                "start_dat": col("date"),
                "ende_dat": col("date", True),
                "stat_txt": col("text"),
            },
            "primary_key": ["abo_nr"],
            "foreign_keys": [fk(["firma_nr"], "companies", ["firma_nr"])],
        },
        "invoices": {
            "columns": {
                "rg_nr": col("integer"),
                "abo_nr": col("integer"),
                "firma_nr": col("integer"),
                "rg_dat": col("date"),
                "netto_betrag": col("decimal"),
                "mwst_betrag": col("decimal"),
                "brutto": col("decimal"),
                "gezahlt": col("decimal"),
                "rest": col("decimal"),
                "stat_txt": col("text"),
            },
            "primary_key": ["rg_nr"],
            "foreign_keys": [fk(["abo_nr"], "subscriptions", ["abo_nr"])],
        },
        "payments": {
            "columns": {
                "zahl_id": col("integer"),
                "rg_nr": col("integer"),
                "zahl_dat": col("date"),
                "betrag": col("decimal"),
                "weg": col("text"),
            },
            "primary_key": ["zahl_id"],
            "foreign_keys": [],
        },
        "feature_usage": {
            "columns": {
                "nutz_id": col("integer"),
                "firma_nr": col("integer"),
                "usr_id": col("integer"),
                "feat": col("text"),
                "am": col("date"),
                "anz_events": col("integer"),
            },
            "primary_key": ["nutz_id"],
            "foreign_keys": [],
        },
        "tickets": {
            "columns": {
                "tix_nr": col("integer"),
                "firma_nr": col("integer"),
                "usr_id": col("integer", True),
                "auf_dat": col("date"),
                "typ_txt": col("text"),
                "prio": col("text"),
                "bearb_h": col("decimal"),
                "stat_txt": col("text"),
            },
            "primary_key": ["tix_nr"],
            "foreign_keys": [fk(["firma_nr"], "companies", ["firma_nr"])],
        },
    }
    truth = {
        "relationships": [
            {"from": "users.firma_nr", "to": "companies.firma_nr"},
            {"from": "subscriptions.firma_nr", "to": "companies.firma_nr"},
            {"from": "subscriptions.tarif_id", "to": "plans.tarif_id"},
            {"from": "invoices.abo_nr", "to": "subscriptions.abo_nr"},
            {"from": "invoices.firma_nr", "to": "companies.firma_nr"},
            {"from": "payments.rg_nr", "to": "invoices.rg_nr"},
            {"from": "feature_usage.firma_nr", "to": "companies.firma_nr"},
            {"from": "feature_usage.usr_id", "to": "users.usr_id"},
            {"from": "tickets.firma_nr", "to": "companies.firma_nr"},
            {"from": "tickets.usr_id", "to": "users.usr_id"},
        ],
        "kpis": [
            {"name": "MRR", "definition": "SUM(subscriptions.mrr_betrag)"},
            {"name": "ARR", "definition": "SUM(subscriptions.arr_betrag)"},
            {"name": "Invoice Amount", "definition": "SUM(invoices.brutto)"},
            {"name": "Ticket Count", "definition": "COUNT(DISTINCT tickets.tix_nr)"},
        ],
    }
    emit(
        "saas_dirty",
        {
            "plans": plans,
            "companies": companies,
            "users": users,
            "subscriptions": subscriptions,
            "invoices": invoices,
            "payments": payments,
            "feature_usage": usage,
            "tickets": tickets,
        },
        schema,
        truth,
    )


def generate_projects_clean() -> None:
    rng = random.Random(SEED + 50)
    customers = [
        {
            "customer_id": i,
            "name": f"{COMPANIES[i % len(COMPANIES)]} Consulting {i}",
            "industry": rng.choice(["Public", "Energy", "Banking", "Retail"]),
            "city": CITIES[i % len(CITIES)],
        }
        for i in range(1, 41)
    ]
    teams = [
        {"team_id": i, "name": n, "location": SITES[i % len(SITES)]}
        for i, n in enumerate(["Delivery", "Design", "Data", "PMO", "Support", "Sales", "Ops", "QA"], start=1)
    ]
    employees = [
        {
            "employee_id": i,
            "team_id": 1 + ((i - 1) % len(teams)),
            "name": f"{FIRST[i % len(FIRST)]} {LAST[i % len(LAST)]}",
            "role": rng.choice(["consultant", "manager", "designer", "engineer"]),
            "hourly_rate": round(70 + (i % 9) * 15, 2),
            "hired_on": date(2019, 4, 1) + timedelta(days=i * 12),
        }
        for i in range(1, 61)
    ]
    projects = []
    assignments = []
    tasks = []
    times = []
    costs = []
    invoices = []
    budgets = []
    asg_id = 1
    task_id = 1
    time_id = 1
    cost_id = 1
    inv_id = 1
    for pid in range(1, 81):
        cust = rng.choice(customers)
        budget_amt = round(rng.uniform(25000, 280000), 2)
        logged = 0.0
        spent = 0.0
        billed = 0.0
        projects.append(
            {
                "project_id": pid,
                "customer_id": cust["customer_id"],
                "name": f"Project {pid}",
                "type": rng.choice(PROJECT_TYPES),
                "started_on": date(2024, 9, 1) + timedelta(days=rng.randint(0, 300)),
                "status": rng.choice(["active", "done", "on_hold"]),
                "logged_hours": 0.0,
            }
        )
        assigned = rng.sample(employees, k=rng.randint(2, 5))
        for emp in assigned:
            assignments.append(
                {
                    "assignment_id": asg_id,
                    "project_id": pid,
                    "employee_id": emp["employee_id"],
                    "role": rng.choice(["lead", "member"]),
                    "allocation_pct": rng.choice([25, 50, 75, 100]),
                }
            )
            asg_id += 1
        for _ in range(rng.randint(3, 7)):
            tasks.append(
                {
                    "task_id": task_id,
                    "project_id": pid,
                    "assignee_id": rng.choice(assigned)["employee_id"],
                    "title": f"Task {task_id}",
                    "estimate_hours": rng.choice([4, 8, 16, 24, 40]),
                    "status": rng.choice(["todo", "doing", "done"]),
                }
            )
            task_id += 1
        for _ in range(rng.randint(6, 14)):
            emp = rng.choice(assigned)
            hours = round(rng.choice([1.0, 2.0, 4.0, 6.0, 8.0]), 1)
            rate = float(emp["hourly_rate"])
            amount = round(hours * rate, 2)
            logged += hours
            billed += amount
            times.append(
                {
                    "entry_id": time_id,
                    "project_id": pid,
                    "employee_id": emp["employee_id"],
                    "worked_on": date(2025, 1, 6) + timedelta(days=rng.randint(0, 180)),
                    "hours": hours,
                    "hourly_rate": rate,
                    "bill_amount": amount,
                    "billable": "true" if rng.random() < 0.85 else "false",
                }
            )
            time_id += 1
        for _ in range(rng.randint(1, 4)):
            amt = round(rng.uniform(120, 4500), 2)
            spent += amt
            costs.append(
                {
                    "cost_id": cost_id,
                    "project_id": pid,
                    "cost_date": date(2025, 1, 10) + timedelta(days=rng.randint(0, 170)),
                    "category": rng.choice(["travel", "software", "subcontractor", "other"]),
                    "amount": amt,
                }
            )
            cost_id += 1
        net = round(billed * rng.uniform(0.4, 1.0), 2)
        tax = round(net * 0.19, 2)
        gross = round(net + tax, 2)
        invoices.append(
            {
                "invoice_id": inv_id,
                "project_id": pid,
                "customer_id": cust["customer_id"],
                "invoice_date": date(2025, 2, 1) + timedelta(days=rng.randint(0, 150)),
                "net_amount": net,
                "tax_amount": tax,
                "gross_amount": gross,
                "status": rng.choice(["draft", "sent", "paid"]),
            }
        )
        inv_id += 1
        remaining = round(budget_amt - spent - billed * 0.3, 2)
        budgets.append(
            {
                "budget_id": pid,
                "project_id": pid,
                "amount": budget_amt,
                "spent_amount": round(spent + billed * 0.3, 2),
                "remaining_amount": remaining,
            }
        )
        projects[-1]["logged_hours"] = round(logged, 1)

    schema = {
        "customers": {
            "columns": {
                "customer_id": col("integer"),
                "name": col("text"),
                "industry": col("text"),
                "city": col("text"),
            },
            "primary_key": ["customer_id"],
        },
        "teams": {
            "columns": {
                "team_id": col("integer"),
                "name": col("text"),
                "location": col("text"),
            },
            "primary_key": ["team_id"],
        },
        "employees": {
            "columns": {
                "employee_id": col("integer"),
                "team_id": col("integer"),
                "name": col("text"),
                "role": col("text"),
                "hourly_rate": col("decimal"),
                "hired_on": col("date"),
            },
            "primary_key": ["employee_id"],
            "foreign_keys": [fk(["team_id"], "teams", ["team_id"])],
        },
        "projects": {
            "columns": {
                "project_id": col("integer"),
                "customer_id": col("integer"),
                "name": col("text"),
                "type": col("text"),
                "started_on": col("date"),
                "status": col("text"),
                "logged_hours": col("decimal"),
            },
            "primary_key": ["project_id"],
            "foreign_keys": [fk(["customer_id"], "customers", ["customer_id"])],
        },
        "assignments": {
            "columns": {
                "assignment_id": col("integer"),
                "project_id": col("integer"),
                "employee_id": col("integer"),
                "role": col("text"),
                "allocation_pct": col("integer"),
            },
            "primary_key": ["assignment_id"],
            "foreign_keys": [
                fk(["project_id"], "projects", ["project_id"]),
                fk(["employee_id"], "employees", ["employee_id"]),
            ],
        },
        "tasks": {
            "columns": {
                "task_id": col("integer"),
                "project_id": col("integer"),
                "assignee_id": col("integer"),
                "title": col("text"),
                "estimate_hours": col("integer"),
                "status": col("text"),
            },
            "primary_key": ["task_id"],
            "foreign_keys": [
                fk(["project_id"], "projects", ["project_id"]),
                fk(["assignee_id"], "employees", ["employee_id"]),
            ],
        },
        "time_entries": {
            "columns": {
                "entry_id": col("integer"),
                "project_id": col("integer"),
                "employee_id": col("integer"),
                "worked_on": col("date"),
                "hours": col("decimal"),
                "hourly_rate": col("decimal"),
                "bill_amount": col("decimal"),
                "billable": col("boolean"),
            },
            "primary_key": ["entry_id"],
            "foreign_keys": [
                fk(["project_id"], "projects", ["project_id"]),
                fk(["employee_id"], "employees", ["employee_id"]),
            ],
        },
        "costs": {
            "columns": {
                "cost_id": col("integer"),
                "project_id": col("integer"),
                "cost_date": col("date"),
                "category": col("text"),
                "amount": col("decimal"),
            },
            "primary_key": ["cost_id"],
            "foreign_keys": [fk(["project_id"], "projects", ["project_id"])],
        },
        "invoices": {
            "columns": {
                "invoice_id": col("integer"),
                "project_id": col("integer"),
                "customer_id": col("integer"),
                "invoice_date": col("date"),
                "net_amount": col("decimal"),
                "tax_amount": col("decimal"),
                "gross_amount": col("decimal"),
                "status": col("text"),
            },
            "primary_key": ["invoice_id"],
            "foreign_keys": [
                fk(["project_id"], "projects", ["project_id"]),
                fk(["customer_id"], "customers", ["customer_id"]),
            ],
        },
        "budgets": {
            "columns": {
                "budget_id": col("integer"),
                "project_id": col("integer"),
                "amount": col("decimal"),
                "spent_amount": col("decimal"),
                "remaining_amount": col("decimal"),
            },
            "primary_key": ["budget_id"],
            "foreign_keys": [fk(["project_id"], "projects", ["project_id"])],
        },
    }
    truth = {
        "relationships": [
            {"from": "employees.team_id", "to": "teams.team_id"},
            {"from": "projects.customer_id", "to": "customers.customer_id"},
            {"from": "assignments.project_id", "to": "projects.project_id"},
            {"from": "assignments.employee_id", "to": "employees.employee_id"},
            {"from": "tasks.project_id", "to": "projects.project_id"},
            {"from": "tasks.assignee_id", "to": "employees.employee_id"},
            {"from": "time_entries.project_id", "to": "projects.project_id"},
            {"from": "time_entries.employee_id", "to": "employees.employee_id"},
            {"from": "costs.project_id", "to": "projects.project_id"},
            {"from": "invoices.project_id", "to": "projects.project_id"},
            {"from": "invoices.customer_id", "to": "customers.customer_id"},
            {"from": "budgets.project_id", "to": "projects.project_id"},
        ],
        "kpis": [
            {"name": "Billable Hours", "definition": "SUM(time_entries.hours)"},
            {"name": "Project Cost", "definition": "SUM(costs.amount)"},
            {"name": "Invoice Amount", "definition": "SUM(invoices.gross_amount)"},
        ],
    }
    emit(
        "projects_clean",
        {
            "customers": customers,
            "teams": teams,
            "employees": employees,
            "projects": projects,
            "assignments": assignments,
            "tasks": tasks,
            "time_entries": times,
            "costs": costs,
            "invoices": invoices,
            "budgets": budgets,
        },
        schema,
        truth,
    )


def generate_projects_dirty() -> None:
    rng = random.Random(SEED + 51)

    customers = [
        {
            "kd_nr": i,
            "kd_name": f"{COMPANIES[i % len(COMPANIES)]} Dienst {i}",
            "branche": rng.choice(["ÖD", "Energie", "Bank", "Handel"]),
            "ort": CITIES[i % len(CITIES)],
        }
        for i in range(1, 61)
    ]
    teams = [
        {"team_nr": i, "team_bez": n, "standort": SITES[i % len(SITES)]}
        for i, n in enumerate(
            ["Delivery", "Design", "Data", "PMO", "Support", "Sales", "Ops", "QA", "Arch", "Mobil"],
            start=1,
        )
    ]
    employees = [
        {
            "ma_nr": i,
            "team_nr": 1 + ((i - 1) % len(teams)),
            "name": f"{FIRST[i % len(FIRST)]} {LAST[i % len(LAST)]}",
            "rolle_txt": rng.choice(["Berater", "PL", "Design", "Dev"]),
            "stdsatz": round(65 + (i % 11) * 12, 2),
            "eintritt": date(2018, 2, 1) + timedelta(days=i * 14),
        }
        for i in range(1, 81)
    ]
    projects = []
    assignments = []
    tasks = []
    times = []
    costs = []
    invoices = []
    budgets = []
    asg_id = 1
    task_id = 1
    time_id = 1
    cost_id = 1
    inv_id = 1
    for pid in range(1, 121):
        cust = rng.choice(customers)
        budget_amt = round(rng.uniform(18000, 320000), 2)
        logged = 0.0
        spent = 0.0
        billed = 0.0
        projects.append(
            {
                "prj_nr": pid,
                "kd_nr": blank(rng, cust["kd_nr"], 0.03),
                "prj_bez": f"Vorhaben {pid}",
                "typ_txt": rng.choice(["FP", "T&M", "Retainer"]),
                "start_dat": date(2024, 6, 1) + timedelta(days=rng.randint(0, 400)),
                "stat_txt": rng.choice(["läuft", "fertig", "stopp"]),
                "std_ist": 0.0,
            }
        )
        assigned = rng.sample(employees, k=rng.randint(2, 4))
        for emp in assigned:
            assignments.append(
                {
                    "zuord_id": asg_id,
                    "prj_nr": pid,
                    "ma_nr": emp["ma_nr"],
                    "rolle_txt": rng.choice(["TL", "MA"]),
                    "ausl_pct": rng.choice([20, 40, 60, 80, 100]),
                }
            )
            asg_id += 1
        for _ in range(rng.randint(4, 8)):
            tasks.append(
                {
                    "aufg_nr": task_id,
                    "prj_nr": pid,
                    "ma_nr": rng.choice(assigned)["ma_nr"],
                    "titel": f"Aufg {task_id}",
                    "plan_h": rng.choice([4, 8, 16, 32]),
                    "stat_txt": rng.choice(["offen", "arbeit", "fertig"]),
                }
            )
            task_id += 1
        for _ in range(rng.randint(12, 28)):
            if time_id > 2800:
                break
            emp = rng.choice(assigned)
            hours = round(rng.choice([1.0, 2.5, 4.0, 8.0]), 1)
            rate = float(emp["stdsatz"])
            amount = round(hours * rate, 2)
            logged += hours
            billed += amount
            times.append(
                {
                    "zeit_id": time_id,
                    "prj_nr": pid,
                    "ma_nr": emp["ma_nr"],
                    "tag": date(2025, 1, 3) + timedelta(days=rng.randint(0, 200)),
                    "std": hours,
                    "stdsatz": rate,
                    "betrag": amount,
                    "fakt_kz": "true" if rng.random() < 0.8 else "false",
                }
            )
            time_id += 1
        for _ in range(rng.randint(2, 5)):
            amt = round(rng.uniform(80, 5200), 2)
            spent += amt
            costs.append(
                {
                    "ko_nr": cost_id,
                    "prj_nr": pid,
                    "ko_dat": date(2025, 1, 8) + timedelta(days=rng.randint(0, 190)),
                    "art": rng.choice(["Reise", "Lizenz", "NachU", "Sonst"]),
                    "betrag": amt,
                }
            )
            cost_id += 1
        netto = round(billed * rng.uniform(0.3, 0.95), 2)
        mwst = round(netto * 0.19, 2)
        brutto = round(netto + mwst, 2)
        invoices.append(
            {
                "rg_nr": inv_id,
                "prj_nr": pid,
                "kd_nr": cust["kd_nr"],
                "rg_dat": date(2025, 2, 5) + timedelta(days=rng.randint(0, 160)),
                "netto_betrag": netto,
                "mwst_betrag": mwst,
                "brutto": brutto,
                "stat_txt": rng.choice(["entwurf", "versandt", "bezahlt"]),
            }
        )
        inv_id += 1
        spent_store = round(spent + billed * 0.25, 2)
        budgets.append(
            {
                "bdg_id": pid,
                "prj_nr": pid,
                "budget_betrag": budget_amt,
                "verbraucht": spent_store,
                "rest": round(budget_amt - spent_store, 2),
            }
        )
        # dirty: stored hours may drift from the true sum
        projects[-1]["std_ist"] = round(logged * rng.choice([1.0, 1.0, 1.0, 0.92]), 1)
        if time_id > 2800:
            break
    schema = {
        "customers": {
            "columns": {
                "kd_nr": col("integer"),
                "kd_name": col("text"),
                "branche": col("text"),
                "ort": col("text"),
            },
            "primary_key": ["kd_nr"],
        },
        "teams": {
            "columns": {
                "team_nr": col("integer"),
                "team_bez": col("text"),
                "standort": col("text"),
            },
            "primary_key": ["team_nr"],
        },
        "employees": {
            "columns": {
                "ma_nr": col("integer"),
                "team_nr": col("integer"),
                "name": col("text"),
                "rolle_txt": col("text"),
                "stdsatz": col("decimal"),
                "eintritt": col("date"),
            },
            "primary_key": ["ma_nr"],
            "foreign_keys": [fk(["team_nr"], "teams", ["team_nr"])],
        },
        "projects": {
            "columns": {
                "prj_nr": col("integer"),
                "kd_nr": col("integer", True),
                "prj_bez": col("text"),
                "typ_txt": col("text"),
                "start_dat": col("date"),
                "stat_txt": col("text"),
                "std_ist": col("decimal"),
            },
            "primary_key": ["prj_nr"],
            "foreign_keys": [],
        },
        "assignments": {
            "columns": {
                "zuord_id": col("integer"),
                "prj_nr": col("integer"),
                "ma_nr": col("integer"),
                "rolle_txt": col("text"),
                "ausl_pct": col("integer"),
            },
            "primary_key": ["zuord_id"],
            "foreign_keys": [fk(["prj_nr"], "projects", ["prj_nr"])],
        },
        "tasks": {
            "columns": {
                "aufg_nr": col("integer"),
                "prj_nr": col("integer"),
                "ma_nr": col("integer"),
                "titel": col("text"),
                "plan_h": col("integer"),
                "stat_txt": col("text"),
            },
            "primary_key": ["aufg_nr"],
            "foreign_keys": [fk(["prj_nr"], "projects", ["prj_nr"])],
        },
        "time_entries": {
            "columns": {
                "zeit_id": col("integer"),
                "prj_nr": col("integer"),
                "ma_nr": col("integer"),
                "tag": col("date"),
                "std": col("decimal"),
                "stdsatz": col("decimal"),
                "betrag": col("decimal"),
                "fakt_kz": col("boolean"),
            },
            "primary_key": ["zeit_id"],
            "foreign_keys": [fk(["prj_nr"], "projects", ["prj_nr"])],
        },
        "costs": {
            "columns": {
                "ko_nr": col("integer"),
                "prj_nr": col("integer"),
                "ko_dat": col("date"),
                "art": col("text"),
                "betrag": col("decimal"),
            },
            "primary_key": ["ko_nr"],
            "foreign_keys": [],
        },
        "invoices": {
            "columns": {
                "rg_nr": col("integer"),
                "prj_nr": col("integer"),
                "kd_nr": col("integer"),
                "rg_dat": col("date"),
                "netto_betrag": col("decimal"),
                "mwst_betrag": col("decimal"),
                "brutto": col("decimal"),
                "stat_txt": col("text"),
            },
            "primary_key": ["rg_nr"],
            "foreign_keys": [fk(["prj_nr"], "projects", ["prj_nr"])],
        },
        "budgets": {
            "columns": {
                "bdg_id": col("integer"),
                "prj_nr": col("integer"),
                "budget_betrag": col("decimal"),
                "verbraucht": col("decimal"),
                "rest": col("decimal"),
            },
            "primary_key": ["bdg_id"],
            "foreign_keys": [fk(["prj_nr"], "projects", ["prj_nr"])],
        },
    }
    truth = {
        "relationships": [
            {"from": "employees.team_nr", "to": "teams.team_nr"},
            {"from": "projects.kd_nr", "to": "customers.kd_nr"},
            {"from": "assignments.prj_nr", "to": "projects.prj_nr"},
            {"from": "assignments.ma_nr", "to": "employees.ma_nr"},
            {"from": "tasks.prj_nr", "to": "projects.prj_nr"},
            {"from": "tasks.ma_nr", "to": "employees.ma_nr"},
            {"from": "time_entries.prj_nr", "to": "projects.prj_nr"},
            {"from": "time_entries.ma_nr", "to": "employees.ma_nr"},
            {"from": "costs.prj_nr", "to": "projects.prj_nr"},
            {"from": "invoices.prj_nr", "to": "projects.prj_nr"},
            {"from": "invoices.kd_nr", "to": "customers.kd_nr"},
            {"from": "budgets.prj_nr", "to": "projects.prj_nr"},
        ],
        "kpis": [
            {"name": "Billable Hours", "definition": "SUM(time_entries.std)"},
            {"name": "Project Cost", "definition": "SUM(costs.betrag)"},
            {"name": "Invoice Amount", "definition": "SUM(invoices.brutto)"},
        ],
    }
    emit(
        "projects_dirty",
        {
            "customers": customers,
            "teams": teams,
            "employees": employees,
            "projects": projects,
            "assignments": assignments,
            "tasks": tasks,
            "time_entries": times,
            "costs": costs,
            "invoices": invoices,
            "budgets": budgets,
        },
        schema,
        truth,
    )