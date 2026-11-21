"""Commodity, planet-trait and combat icon geometry (see icon_shapes.py for the grid)."""
import math


def gear(pen):
    pen.circle((0, 0), 2.5)
    pen.circle((0, 0), 5.5)
    pen.rays(5.5, 8)


def sun(pen):
    pen.circle((0, 0), 3.5)
    pen.rays(5.5, 8)


def g_food(p):
    p.line((0, 8), (0, -8))
    for y in (-5, -1, 3):
        p.line((0, y), (-4.5, y - 3))
        p.line((0, y), (4.5, y - 3))


def g_ore(p):
    p.closed([(-7, 2), (-4, -5), (2, -7), (7, -2), (5, 6), (-3, 7)])
    p.path([(-4, -5), (0, 0), (7, -2)])
    p.line((0, 0), (-3, 7))


def g_medicine(p):
    p.closed([(-2, -7), (2, -7), (2, -2), (7, -2), (7, 2), (2, 2), (2, 7), (-2, 7),
              (-2, 2), (-7, 2), (-7, -2), (-2, -2)])


def g_textiles(p):
    p.line((-6, -7), (6, -7))
    p.line((-6, 7), (6, 7))
    p.rect((-4, -7), (4, 7))
    for y in (-3.5, 0, 3.5):
        p.line((-4, y + 1.5), (4, y - 1.5))


def g_electronics(p):
    p.rect((-5, -5), (5, 5))
    p.rect((-2, -2), (2, 2))
    for t in (-3, 0, 3):
        p.line((t, -5), (t, -8))
        p.line((t, 5), (t, 8))
        p.line((-5, t), (-8, t))
        p.line((5, t), (8, t))


def e_agricultural(p):
    # A sprout above ploughed furrows, distinct from the Food wheat stalk.
    p.line((-8, 7), (8, 7))
    p.line((-6, 3.5), (6, 3.5))
    p.line((0, 1), (0, -6))
    p.closed([(0, -3), (-5, -6), (-6, -8), (-2, -7)])
    p.closed([(0, -5), (5, -8), (6.5, -7), (2, -4)])


def e_extraction(p):
    p.line((-6, 7), (3, -2))
    p.arc((3, -2), (7, 7), -2.6, -0.5)


def e_industrial(p):
    p.closed([(-8, 7), (-8, -1), (-3, 2), (-3, -1), (2, 2), (2, -7), (6, -7), (6, 7)])


def b_temperate(p):
    p.closed([(-6, 6), (-6, 0), (-2, -5), (7, -7), (5, 1), (0, 6)])
    p.line((-6, 6), (3, -3))


def b_frozen(p):
    for step in range(3):
        a = step * math.tau / 6
        c, s = math.cos(a), math.sin(a)
        p.line((-8 * c, -8 * s), (8 * c, 8 * s))
        for sign in (-1, 1):
            ex, ey = sign * 5.5 * c, sign * 5.5 * s
            p.line((ex, ey), (ex - sign * 2 * s, ey + sign * 2 * c))
            p.line((ex, ey), (ex + sign * 2 * s, ey - sign * 2 * c))


def b_gas(p):
    for y in (-5, 0, 5):
        p.path([(-8, y), (-4, y - 2), (0, y), (4, y + 2), (8, y)])


def b_ocean(p):
    # A droplet riding over two swells.
    p.closed([(0, -8), (3.5, -2.5), (2.5, 0.5), (0, 1.5), (-2.5, 0.5), (-3.5, -2.5)])
    p.path([(-8, 4), (-5, 2.5), (-2, 4), (1, 5.5), (4, 4), (8, 2.5)])
    p.path([(-8, 7.5), (-5, 6), (-2, 7.5), (1, 9), (4, 7.5), (8, 6)])


def b_jungle(p):
    # A palm: a curved trunk under drooping fronds.
    p.path([(1, 8), (0.5, 3), (-0.5, -1), (0, -4)])
    for tip in ((-8, -1), (-6, -7), (6, -7), (8, -1)):
        mid = ((tip[0]) * 0.5, -6.5 if abs(tip[0]) < 7 else -5)
        p.path([(0, -4), mid, tip])
    p.line((-7, 8), (7, 8))


def b_volcanic(p):
    # A cone with a smoking crater.
    p.closed([(-8, 7), (-2.5, -2), (2.5, -2), (8, 7)])
    p.path([(-2.5, -2), (-1, -0.5), (1, -0.5), (2.5, -2)])
    p.path([(0, -3.5), (-1.5, -5.5), (0.5, -7), (-0.5, -9)])
    p.path([(3, -3.5), (4.5, -5.5), (3.5, -7.5)])


def b_barren(p):
    # A cratered disc.
    p.circle((0, 0), 7.5)
    p.circle((-2.5, -2), 2.2)
    p.circle((3, 2.5), 1.5)
    p.circle((-1.5, 4), 0.9)


def b_metallic(p):
    # A faceted ore crystal with a bright seam.
    p.closed([(-6, 1), (-3, -6), (4, -7), (7, -1), (4, 6), (-3, 7)])
    p.path([(-3, -6), (0, -1), (4, -7)])
    p.path([(-6, 1), (0, -1), (7, -1)])
    p.path([(0, -1), (-3, 7)])
    p.path([(0, -1), (4, 6)])


def c_hull(p):
    # Hull plating: a ship outline crossed by a structural rib.
    p.closed([(0, -8), (6, -2), (6, 7), (-6, 7), (-6, -2)])
    p.line((-6, 2), (6, 2))


def c_shield(p):
    p.closed([(0, -8), (7, -5), (6, 2), (0, 8), (-6, 2), (-7, -5)])
    p.arc((0, 0), (4, 4), math.pi * 1.1, math.pi * 1.9)


def c_armor(p):
    # Overlapping armor plates.
    for y in (-6, -1, 4):
        p.closed([(-7, y), (7, y), (6, y + 4), (-6, y + 4)])


def c_engine(p):
    p.rect((-6, -7), (2, 7))
    for y in (-4, 0, 4):
        p.line((4, y), (8, y))


def c_weapon(p):
    # Crosshair reticle.
    p.circle((0, 0), 6)
    p.dot((0, 0), 1.3)
    for a, b in (((0, -9), (0, -3.5)), ((0, 3.5), (0, 9)), ((-9, 0), (-3.5, 0)), ((3.5, 0), (9, 0))):
        p.line(a, b)


WORLD_ICONS = {
    "settings": gear, "good-food": g_food, "good-ore": g_ore, "good-medicine": g_medicine,
    "good-machinery": gear, "good-textiles": g_textiles,
    "good-electronics": g_electronics,
    "economy-agricultural": e_agricultural, "economy-extraction": e_extraction,
    "economy-industrial": e_industrial,
    "biome-temperate": b_temperate, "biome-arid": sun, "biome-frozen": b_frozen,
    "biome-gas": b_gas, "biome-ocean": b_ocean, "biome-jungle": b_jungle,
    "biome-volcanic": b_volcanic, "biome-barren": b_barren, "biome-metallic": b_metallic,
    "combat-hull": c_hull, "combat-shield": c_shield, "combat-armor": c_armor,
    "combat-engine": c_engine, "combat-weapon": c_weapon,
}
