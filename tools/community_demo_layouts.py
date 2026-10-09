"""Original fictional buildings and signal plans for the offline demos.

This is authoring data/tooling, not an application manufacturer catalogue.
Each channel is allocated to one named room. All generated group addresses
have a matching DPT and at least one source and destination.
"""
from __future__ import annotations

from collections import defaultdict


def room(key, name, parent, zone, *, dim=False, blind=False, heat=False, presence=False):
    return dict(key=key, name=name, parent=parent, zone=zone,
                dim=dim, blind=blind, heat=heat, presence=presence)


def layouts():
    home = [
        room("living", "Living room", "ground", 0, dim=True, blind=True, heat=True),
        room("kitchen", "Kitchen", "ground", 0, dim=True, blind=True, heat=True),
        room("dining", "Dining room", "ground", 0, dim=True, blind=True, heat=True),
        room("hall", "Entrance hall", "ground", 0, heat=True, presence=True),
        room("study", "Study", "ground", 0, dim=True, blind=True, heat=True),
        room("master", "Main bedroom", "first", 1, dim=True, blind=True, heat=True),
        room("bedroom2", "Bedroom 2", "first", 1, dim=True, blind=True, heat=True),
        room("bedroom3", "Bedroom 3", "first", 1, blind=True, heat=True),
        room("bath", "Bathroom", "first", 1, heat=True, presence=True),
        room("utility", "Utility room", "basement", 2, heat=True, presence=True),
        room("garage", "Garage", "outside", 3, presence=True),
        room("patio", "Patio", "outside", 3),
    ]
    home_parents = [("basement", "Basement and technical", "Floor", "building"),
                    ("ground", "Ground floor", "Floor", "building"),
                    ("first", "First floor", "Floor", "building"),
                    ("outside", "Garage and outside", "BuildingPart", "building")]
    residential_parents = [("basement", "Basement and shared technical", "Floor", "building")]
    residential = []
    for floor in range(3):
        fkey = f"floor-{floor}"
        residential_parents.append((fkey, ["Ground floor", "First floor", "Second floor"][floor], "Floor", "building"))
        for n in range(floor * 2 + 1, floor * 2 + 3):
            akey = f"apt-{n}"
            residential_parents.append((akey, f"Apartment {n:02d}", "BuildingPart", fkey))
            for index, name in enumerate(["Living room", "Kitchen", "Main bedroom", "Bedroom 2", "Bathroom", "Hall"]):
                residential.append(room(f"apt-{n}-room-{index}", f"Apartment {n:02d} / {name}", akey, n,
                                        dim=index == 0, blind=index < 4, heat=True))
    for index, (name, parent) in enumerate([("Shared entrance", "floor-0"), ("Ground floor stairway", "floor-0"),
                                           ("First floor stairway", "floor-1"), ("Second floor stairway", "floor-2"),
                                           ("Shared laundry", "basement")]):
        residential.append(room(f"common-{index}", name, parent, 0,
                                dim=index == 0, heat=index == 4, presence=True))
    office = []
    office_parents = []
    for floor in range(3):
        fkey = f"floor-{floor}"
        label = ["Ground floor", "First floor", "Second floor"][floor]
        office_parents.append((fkey, label, "Floor", "building"))
        names = ([f"Open office zone {letter}" for letter in "ABCD"]
                 + [f"Private office {n:02d}" for n in range(1, 5)]
                 + [f"Meeting room {n:02d}" for n in range(1, 4)]
                 + ["East corridor", "West corridor", "Kitchenette", "Print room", "Restroom", "Technical room",
                    "Reception" if floor == 0 else "Breakout lounge"])
        for index, name in enumerate(names):
            office.append(room(f"office-{floor}-{index}", f"{label} / {name}", fkey, floor,
                               dim=index < 11 or index == 17, blind=index < 11,
                               heat=index < 11 or index in (13, 17), presence=index != 16))
    return [
        dict(key="single-family-home", name="Single-Family Home", rooms=home, parents=home_parents,
             zones={0: "Ground floor", 1: "First floor", 2: "Basement", 3: "Garage and outside"},
             zone_lines={0: 1, 1: 1, 2: 1, 3: 1}, lines={0: "Main line / IP interface", 1: "House TP line"},
             keypad_zones=[0], keypad_scope="all", board_parent="basement"),
        dict(key="multi-unit-residential", name="Multi-Unit Residential Building", rooms=residential,
             parents=residential_parents, zones={0: "Shared areas", **{n: f"Apartment {n:02d}" for n in range(1, 7)}},
             zone_lines={0: 7, **{n: n for n in range(1, 7)}},
             lines={0: "Main line / IP interface", **{n: f"Apartment {n:02d} TP line" for n in range(1, 7)}, 7: "Shared areas TP line"},
             keypad_zones=list(range(7)), keypad_scope="zone", board_parent="basement"),
        dict(key="office-building", name="Office Building", rooms=office, parents=office_parents,
             zones={n: ["Ground floor", "First floor", "Second floor"][n] for n in range(3)},
             zone_lines={n: n + 1 for n in range(3)},
             lines={0: "Main line / IP interface", 1: "Ground floor TP line", 2: "First floor TP line", 3: "Second floor TP line"},
             keypad_zones=list(range(3)), keypad_scope="zone", board_parent="floor-0"),
    ]


def build_spec(layout, products, product_ids):
    parts = [dict(key="building", name=layout["name"], kind="Building", parent=None)]
    parts.extend(dict(key=k, name=n, kind=t, parent=p) for k, n, t, p in layout["parents"])
    parts.extend(dict(key=r["key"], name=r["name"], kind="Room", parent=r["parent"]) for r in layout["rooms"])
    for zone, name in layout["zones"].items():
        parent = (f"apt-{zone}" if layout["key"] == "multi-unit-residential" and zone else
                  f"floor-{zone}" if layout["key"] == "office-building" else layout["board_parent"])
        parts.append(dict(key=f"board-{zone}", name=f"{name} distribution board", kind="DistributionBoard", parent=parent))
    parts.append(dict(key="main-board", name="Main distribution board", kind="DistributionBoard", parent=layout["board_parent"]))
    devices, groups = [], []
    next_address = defaultdict(lambda: 1)
    next_sub = defaultdict(lambda: 1)
    group_lookup = {}

    def group(main, zone, name, dpt, central=False):
        sub = next_sub[main, zone]
        if sub > 255:
            raise ValueError("Demo group range exhausted")
        next_sub[main, zone] += 1
        address = f"{main}/{zone}/{sub}"
        groups.append(dict(address=address, name=name, dpt=dpt, central=central))
        return address

    def add_device(key, name, zone, part, *, line=None, address=None):
        line = layout["zone_lines"][zone] if line is None else line
        if address is None:
            address = next_address[line]
            next_address[line] += 1
        device = dict(key=f"device-{len(devices) + 1}", name=name, product=key,
                      address=[1, line, address], part=part, links=[], description="Fictional offline demo device. No firmware or real bus operation.")
        devices.append(device)
        return device

    def link(device, number, *addresses):
        if not addresses:
            return
        binding = next((b for b in device["links"] if b["number"] == number), None)
        if binding is None:
            binding = dict(number=number, groups=[])
            device["links"].append(binding)
        for address in addresses:
            if address not in binding["groups"]:
                binding["groups"].append(address)

    add_device("IP", "Main line / KNX IP interface (illustrative)", 0, "main-board", line=0)
    for line, name in layout["lines"].items():
        if line:
            zone = next(z for z, l in layout["zone_lines"].items() if l == line)
            add_device("LC", f"{name} / Line coupler (illustrative)", zone, f"board-{zone}", line=line, address=0)

    central_off, central_scene = {}, {}
    for zone in layout["keypad_zones"]:
        name = "Whole house" if layout["keypad_scope"] == "all" else layout["zones"][zone]
        central_off[zone] = group(6, zone, f"{name} / Central off command (0 = off)", "1.001", True)
        central_scene[zone] = group(5, zone, f"{name} / Central scene recall command", "17.001", True)
        key = add_device("KEY", f"{name} / Central and scene keypad", zone, f"board-{zone}")
        link(key, 0, central_off[zone])
        link(key, 1, central_scene[zone])

    families = {"SW4": (0, ["Switch command", "Switch status"]),
                "DIM4": (1, ["Level command", "Level status"]),
                "BL4": (2, ["Blind move command", "Blind position command", "Blind position status"]),
                "HEAT4": (3, ["Actual temperature status", "Temperature setpoint command", "Heating valve status"])}
    dpt_by_name = {s["name"]: s["dpt"] for _, _, signals in products.values() for s in signals}
    wall_by_name = {s["name"]: i for i, s in enumerate(products["WALL"][2])}
    for r in layout["rooms"]:
        wall = add_device("WALL", f"{r['name']} / Wall controller", r["zone"], r["key"])
        enabled = {"SW4": True, "DIM4": r["dim"], "BL4": r["blind"], "HEAT4": r["heat"]}
        for key, (_, names) in families.items():
            if not enabled[key]:
                continue
            for name in names:
                ga = group(families[key][0], r["zone"], f"{r['name']} / {name}", dpt_by_name[name])
                group_lookup[r["key"], name] = ga
                link(wall, wall_by_name[name], ga)
        scene = group(5, r["zone"], f"{r['name']} / Scene recall command", "17.001")
        group_lookup[r["key"], "Scene recall command"] = scene
        link(wall, wall_by_name["Scene recall command"], scene)
        if r["presence"]:
            ga = group(4, r["zone"], f"{r['name']} / Occupancy status", "1.018")
            group_lookup[r["key"], "Occupancy status"] = ga
            presence = add_device("PRES", f"{r['name']} / Presence detector", r["zone"], r["key"])
            link(presence, 0, ga)

    for zone, zone_name in layout["zones"].items():
        for key, (_, names) in families.items():
            flag = {"SW4": None, "DIM4": "dim", "BL4": "blind", "HEAT4": "heat"}[key]
            rooms = [r for r in layout["rooms"] if r["zone"] == zone and (flag is None or r[flag])]
            _, channels, signals = products[key]
            for start in range(0, len(rooms), channels):
                actor = add_device(key, f"{zone_name} / {products[key][0]} {start // channels + 1:02d}", zone, f"board-{zone}")
                assignment = []
                for channel, r in enumerate(rooms[start:start + channels]):
                    assignment.append(f"Channel {channel + 1}: {r['name']}")
                    for n, s in enumerate(signals):
                        number = channel * len(signals) + n
                        ga = group_lookup.get((r["key"], s["name"]))
                        if ga:
                            link(actor, number, ga)
                        scope = 0 if layout["keypad_scope"] == "all" else zone
                        if s["name"] == "Switch command":
                            link(actor, number, central_off[scope])
                        if s["name"] == "Scene recall command":
                            link(actor, number, central_scene[scope])
                actor["description"] += " " + "; ".join(assignment) + ". Unassigned channels/objects are documented reserves."
    return dict(key=layout["key"], name=f"{layout['name']} (fictional demo)",
                zones=[dict(number=z, name=n) for z, n in layout["zones"].items()],
                lines=[dict(number=n, name=s) for n, s in layout["lines"].items()],
                parts=parts, devices=devices, groups=groups)
