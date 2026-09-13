"""Logistics, SaaS and project fixtures. Imported by domain_fixtures."""

from __future__ import annotations

import random
from datetime import date, datetime, timedelta

from domain_fixtures import (
    CITIES,
    COMPANIES,
    FIRST,
    LAST,
    SEED,
    SITES,
    blank,
    col,
    emit,
    fk,
    ts,
)


def generate_logistics_clean() -> None:
    rng = random.Random(SEED + 30)
    depots = [
        {
            "depot_id": i,
            "name": f"Depot {SITES[i % len(SITES)]}",
            "city": SITES[i % len(SITES)],
            "capacity_pallets": 400 + i * 80,
        }
        for i in range(1, 9)
    ]
    vehicles = [
        {
            "vehicle_id": i,
            "plate": f"HH-SP {100 + i}",
            "type": rng.choice(["van", "truck", "trailer"]),
            "depot_id": 1 + (i % len(depots)),
            "capacity_kg": rng.choice([800, 3500, 12000, 24000]),
        }
        for i in range(1, 26)
    ]
    drivers = [
        {
            "driver_id": i,
            "name": f"{FIRST[i % len(FIRST)]} {LAST[i % len(LAST)]}",
            "depot_id": 1 + (i % len(depots)),
            "license_class": rng.choice(["B", "C", "CE"]),
            "hired_on": date(2019, 1, 1) + timedelta(days=i * 20),
        }
        for i in range(1, 31)
    ]
    routes = []
    shipments = []
    stops = []
    scans = []
    statuses = []
    stop_id = 1
    scan_id = 1
    status_id = 1
    ship_id = 1
    for rid in range(1, 201):
        depot = rng.choice(depots)
        same_v = [v for v in vehicles if v["depot_id"] == depot["depot_id"]] or vehicles
        same_d = [d for d in drivers if d["depot_id"] == depot["depot_id"]] or drivers
        vehicle = rng.choice(same_v)
        driver = rng.choice(same_d)
        day = date(2025, 4, 1) + timedelta(days=rng.randint(0, 90))
        n_stops = rng.randint(3, 7)
        total_km = 0.0
        routes.append(
            {
                "route_id": rid,
                "depot_id": depot["depot_id"],
                "vehicle_id": vehicle["vehicle_id"],
                "driver_id": driver["driver_id"],
                "route_date": day,
                "planned_stops": n_stops,
                "total_distance_km": 0.0,
            }
        )
        for seq in range(1, n_stops + 1):
            if scan_id >= 2800:
                break
            leg = round(rng.uniform(4, 48), 1)
            total_km += leg
            promised = ts(day, min(8 + seq, 22), rng.choice([0, 15, 30, 45]))
            delay = rng.choice([-20, -5, 0, 10, 25, 55])
            actual_dt = datetime.strptime(promised, "%Y-%m-%d %H:%M:%S") + timedelta(minutes=delay)
            delivered = rng.random() < 0.92
            shipments.append(
                {
                    "shipment_id": ship_id,
                    "route_id": rid,
                    "depot_id": depot["depot_id"],
                    "consignee": f"{COMPANIES[seq % len(COMPANIES)]} {ship_id}",
                    "promised_at": promised,
                    "delivered_at": actual_dt.strftime("%Y-%m-%d %H:%M:%S") if delivered else "",
                    "delay_min": delay if delivered else "",
                    "weight_kg": round(rng.uniform(12, 420), 1),
                    "status": "delivered"
                    if delivered and delay <= 15
                    else rng.choice(["delivered", "late", "exception"]),
                }
            )
            dwell = rng.randint(4, 35)
            stops.append(
                {
                    "stop_id": stop_id,
                    "route_id": rid,
                    "shipment_id": ship_id,
                    "seq": seq,
                    "city": rng.choice(CITIES),
                    "leg_km": leg,
                    "dwell_min": dwell,
                    "arrived_at": actual_dt.strftime("%Y-%m-%d %H:%M:%S"),
                    "departed_at": (actual_dt + timedelta(minutes=dwell)).strftime("%Y-%m-%d %H:%M:%S"),
                }
            )
            for ev in ("pickup", "out_for_delivery", "delivered"):
                if ev == "delivered" and not delivered:
                    continue
                scans.append(
                    {
                        "scan_id": scan_id,
                        "shipment_id": ship_id,
                        "stop_id": stop_id,
                        "scanned_at": (actual_dt - timedelta(minutes=rng.randint(0, 40))).strftime(
                            "%Y-%m-%d %H:%M:%S"
                        ),
                        "event_type": ev,
                        "location": rng.choice(CITIES),
                    }
                )
                scan_id += 1
            statuses.append(
                {
                    "status_id": status_id,
                    "shipment_id": ship_id,
                    "status": "late" if delay > 15 else "on_time",
                    "changed_at": actual_dt.strftime("%Y-%m-%d %H:%M:%S"),
                }
            )
            status_id += 1
            stop_id += 1
            ship_id += 1
        routes[-1]["total_distance_km"] = round(total_km, 1)
        if scan_id >= 2800:
            break
    schema = {
        "depots": {
            "columns": {
                "depot_id": col("integer"),
                "name": col("text"),
                "city": col("text"),
                "capacity_pallets": col("integer"),
            },
            "primary_key": ["depot_id"],
        },
        "vehicles": {
            "columns": {
                "vehicle_id": col("integer"),
                "plate": col("text"),
                "type": col("text"),
                "depot_id": col("integer"),
                "capacity_kg": col("integer"),
            },
            "primary_key": ["vehicle_id"],
            "foreign_keys": [fk(["depot_id"], "depots", ["depot_id"])],
        },
        "drivers": {
            "columns": {
                "driver_id": col("integer"),
                "name": col("text"),
                "depot_id": col("integer"),
                "license_class": col("text"),
                "hired_on": col("date"),
            },
            "primary_key": ["driver_id"],
            "foreign_keys": [fk(["depot_id"], "depots", ["depot_id"])],
        },
        "routes": {
            "columns": {
                "route_id": col("integer"),
                "depot_id": col("integer"),
                "vehicle_id": col("integer"),
                "driver_id": col("integer"),
                "route_date": col("date"),
                "planned_stops": col("integer"),
                "total_distance_km": col("decimal"),
            },
            "primary_key": ["route_id"],
            "foreign_keys": [
                fk(["depot_id"], "depots", ["depot_id"]),
                fk(["vehicle_id"], "vehicles", ["vehicle_id"]),
                fk(["driver_id"], "drivers", ["driver_id"]),
            ],
        },
        "shipments": {
            "columns": {
                "shipment_id": col("integer"),
                "route_id": col("integer"),
                "depot_id": col("integer"),
                "consignee": col("text"),
                "promised_at": col("timestamp"),
                "delivered_at": col("timestamp", True),
                "delay_min": col("integer", True),
                "weight_kg": col("decimal"),
                "status": col("text"),
            },
            "primary_key": ["shipment_id"],
            "foreign_keys": [
                fk(["route_id"], "routes", ["route_id"]),
                fk(["depot_id"], "depots", ["depot_id"]),
            ],
        },
        "stops": {
            "columns": {
                "stop_id": col("integer"),
                "route_id": col("integer"),
                "shipment_id": col("integer"),
                "seq": col("integer"),
                "city": col("text"),
                "leg_km": col("decimal"),
                "dwell_min": col("integer"),
                "arrived_at": col("timestamp"),
                "departed_at": col("timestamp"),
            },
            "primary_key": ["stop_id"],
            "foreign_keys": [
                fk(["route_id"], "routes", ["route_id"]),
                fk(["shipment_id"], "shipments", ["shipment_id"]),
            ],
        },
        "scan_events": {
            "columns": {
                "scan_id": col("integer"),
                "shipment_id": col("integer"),
                "stop_id": col("integer"),
                "scanned_at": col("timestamp"),
                "event_type": col("text"),
                "location": col("text"),
            },
            "primary_key": ["scan_id"],
            "foreign_keys": [
                fk(["shipment_id"], "shipments", ["shipment_id"]),
                fk(["stop_id"], "stops", ["stop_id"]),
            ],
        },
        "delivery_status": {
            "columns": {
                "status_id": col("integer"),
                "shipment_id": col("integer"),
                "status": col("text"),
                "changed_at": col("timestamp"),
            },
            "primary_key": ["status_id"],
            "foreign_keys": [fk(["shipment_id"], "shipments", ["shipment_id"])],
        },
    }
    truth = {
        "relationships": [
            {"from": "vehicles.depot_id", "to": "depots.depot_id"},
            {"from": "drivers.depot_id", "to": "depots.depot_id"},
            {"from": "routes.depot_id", "to": "depots.depot_id"},
            {"from": "routes.vehicle_id", "to": "vehicles.vehicle_id"},
            {"from": "routes.driver_id", "to": "drivers.driver_id"},
            {"from": "shipments.route_id", "to": "routes.route_id"},
            {"from": "shipments.depot_id", "to": "depots.depot_id"},
            {"from": "stops.route_id", "to": "routes.route_id"},
            {"from": "stops.shipment_id", "to": "shipments.shipment_id"},
            {"from": "scan_events.shipment_id", "to": "shipments.shipment_id"},
            {"from": "scan_events.stop_id", "to": "stops.stop_id"},
            {"from": "delivery_status.shipment_id", "to": "shipments.shipment_id"},
        ],
        "kpis": [
            {"name": "Shipment Count", "definition": "COUNT(DISTINCT shipments.shipment_id)"},
            {"name": "Delay", "definition": "SUM(shipments.delay_min)"},
            {"name": "Distance", "definition": "SUM(routes.total_distance_km)"},
        ],
    }
    emit(
        "logistics_clean",
        {
            "depots": depots,
            "vehicles": vehicles,
            "drivers": drivers,
            "routes": routes,
            "shipments": shipments,
            "stops": stops,
            "scan_events": scans,
            "delivery_status": statuses,
        },
        schema,
        truth,
    )


def generate_logistics_dirty() -> None:
    rng = random.Random(SEED + 31)
    depots = [
        {
            "nl_id": i,
            "nl_bez": f"NL-{SITES[i % len(SITES)][:3].upper()}",
            "ort": SITES[i % len(SITES)],
            "kap_pal": 350 + i * 90,
        }
        for i in range(1, 11)
    ]
    vehicles = [
        {
            "fz_nr": i,
            "kennzeichen": f"M-SP {200 + i}",
            "fz_typ": rng.choice(["Sprinter", "LKW", "Sattel"]),
            "nl_id": 1 + (i % len(depots)),
            "nutzlast_kg": rng.choice([700, 2800, 11000, 22000]),
        }
        for i in range(1, 41)
    ]
    drivers = [
        {
            "fahrer_nr": i,
            "name": f"{FIRST[i % len(FIRST)]} {LAST[i % len(LAST)]}",
            "nl_id": 1 + (i % len(depots)),
            "fuehrerschein": rng.choice(["B", "C", "CE"]),
            "eintritt": date(2018, 5, 1) + timedelta(days=i * 17),
        }
        for i in range(1, 46)
    ]
    routes = []
    shipments = []
    stops = []
    scans = []
    statuses = []
    stop_id = 1
    scan_id = 1
    status_id = 1
    send_nr = 1
    for rid in range(1, 281):
        depot = rng.choice(depots)
        vehicle = rng.choice(vehicles)
        driver = rng.choice(drivers)
        day = date(2025, 1, 10) + timedelta(days=rng.randint(0, 120))
        n_stops = rng.randint(2, 6)
        total_km = 0.0
        routes.append(
            {
                "tour_nr": rid,
                "nl_id": depot["nl_id"],
                "fz_nr": vehicle["fz_nr"],
                "fahrer_nr": blank(rng, driver["fahrer_nr"], 0.06),
                "tour_dat": day,
                "plan_stops": n_stops,
                "km_summe": 0.0,
            }
        )
        for seq in range(1, n_stops + 1):
            if scan_id >= 4500:
                break
            leg = round(rng.uniform(3, 55), 1)
            total_km += leg
            promised = ts(day, min(7 + seq, 22), rng.choice([0, 10, 20, 40]))
            delay = rng.choice([-15, 0, 5, 18, 40, 90])
            actual_dt = datetime.strptime(promised, "%Y-%m-%d %H:%M:%S") + timedelta(minutes=delay)
            delivered = rng.random() < 0.9
            shipments.append(
                {
                    "send_nr": send_nr,
                    "tour_nr": rid,
                    "nl_id": depot["nl_id"],
                    "empf": f"Empf {send_nr}",
                    "soll_ts": promised,
                    "ist_ts": actual_dt.strftime("%Y-%m-%d %H:%M:%S") if delivered else "",
                    "versp_min": delay if delivered else "",
                    "gew_kg": round(rng.uniform(8, 500), 1),
                    "stat_txt": "zugestellt"
                    if delivered and delay <= 20
                    else rng.choice(["unterwegs", "verspätet", "fehl"]),
                }
            )
            dwell = rng.randint(3, 40)
            stops.append(
                {
                    "stop_nr": stop_id,
                    "tour_nr": rid,
                    "send_nr": send_nr,
                    "lfd": seq,
                    "ort": rng.choice(CITIES),
                    "etappe_km": leg,
                    "standzeit_min": dwell,
                    "ankunft": actual_dt.strftime("%Y-%m-%d %H:%M:%S"),
                    "abfahrt": (actual_dt + timedelta(minutes=dwell)).strftime("%Y-%m-%d %H:%M:%S"),
                }
            )
            events = ("uebernahme", "scan_hub", "zustellung")
            n_scans = 2 if rng.random() < 0.45 else 3
            for ev in events[:n_scans]:
                scans.append(
                    {
                        "scan_id": scan_id,
                        "send_nr": send_nr,
                        "stop_nr": stop_id,
                        "scan_ts": (actual_dt - timedelta(minutes=rng.randint(0, 50))).strftime(
                            "%Y-%m-%d %H:%M:%S"
                        ),
                        "evt": ev,
                        "ort_txt": rng.choice(CITIES),
                    }
                )
                scan_id += 1
            statuses.append(
                {
                    "hist_id": status_id,
                    "send_nr": send_nr,
                    "stat_txt": "ok" if delay <= 20 else "late",
                    "geaendert": actual_dt.strftime("%Y-%m-%d %H:%M:%S"),
                }
            )
            status_id += 1
            stop_id += 1
            send_nr += 1
        routes[-1]["km_summe"] = round(total_km, 1)
        if scan_id >= 4500:
            break
    schema = {
        "depots": {
            "columns": {
                "nl_id": col("integer"),
                "nl_bez": col("text"),
                "ort": col("text"),
                "kap_pal": col("integer"),
            },
            "primary_key": ["nl_id"],
        },
        "vehicles": {
            "columns": {
                "fz_nr": col("integer"),
                "kennzeichen": col("text"),
                "fz_typ": col("text"),
                "nl_id": col("integer"),
                "nutzlast_kg": col("integer"),
            },
            "primary_key": ["fz_nr"],
            "foreign_keys": [fk(["nl_id"], "depots", ["nl_id"])],
        },
        "drivers": {
            "columns": {
                "fahrer_nr": col("integer"),
                "name": col("text"),
                "nl_id": col("integer"),
                "fuehrerschein": col("text"),
                "eintritt": col("date"),
            },
            "primary_key": ["fahrer_nr"],
            "foreign_keys": [],
        },
        "routes": {
            "columns": {
                "tour_nr": col("integer"),
                "nl_id": col("integer"),
                "fz_nr": col("integer"),
                "fahrer_nr": col("integer", True),
                "tour_dat": col("date"),
                "plan_stops": col("integer"),
                "km_summe": col("decimal"),
            },
            "primary_key": ["tour_nr"],
            "foreign_keys": [fk(["nl_id"], "depots", ["nl_id"])],
        },
        "shipments": {
            "columns": {
                "send_nr": col("integer"),
                "tour_nr": col("integer"),
                "nl_id": col("integer"),
                "empf": col("text"),
                "soll_ts": col("timestamp"),
                "ist_ts": col("timestamp", True),
                "versp_min": col("integer", True),
                "gew_kg": col("decimal"),
                "stat_txt": col("text"),
            },
            "primary_key": ["send_nr"],
            "foreign_keys": [fk(["tour_nr"], "routes", ["tour_nr"])],
        },
        "stops": {
            "columns": {
                "stop_nr": col("integer"),
                "tour_nr": col("integer"),
                "send_nr": col("integer"),
                "lfd": col("integer"),
                "ort": col("text"),
                "etappe_km": col("decimal"),
                "standzeit_min": col("integer"),
                "ankunft": col("timestamp"),
                "abfahrt": col("timestamp"),
            },
            "primary_key": ["stop_nr"],
            "foreign_keys": [fk(["tour_nr"], "routes", ["tour_nr"])],
        },
        "scan_events": {
            "columns": {
                "scan_id": col("integer"),
                "send_nr": col("integer"),
                "stop_nr": col("integer"),
                "scan_ts": col("timestamp"),
                "evt": col("text"),
                "ort_txt": col("text"),
            },
            "primary_key": ["scan_id"],
            "foreign_keys": [],
        },
        "delivery_status": {
            "columns": {
                "hist_id": col("integer"),
                "send_nr": col("integer"),
                "stat_txt": col("text"),
                "geaendert": col("timestamp"),
            },
            "primary_key": ["hist_id"],
            "foreign_keys": [],
        },
    }
    truth = {
        "relationships": [
            {"from": "vehicles.nl_id", "to": "depots.nl_id"},
            {"from": "drivers.nl_id", "to": "depots.nl_id"},
            {"from": "routes.nl_id", "to": "depots.nl_id"},
            {"from": "routes.fz_nr", "to": "vehicles.fz_nr"},
            {"from": "routes.fahrer_nr", "to": "drivers.fahrer_nr"},
            {"from": "shipments.tour_nr", "to": "routes.tour_nr"},
            {"from": "shipments.nl_id", "to": "depots.nl_id"},
            {"from": "stops.tour_nr", "to": "routes.tour_nr"},
            {"from": "stops.send_nr", "to": "shipments.send_nr"},
            {"from": "scan_events.send_nr", "to": "shipments.send_nr"},
            {"from": "scan_events.stop_nr", "to": "stops.stop_nr"},
            {"from": "delivery_status.send_nr", "to": "shipments.send_nr"},
        ],
        "kpis": [
            {"name": "Shipment Count", "definition": "COUNT(DISTINCT shipments.send_nr)"},
            {"name": "Delay", "definition": "SUM(shipments.versp_min)"},
            {"name": "Distance", "definition": "SUM(routes.km_summe)"},
        ],
    }
    emit(
        "logistics_dirty",
        {
            "depots": depots,
            "vehicles": vehicles,
            "drivers": drivers,
            "routes": routes,
            "shipments": shipments,
            "stops": stops,
            "scan_events": scans,
            "delivery_status": statuses,
        },
        schema,
        truth,
    )
