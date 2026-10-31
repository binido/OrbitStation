"""Draws the OrbitStation icons into the given folder: make_icon.py assets/icons
Concept SVGs A/B/C from the logo exploration are written too; C is the chosen one."""
import math, sys

OUT = sys.argv[1]


def f(v):
    return f"{v:.2f}".rstrip("0").rstrip(".")


# Gamepad silhouette (v3): flat top, round shoulders, grips flaring outwards and down, a straight bridge
# between the inner grip curves. Mirror-symmetric about x=128, height 72..191. Scaled by `k` about (128,128).
OUTLINE = [
    ("M", (88, 72)), ("H", 168),
    ("C", (200, 72), (220, 84), (226, 112)),
    ("C", (232, 140), (236, 168), (224, 182)),
    ("C", (214, 194), (192, 194), (182, 182)),
    ("C", (172, 170), (166, 156), (152, 154)),
    ("H", 104),
    ("C", (90, 156), (84, 170), (74, 182)),
    ("C", (64, 194), (42, 194), (32, 182)),
    ("C", (20, 168), (24, 140), (30, 112)),
    ("C", (36, 84), (56, 72), (88, 72)),
]


def pad(dy=0, k=1.0, holes=True):
    p = lambda pt: f"{f(128 + (pt[0] - 128) * k)} {f(128 + (pt[1] + dy - 128) * k)}"
    d = ""
    for op, *a in OUTLINE:
        d += f"{op}{f(128 + (a[0] - 128) * k)} " if op == "H" else f"{op}{' '.join(p(x) for x in a)} "
    d += "Z"
    if holes:
        cy = 108 + dy
        # Vader layout: D-pad left, stick right; asymmetric so the pad never reads as a face
        d += plus(128 + (88 - 128) * k, 128 + (cy - 128) * k, 17 * k, 12 * k)
        d += circle(128 + (168 - 128) * k, 128 + (cy - 128) * k, 16 * k)
    return d


def circle(cx, cy, r):
    return f" M{f(cx - r)} {f(cy)} A{f(r)} {f(r)} 0 1 0 {f(cx + r)} {f(cy)} A{f(r)} {f(r)} 0 1 0 {f(cx - r)} {f(cy)} Z"


def plus(cx, cy, arm, w):
    h = w / 2
    return (f" M{f(cx - h)} {f(cy - arm)} H{f(cx + h)} V{f(cy - h)} H{f(cx + arm)} V{f(cy + h)} H{f(cx + h)} "
            f"V{f(cy + arm)} H{f(cx - h)} V{f(cy + h)} H{f(cx - arm)} V{f(cy - h)} H{f(cx - h)} Z")


def svg(title, inner):
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" width="256" height="256">'
            f"<title>{title}</title>{inner}</svg>\n")


def ellipse_arc(cx, cy, rx, ry, rot, a0, a1, steps=1):
    """Arc endpoints of a rotated ellipse from angle a0 to a1 (degrees)."""
    def pt(a):
        t, r = math.radians(a), math.radians(rot)
        x, y = rx * math.cos(t), ry * math.sin(t)
        return cx + x * math.cos(r) - y * math.sin(r), cy + x * math.sin(r) + y * math.cos(r)
    (x0, y0), (x1, y1) = pt(a0), pt(a1)
    large = 1 if (a1 - a0) % 360 > 180 else 0
    return f"M{f(x0)} {f(y0)} A{rx} {ry} {rot} {large} 1 {f(x1)} {f(y1)}", (x1, y1)


# A: ring. The back of the orbit hides behind the pad, the front crosses its lower half with a gap.
ring_c, ring = (128, 158), (122, 22, -8)
back, _ = ellipse_arc(*ring_c, *ring, 180, 360)
front, _ = ellipse_arc(*ring_c, *ring, 0, 180)
gap, _ = ellipse_arc(*ring_c, *ring, 12, 168)
a = svg("OrbitStation concept A", (
    f'<path d="{back}" fill="none" stroke="#000" stroke-width="11" stroke-linecap="round"/>'
    f'<path d="{pad(-6)}" fill="#000" fill-rule="evenodd"/>'
    f'<path d="{gap}" fill="none" stroke="#fff" stroke-width="27" stroke-linecap="butt"/>'
    f'<path d="{front}" fill="none" stroke="#000" stroke-width="11" stroke-linecap="round"/>'))

# B: moon stick. The right stick is a planet with an orbit and a moon on it.
b = svg("OrbitStation concept B", (
    f'<path d="{pad(holes=False) + plus(88, 108, 17, 12) + circle(168, 108, 26)}" fill="#000" fill-rule="evenodd"/>'
    f'<circle cx="168" cy="108" r="11" fill="#000"/>'
    f'<circle cx="186.4" cy="89.6" r="7" fill="#000"/>'))

# C: station. The pad sits inside an orbit ring; a moon rides the ring at the top right.
moon = (128 + 108 * 0.7071, 128 - 108 * 0.7071)
c = svg("OrbitStation concept C", (
    f'<circle cx="128" cy="128" r="108" fill="none" stroke="#000" stroke-width="12"/>'
    f'<path d="{pad(-4, 0.74)}" fill="#000" fill-rule="evenodd"/>'
    f'<circle cx="{f(moon[0])}" cy="{f(moon[1])}" r="26" fill="#fff"/>'
    f'<circle cx="{f(moon[0])}" cy="{f(moon[1])}" r="16" fill="#000"/>'))

for name, data in (("concept-a.svg", a), ("concept-b.svg", b), ("concept-c.svg", c)):
    open(f"{OUT}/{name}", "w").write(data)
print("ok")

# Final C, colour: a space disc, the orbit ring, the pad, a warm moon riding the ring with a gap around it.
NAVY, RING, PAD, MOON = "#1B2140", "#7C95FF", "#F5F7FF", "#FFA64D"


def station(ring=True, k=0.7, dy=-4, moon_r=15, gap_r=24, ring_r=98, ring_w=11):
    a = 0.7071
    m = (128 + ring_r * a, 128 - ring_r * a)
    out = f'<circle cx="128" cy="128" r="120" fill="{NAVY}"/>'
    if ring:
        h = ring_w / 2
        out += f'<path d="{circle(128, 128, ring_r + h) + circle(128, 128, ring_r - h)}" fill="{RING}" fill-rule="evenodd"/>'
        out += f'<circle cx="{f(m[0])}" cy="{f(m[1])}" r="{gap_r}" fill="{NAVY}"/>'
    else:
        m = (128 + 92 * a, 128 - 92 * a)
    out += f'<path d="{pad(dy, k)}" fill="{PAD}" fill-rule="evenodd"/>'
    out += f'<circle cx="{f(m[0])}" cy="{f(m[1])}" r="{moon_r}" fill="{MOON}"/>'
    return out


open(f"{OUT}/orbitstation.svg", "w").write(svg("OrbitStation", station()))
# 16-24 px cut: no ring, a bigger pad, a bigger moon, so nothing thinner than a pixel survives
open(f"{OUT}/orbitstation-small.svg", "w").write(svg("OrbitStation", station(ring=False, k=0.86, dy=6, moon_r=18)))

# One colour, no tile, for README headers and anything that is not an app icon. The ring is an open
# annulus with a gap around the moon, so the colour and mono versions share one drawing.
def ring_gap(r, w, at=-45, gap=15):
    def pt(rad, deg):
        t = math.radians(deg)
        return f"{f(128 + rad * math.cos(t))} {f(128 + rad * math.sin(t))}"
    ro, ri, a0, a1 = r + w / 2, r - w / 2, at + gap, at + 360 - gap
    return (f"M{pt(ro, a0)} A{f(ro)} {f(ro)} 0 1 1 {pt(ro, a1)} L{pt(ri, a1)} "
            f"A{f(ri)} {f(ri)} 0 1 0 {pt(ri, a0)} Z")


r = 108
m = (128 + r * 0.7071, 128 - r * 0.7071)
open(f"{OUT}/orbitstation-mono.svg", "w").write(svg("OrbitStation", (
    f'<path d="{ring_gap(r, 12)}" fill="currentColor"/>'
    f'<path d="{pad(-4, 0.74)}" fill="currentColor" fill-rule="evenodd"/>'
    f'<circle cx="{f(m[0])}" cy="{f(m[1])}" r="16" fill="currentColor"/>')))
