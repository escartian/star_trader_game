"""Original interface icon geometry, shared by the Blender icon build script.

Each icon is drawn on a 20-unit grid centered at (0, 0) with y pointing down
(SVG convention). A drawing is a list of elements:
  ("stroke", points, closed)       1.4-unit line
  ("fill", points, opacity)        filled polygon
Circles and arcs are sampled into polylines so every element is a plain path.
"""
import math

from icon_shapes_world import WORLD_ICONS

STROKE = 1.4


class Pen:
    def __init__(self, offset=(0.0, 0.0), factor=1.0, out=None):
        self.offset, self.factor = offset, factor
        self.out = [] if out is None else out

    def p(self, point):
        return (self.offset[0] + point[0] * self.factor,
                self.offset[1] + point[1] * self.factor)

    def sub(self, offset, factor):
        return Pen(self.p(offset), self.factor * factor, self.out)

    def path(self, points, closed=False):
        self.out.append(("stroke", [self.p(q) for q in points], closed))

    def line(self, a, b):
        self.path([a, b])

    def closed(self, points):
        self.path(points, True)

    def fill(self, points, opacity=1.0):
        self.out.append(("fill", [self.p(q) for q in points], opacity))

    def arc_points(self, center, radii, start, end, steps=24):
        return [(center[0] + radii[0] * math.cos(start + (end - start) * i / steps),
                 center[1] + radii[1] * math.sin(start + (end - start) * i / steps))
                for i in range(steps + 1)]

    def arc(self, center, radii, start, end):
        self.path(self.arc_points(center, radii, start, end))

    def circle(self, center, radius):
        self.closed(self.arc_points(center, (radius, radius), 0, math.tau, 32)[:-1])

    def dot(self, center, radius):
        self.fill(self.arc_points(center, (radius, radius), 0, math.tau, 16)[:-1])

    def rect(self, a, b):
        self.closed([a, (b[0], a[1]), b, (a[0], b[1])])

    def arrow_head(self, tip, dx, dy):
        """`dx, dy` point from the tip back along the shaft."""
        nx, ny = -dy, dx
        self.path([(tip[0] + 4 * dx + 3 * nx, tip[1] + 4 * dy + 3 * ny), tip,
                   (tip[0] + 4 * dx - 3 * nx, tip[1] + 4 * dy - 3 * ny)])

    def rays(self, inner, outer, count=8):
        for step in range(count):
            a = step * math.tau / count
            self.line((inner * math.cos(a), inner * math.sin(a)),
                      (outer * math.cos(a), outer * math.sin(a)))


HULL = [(0, -8), (6, 7), (0, 4), (-6, 7)]
DROP = [(0, -8), (4, -1.5), (5, 2), (4, 5), (1, 7), (-1, 7), (-4, 5), (-5, 2), (-4, -1.5)]
TRAY = [(-7, 2), (-7, 7), (7, 7), (7, 2)]


def i_fleets(p):
    for x in (-4.5, 4.5):
        p.sub((x, 0), 0.7).closed(HULL)


def i_shipyard(p):
    p.sub((0, -1), 0.7).closed(HULL)
    p.path([(-8, -6), (-8, 8), (8, 8), (8, -6)])


def i_save(p):
    p.rect((-7, -7), (7, 7))
    p.rect((-3, -7), (3, -2))
    p.line((-4, 4), (4, 4))


def i_load(p):
    p.closed([(-8, -6), (-3, -6), (-1, -4), (8, -4), (8, 6), (-8, 6)])
    p.line((-8, -1), (8, -1))


def i_new(p):
    p.circle((0, 0), 7.5)
    p.line((0, -4), (0, 4))
    p.line((-4, 0), (4, 0))


def i_navigation(p):
    p.circle((0, 0), 7.5)
    p.closed([(3, -5), (1, 3), (-4, 5), (-1, -2)])


def i_market(p):
    for x in (-5, 0, 5):
        p.line((x, 6), (x, -x - 1))
    p.line((-8, 8), (8, 8))


def i_execute(p):
    for x in (-5, 2):
        p.path([(x, -6), (x + 5, 0), (x, 6)])


def i_back(p):
    p.line((7, 0), (-7, 0))
    p.arrow_head((-7, 0), 1, 0)


def i_focus(p):
    for s in (-1, 1):
        p.path([(s * 8, -3), (s * 8, -8), (s * 3, -8)])
        p.path([(s * 8, 3), (s * 8, 8), (s * 3, 8)])
    p.circle((0, 0), 2)


def i_overview(p):
    p.circle((0, 0), 2.5)
    p.closed(p.arc_points((0, 0), (8, 4), 0, math.tau, 32)[:-1])
    p.dot((6.5, -2.3), 1.3)


def i_credits(p):
    p.circle((0, 0), 7.5)
    p.closed([(0, -4.5), (3, 0), (0, 4.5), (-3, 0)])


def i_funds(p):
    p.closed([(-8, -3), (0, -8), (8, -3)])
    for x in (-5, 0, 5):
        p.line((x, -1), (x, 5))
    p.line((-8, 7), (8, 7))


def i_cargo(p):
    p.rect((-7, -7), (7, 7))
    p.line((-7, -7), (7, 7))
    p.line((7, -7), (-7, 7))


def i_refuel(p):
    p.sub((-2.5, 1), 0.8).closed(DROP)
    p.line((6, -8), (6, -2))
    p.line((3, -5), (9, -5))


def i_clock(p):
    p.circle((0, 0), 7.5)
    p.path([(0, -4.5), (0, 0), (3.5, 2)])


def i_dock(p):
    # Docking clamp: two brackets gripping a hull block.
    p.path([(-4, -7), (-8, -7), (-8, 7), (-4, 7)])
    p.path([(4, -7), (8, -7), (8, 7), (4, 7)])
    p.fill([(-3, -3.5), (3, -3.5), (3, 3.5), (-3, 3.5)])


def i_orbit(p):
    p.dot((0, 0), 3.5)
    p.arc((0, 0), (8.5, 3.5), -0.2, math.pi + 0.2)


def i_pin(p):
    p.circle((0, -3), 4)
    p.path([(-3.5, -1), (0, 8), (3.5, -1)])


def i_deep_space(p):
    p.closed([(0, -8), (2, -2), (8, 0), (2, 2), (0, 8), (-2, 2), (-8, 0), (-2, -2)])


def i_underway(p):
    p.line((-2, 0), (8, 0))
    p.arrow_head((8, 0), -1, 0)
    p.line((-8, -4), (-3, -4))
    p.line((-8, 4), (-3, 4))


def i_buy(p):
    p.line((0, -8), (0, 3))
    p.arrow_head((0, 3), 0, -1)
    p.path(TRAY)


def i_sell(p):
    p.line((0, 3), (0, -8))
    p.arrow_head((0, -8), 0, 1)
    p.path(TRAY)


def i_upgrade(p):
    p.path([(-6, 0), (0, -6), (6, 0)])
    p.path([(-6, 6), (0, 0), (6, 6)])


def i_hold(p):
    p.rect((-5, -6), (-1.5, 6))
    p.rect((1.5, -6), (5, 6))


def i_cancel(p):
    p.line((-6, -6), (6, 6))
    p.line((6, -6), (-6, 6))


def i_plot(p):
    p.line((-5, 8), (-5, -8))
    flag = [(-5, -8), (7, -5), (-5, -2)]
    p.fill(flag, 0.35)
    p.closed(flag)


def i_split(p):
    p.line((0, 8), (0, 1))
    p.line((0, 1), (-6, -6))
    p.line((0, 1), (6, -6))
    p.dot((-6, -6), 1.8)
    p.dot((6, -6), 1.8)


def i_merge(p):
    p.line((-6, 7), (0, 0))
    p.line((6, 7), (0, 0))
    p.line((0, 0), (0, -8))
    p.arrow_head((0, -8), 0, 1)


def i_rename(p):
    p.closed([(-6, 7), (-7, 3), (3, -7), (7, -3), (-3, 7)])
    p.line((1, -5), (5, -1))


def i_transfer(p):
    p.line((-7, -3), (7, -3))
    p.arrow_head((7, -3), -1, 0)
    p.line((7, 3), (-7, 3))
    p.arrow_head((-7, 3), 1, 0)


def i_log(p):
    for y in (-5, 0, 5):
        p.dot((-6.5, y), 1.2)
        p.line((-3, y), (7, y))


def i_reset(p):
    p.arc((0, 0), (7, 7), -math.pi / 2 + 0.5, math.pi * 1.5 - 0.1)
    p.arrow_head((3.4, -6.1), -0.2, 1)


def i_pan(p):
    p.line((-8, 0), (8, 0))
    p.line((0, -8), (0, 8))
    for tip, dx, dy in (((8, 0), -0.7, 0), ((-8, 0), 0.7, 0), ((0, 8), 0, -0.7), ((0, -8), 0, 0.7)):
        p.arrow_head(tip, dx, dy)


def i_star(p):
    p.dot((0, 0), 3.5)
    p.rays(5.5, 8)


def i_warning(p):
    p.closed([(0, -8), (8.5, 7), (-8.5, 7)])
    p.line((0, -3), (0, 2))
    p.dot((0, 4.5), 1.1)


def i_info(p):
    p.circle((0, 0), 7.5)
    p.dot((0, -3.8), 1.2)
    p.line((0, -1), (0, 4.5))


def i_distance(p):
    p.line((-7, 0), (7, 0))
    p.arrow_head((7, 0), -0.8, 0)
    p.arrow_head((-7, 0), 0.8, 0)
    p.line((-8.5, -5), (-8.5, 5))
    p.line((8.5, -5), (8.5, 5))


def i_legs(p):
    nodes = [(-7, 5), (-2, -3), (3, 2), (7, -6)]
    p.path(nodes)
    for node in nodes:
        p.dot(node, 1.6)


def i_bypass(p):
    p.path([(-7, 7), (-7, 1), (-5, -3), (-1, -5), (7, -5)])
    p.arrow_head((7, -5), -1, 0)


def i_contact(p):
    p.circle((0, 0), 7.5)
    p.circle((0, 0), 3.5)
    p.line((0, 0), (5.3, -5.3))
    p.dot((3, 3.5), 1.5)


def i_enter(p):
    p.path([(-1, -7), (7, -7), (7, 7), (-1, 7)])
    p.line((-8, 0), (3, 0))
    p.arrow_head((3, 0), -1, 0)


def i_seed(p):
    p.rect((-7, -7), (7, 7))
    for q in ((-3.5, -3.5), (0, 0), (3.5, 3.5)):
        p.dot(q, 1.4)


def i_help(p):
    p.circle((0, 0), 7.5)
    p.arc((0, -2), (3, 2.8), math.pi, math.tau + 0.6)
    p.line((2.4, -0.4), (0, 1.8))
    p.dot((0, 4.8), 1.1)


def i_danger(p):
    p.closed([(0, -8), (7, -5), (6, 2), (0, 8), (-6, 2), (-7, -5)])


































ICONS = {
    "fleets": i_fleets, "ship": lambda p: p.closed(HULL), "shipyard": i_shipyard,
    "save": i_save, "load": i_load, "new": i_new, "navigation": i_navigation,
    "market": i_market, "execute": i_execute, "back": i_back, "focus": i_focus,
    "overview": i_overview, "credits": i_credits, "funds": i_funds, "cargo": i_cargo,
    "fuel": lambda p: p.closed(DROP), "refuel": i_refuel, "clock": i_clock,
    "dock": i_dock, "orbit": i_orbit, "pin": i_pin, "deep-space": i_deep_space,
    "underway": i_underway, "buy": i_buy, "sell": i_sell, "upgrade": i_upgrade,
    "hold": i_hold, "cancel": i_cancel,
    "confirm": lambda p: p.path([(-7, 0), (-2, 5), (7, -6)]),
    "plot": i_plot, "split": i_split, "merge": i_merge, "rename": i_rename,
    "transfer": i_transfer, "log": i_log, "reset": i_reset, "pan": i_pan,
    "star": i_star,
    "warning": i_warning, "info": i_info, "distance": i_distance, "legs": i_legs,
    "bypass": i_bypass, "contact": i_contact, "enter": i_enter, "seed": i_seed,
    "help": i_help, "danger": i_danger,
    **WORLD_ICONS,
}


def elements(name):
    pen = Pen()
    ICONS[name](pen)
    return pen.out
