#!/usr/bin/env python3
"""Render the README hero: an animated "bus nervous system" illustration.

The scene is an *illustration* of KNXBench's telegram-flow view, drawn with
the devices and group addresses of the fictional manual sample house
(tools/manual_sample_project.py). Nothing here is captured from a real bus.

Output is a single self-contained SVG: SMIL for motion (GitHub renders SVG
through <img>, where scripts never run but SMIL does) and a
prefers-color-scheme media query that swaps the green-CRT dark look for
green ink on porcelain in light mode.

Usage: python3 tools/readme_hero_svg.py docs/assets/readme/bus-nervous-system.svg
"""
import sys

W, H = 960, 440
CYCLE = 9.0  # seconds for one full loop of the story
NODE_W, NODE_H = 172, 50

# (key, name, address, x, y) — centre coordinates
DEVICES = [
    ('sa_gf', 'Switch actuator GF', '1.1.1', 480, 220),
    ('pb_liv', 'Push button living', '1.1.10', 150, 92),
    ('pb_kit', 'Push button kitchen', '1.1.11', 120, 268),
    ('pb_hall', 'Push button hall', '1.2.11', 300, 384),
    ('pb_bed', 'Push button bedroom', '1.2.10', 400, 64),
    ('sa_ff', 'Switch actuator FF', '1.2.1', 770, 64),
    ('rt_liv', 'Thermostat living', '1.1.20', 590, 384),
]
# Unresolved destinations: group-address nodes, drawn differently on purpose.
GA_NODES = [
    ('ga_status', '0/1/1', 'status', 840, 212),
    ('ga_valve', '2/1/1', 'valve', 868, 380),
]

# (from, to, group address, bend, start time, travel time, value shown)
EDGES = [
    ('pb_liv', 'sa_gf', '0/0/1', 40, 0.3, 1.1, 'On'),
    ('sa_gf', 'ga_status', '0/1/1', -30, 1.5, 0.9, 'On'),
    ('pb_kit', 'sa_gf', '0/0/2', -30, 2.6, 1.0, 'On'),
    ('pb_bed', 'sa_ff', '0/0/4', 34, 3.4, 1.2, 'On'),
    ('rt_liv', 'ga_valve', '2/1/1', 26, 4.6, 0.9, '38 %'),
    ('pb_hall', 'sa_gf', '0/0/3', 36, 5.5, 1.0, 'Off'),
    ('sa_gf', 'ga_status', '0/1/1', -30, 6.7, 0.9, 'Off'),
]

NODES = {k: (x, y) for k, _, _, x, y in DEVICES} | {k: (x, y) for k, _, _, x, y in GA_NODES}
GA_KEYS = {k for k, *_ in GA_NODES}
LEADER = 'sa_gf'  # sends the most in this story, so the view centres it

STYLE = """
  .bg { fill: #050505; }
  .scan { opacity: .07; }
  .node { fill: #050b06; stroke: #39ff14; stroke-width: 1.4; filter: url(#glow); }
  .ganode { fill: #050505; stroke: #91b48b; stroke-width: 1.2; stroke-dasharray: 4 3; }
  .name { fill: #bedbbb; font: 500 12.5px var(--mono); }
  .addr { fill: #39ff14; font: 600 13px var(--mono); }
  .muted { fill: #91b48b; font: 11px var(--mono); }
  .edge { fill: none; stroke: #31573b; stroke-width: 1.6; }
  .edge-hot { fill: none; stroke: #39ff14; stroke-width: 2.2; filter: url(#glow); }
  .galabel { fill: #b2ffc0; font: 600 11.5px var(--mono); }
  .galbg { fill: #050505; }
  .pulse { fill: #b2ffc0; filter: url(#glow); }
  .value { fill: #050505; font: 700 11px var(--mono); }
  .valbg { fill: #39ff14; }
  .ring { fill: none; stroke: #39ff14; }
  .title { fill: #39ff14; font: 600 14px var(--mono); }
  @media (prefers-color-scheme: light) {
    .bg { fill: #f5f6fa; }
    .scan { opacity: 0; }
    .node { fill: #ffffff; stroke: #137551; filter: none; }
    .ganode { fill: #f5f6fa; stroke: #60697d; }
    .name { fill: #202438; }
    .addr { fill: #087a62; }
    .muted { fill: #60697d; }
    .edge { stroke: #c9d3cf; }
    .edge-hot { stroke: #087a62; filter: none; }
    .galabel { fill: #087a62; }
    .galbg { fill: #f5f6fa; }
    .pulse { fill: #087a62; filter: none; }
    .value { fill: #ffffff; }
    .valbg { fill: #087a62; }
    .ring { stroke: #087a62; }
    .title { fill: #087a62; }
  }
"""


def anchor(key, toward):
    """Point on the node's border facing `toward` (rect for devices, pill for GAs)."""
    x, y = NODES[key]
    tx, ty = toward
    dx, dy = tx - x, ty - y
    hw = (NODE_W if key not in GA_KEYS else 92) / 2
    hh = NODE_H / 2 if key not in GA_KEYS else 17
    if dx == 0 and dy == 0:
        return x, y
    scale = min(hw / abs(dx) if dx else 1e9, hh / abs(dy) if dy else 1e9)
    return x + dx * scale, y + dy * scale


def edge_path(a, b, bend):
    ax, ay = anchor(a, NODES[b])
    bx, by = anchor(b, NODES[a])
    mx, my = (ax + bx) / 2, (ay + by) / 2
    dx, dy = bx - ax, by - ay
    length = (dx * dx + dy * dy) ** 0.5 or 1
    nx, ny = -dy / length, dx / length
    cx, cy = mx + nx * bend, my + ny * bend
    label = (0.25 * ax + 0.5 * cx + 0.25 * bx, 0.25 * ay + 0.5 * cy + 0.25 * by)
    return f'M{ax:.1f},{ay:.1f} Q{cx:.1f},{cy:.1f} {bx:.1f},{by:.1f}', label


def window(start, end, hold=0.0):
    """keyTimes/values for 'invisible → visible during [start,end+hold] → invisible'."""
    t = lambda s: f'{min(max(s / CYCLE, 0), 1):.4f}'
    return t(start), t(start + 0.01), t(end + hold), t(end + hold + 0.01)


def device(key, name, addr, x, y):
    left, top = x - NODE_W / 2, y - NODE_H / 2
    return (f'<g><rect class="node" x="{left}" y="{top}" width="{NODE_W}" height="{NODE_H}" rx="4"/>'
            f'<text class="name" x="{x}" y="{y - 4}" text-anchor="middle">{name}</text>'
            f'<text class="addr" x="{x}" y="{y + 15}" text-anchor="middle">{addr}</text></g>')


def ga_node(key, ga, what, x, y):
    return (f'<g><rect class="ganode" x="{x - 46}" y="{y - 17}" width="92" height="34" rx="17"/>'
            f'<text class="addr" x="{x}" y="{y + 1}" text-anchor="middle">{ga}</text>'
            f'<text class="muted" x="{x}" y="{y + 30}" text-anchor="middle">{what} · no receiver</text></g>')


def render():
    out = [f'<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" '
           f'viewBox="0 0 {W} {H}" width="{W}" height="{H}" role="img" '
           f'aria-labelledby="t d" style="--mono: \'JetBrains Mono\', ui-monospace, SFMono-Regular, Menlo, Consolas, monospace">',
           '<title id="t">KNXBench bus nervous system (illustration)</title>',
           '<desc id="d">Illustration of the telegram-flow view: devices of a fictional sample house, '
           'curved connections labelled with group addresses, and pulses travelling along them when a telegram is observed.</desc>',
           f'<style>{STYLE}</style>',
           '<defs><filter id="glow" x="-30%" y="-30%" width="160%" height="160%">'
           '<feGaussianBlur stdDeviation="2.2" result="b"/><feMerge><feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge></filter>'
           '<pattern id="scan" width="4" height="4" patternUnits="userSpaceOnUse"><rect width="4" height="1.3" fill="#39ff14"/></pattern></defs>',
           f'<rect class="bg" width="{W}" height="{H}" rx="10"/>',
           f'<rect class="scan" width="{W}" height="{H}" rx="10" fill="url(#scan)"/>',
           '<text class="title" x="24" y="34">&gt; knx monitor --flow<tspan class="muted" dx="12">tunnelling · observing</tspan></text>',
           f'<text class="muted" x="24" y="{H - 16}">illustration · fictional sample house</text>']
    pulses = []

    # Static edges, deduplicated (the status edge fires twice).
    seen = {}
    for a, b, ga, bend, *_ in EDGES:
        if (a, b, ga) in seen:
            continue
        d, label = edge_path(a, b, bend)
        seen[(a, b, ga)] = (d, label)
        out.append(f'<path id="e-{a}-{b}-{ga.replace("/", "_")}" class="edge" d="{d}"/>')

    # Pulses, hot edges and value slots, each living in its own time window.
    for i, (a, b, ga, bend, start, travel, value) in enumerate(EDGES):
        pid = f'e-{a}-{b}-{ga.replace("/", "_")}'
        k0, k1, k2, k3 = window(start, start + travel)
        d, _ = seen[(a, b, ga)]
        out.append(f'<path class="edge-hot" d="{d}" opacity="0">'
                   f'<animate attributeName="opacity" dur="{CYCLE}s" repeatCount="indefinite" '
                   f'keyTimes="0;{k0};{k1};{k2};{k3};1" values="0;0;1;1;0;0"/></path>')
        pulses.append(f'<circle class="pulse" r="5" opacity="0">'
                   f'<animateMotion dur="{CYCLE}s" repeatCount="indefinite" calcMode="linear" '
                   f'keyPoints="0;0;1;1" keyTimes="0;{k0};{k2};1"><mpath xlink:href="#{pid}" href="#{pid}"/></animateMotion>'
                   f'<animate attributeName="opacity" dur="{CYCLE}s" repeatCount="indefinite" '
                   f'keyTimes="0;{k0};{k1};{k2};{k3};1" values="0;0;1;1;0;0"/></circle>')
        # The value appears when the telegram is admitted (pulse start), not on arrival,
        # mirroring the real view's rule; it fades after a short hold.
        bx, by = NODES[b]
        oy = -NODE_H / 2 - 14 if b not in GA_KEYS else -30
        v0, v1, v2, v3 = window(start, start, hold=2.2)
        label = f'{ga} = {value}'
        width = 9 + 7.2 * len(label)
        out.append(f'<g opacity="0"><rect class="valbg" x="{bx - width / 2:.1f}" y="{by + oy - 11:.1f}" width="{width:.1f}" height="17" rx="3"/>'
                   f'<text class="value" x="{bx}" y="{by + oy + 2:.1f}" text-anchor="middle">{label}</text>'
                   f'<animate attributeName="opacity" dur="{CYCLE}s" repeatCount="indefinite" '
                   f'keyTimes="0;{v0};{v1};{v2};{v3};1" values="0;0;1;1;0;0"/></g>')

    # Group-address labels last among the edge layers, so a hot edge never hides one.
    for (a, b, ga), (d, (lx, ly)) in seen.items():
        out.append(f'<rect class="galbg" x="{lx - 22}" y="{ly - 9}" width="44" height="17" rx="3"/>'
                   f'<text class="galabel" x="{lx}" y="{ly + 4}" text-anchor="middle">{ga}</text>')

    out += pulses  # above the labels: a pulse passing a label stays visible

    # The activity leader breathes.
    lx, ly = NODES[LEADER]
    out.append(f'<rect class="ring" x="{lx - NODE_W / 2 - 6}" y="{ly - NODE_H / 2 - 6}" width="{NODE_W + 12}" height="{NODE_H + 12}" rx="8">'
               f'<animate attributeName="stroke-opacity" values=".7;.1;.7" dur="2.4s" repeatCount="indefinite"/></rect>')
    out.append(f'<text class="muted" x="{lx}" y="{ly + NODE_H / 2 + 22}" text-anchor="middle">activity leader</text>')

    out += [device(*d) for d in DEVICES]
    out += [ga_node(*g) for g in GA_NODES]
    out.append('</svg>')
    return '\n'.join(out) + '\n'


if __name__ == '__main__':
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    with open(sys.argv[1], 'w', encoding='utf-8') as f:
        f.write(render())
