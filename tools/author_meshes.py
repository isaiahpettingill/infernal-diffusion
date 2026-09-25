"""Rebuild the checked-in low-poly mesh recipes. Not used by monster generation."""

import json
import math
from pathlib import Path


class Mesh:
    def __init__(self):
        self.vertices = []
        self.faces = []

    def vertex(self, point):
        self.vertices.append([round(float(v), 5) for v in point])
        return len(self.vertices) - 1

    def face(self, a, b, c, material):
        self.faces.append([a, b, c, material])

    def ellipsoid(self, center, radius, material, rings=5, sides=10):
        start = len(self.vertices)
        for i in range(rings + 1):
            latitude = -math.pi / 2 + math.pi * i / rings
            for j in range(sides):
                longitude = 2 * math.pi * j / sides
                self.vertex((
                    center[0] + radius[0] * math.cos(latitude) * math.cos(longitude),
                    center[1] + radius[1] * math.sin(latitude),
                    center[2] + radius[2] * math.cos(latitude) * math.sin(longitude),
                ))
        for i in range(rings):
            for j in range(sides):
                a = start + i * sides + j
                b = start + i * sides + (j + 1) % sides
                c = start + (i + 1) * sides + j
                d = start + (i + 1) * sides + (j + 1) % sides
                self.face(a, b, c, material)
                self.face(b, d, c, material)

    def loft(self, sections, material, sides=10):
        """Sections are (x, y_center, z_center, y_radius, z_radius)."""
        start = len(self.vertices)
        for x, y, z, ry, rz in sections:
            for j in range(sides):
                angle = 2 * math.pi * j / sides
                self.vertex((x, y + ry * math.cos(angle), z + rz * math.sin(angle)))
        for i in range(len(sections) - 1):
            for j in range(sides):
                a = start + i * sides + j
                b = start + i * sides + (j + 1) % sides
                c = start + (i + 1) * sides + j
                d = start + (i + 1) * sides + (j + 1) % sides
                self.face(a, b, c, material)
                self.face(b, d, c, material)
        for j in range(1, sides - 1):
            self.face(start, start + j + 1, start + j, material)
            end = start + (len(sections) - 1) * sides
            self.face(end, end + j, end + j + 1, material)

    def save(self, path):
        path.write_text(json.dumps({"vertices": self.vertices, "faces": self.faces}, separators=(",", ":")))


out = Path(__file__).resolve().parents[1] / "assets" / "meshes"
out.mkdir(parents=True, exist_ok=True)


def eyes(mesh, x, y, z, radius=0.2):
    for side in (-1, 1):
        mesh.ellipsoid((x, y, side * z), (radius * 1.2, radius, radius * 0.45), "socket")
        mesh.ellipsoid((x + 0.035, y, side * (z + 0.035)), (radius * 0.34, radius * 0.45, radius * 0.25), "eye")


def teeth(mesh, x_start, count, span, y, z):
    for i in range(count):
        x = x_start + span * i / max(count - 1, 1)
        for side in (-1, 1):
            mesh.loft([(x, y, side * z, 0.08, 0.08), (x + 0.03, y - 0.22, side * z, 0.01, 0.01)], "tooth", 6)


def canid():
    m = Mesh()
    m.ellipsoid((-0.28, 0.12, 0), (0.79, 0.75, 0.69), "base")
    m.loft([(0.08, -0.17, 0, 0.35, 0.48), (0.7, -0.24, 0, 0.29, 0.39), (1.48, -0.29, 0, 0.2, 0.26)], "base")
    m.loft([(0.2, -0.52, 0, 0.10, 0.43), (1.25, -0.51, 0, 0.08, 0.3)], "jaw")
    m.ellipsoid((1.48, -0.23, 0), (0.20, 0.15, 0.3), "nose")
    eyes(m, 0.17, 0.22, 0.61, 0.20)
    teeth(m, 0.54, 4, 0.63, -0.39, 0.29)
    for side in (-1, 1):
        m.loft([(-0.67, 0.46, side * 0.43, 0.28, 0.25), (-0.54, 1.19, side * 0.49, 0.02, 0.025)], "ear", 7)
    return m


def reptile():
    m = Mesh()
    m.ellipsoid((-0.25, 0.0, 0), (0.83, 0.48, 0.87), "base")
    m.loft([(0.1, -0.16, 0, 0.3, 0.73), (0.88, -0.16, 0, 0.24, 0.61), (1.9, -0.17, 0, 0.19, 0.47)], "base")
    m.loft([(0.05, -0.46, 0, 0.12, 0.7), (1.72, -0.48, 0, 0.09, 0.44)], "jaw")
    for side in (-1, 1):
        m.ellipsoid((0.23, 0.32, side * 0.61), (0.42, 0.17, 0.29), "brow")
        m.ellipsoid((1.64, -0.02, side * 0.35), (0.15, 0.075, 0.11), "nose")
    eyes(m, 0.25, 0.27, 0.77, 0.18)
    teeth(m, 0.58, 7, 1.1, -0.33, 0.49)
    return m


def worm_maw():
    m = Mesh()
    m.loft([(-0.9, 0, 0, 0.47, 0.47), (-0.25, 0, 0, 0.72, 0.72),
            (0.52, 0, 0, 0.75, 0.75), (0.82, 0, 0, 0.58, 0.58)], "base", 16)
    m.ellipsoid((0.83, 0, 0), (0.04, 0.47, 0.47), "mouth", 6, 16)
    for i in range(12):
        angle = 2 * math.pi * i / 12
        y, z = math.cos(angle) * 0.53, math.sin(angle) * 0.53
        m.loft([(0.83, y, z, 0.10, 0.10),
                (1.19, y * 0.72, z * 0.72, 0.015, 0.015)], "tooth", 6)
    return m


def feline():
    m = Mesh()
    m.ellipsoid((-0.2, 0.1, 0), (0.89, 0.77, 0.78), "base")
    for side in (-1, 1):
        m.ellipsoid((0.55, -0.31, side * 0.29), (0.39, 0.29, 0.35), "muzzle")
        m.loft([(-0.66, 0.53, side * 0.51, 0.29, 0.27), (-0.53, 1.13, side * 0.57, 0.02, 0.02)], "ear", 7)
    m.ellipsoid((0.88, -0.22, 0), (0.17, 0.11, 0.24), "nose")
    m.ellipsoid((0.45, -0.58, 0), (0.47, 0.15, 0.48), "jaw")
    eyes(m, 0.32, 0.21, 0.69, 0.19)
    teeth(m, 0.47, 2, 0.37, -0.47, 0.34)
    return m


def bovine():
    m = Mesh()
    m.ellipsoid((-0.25, 0.1, 0), (0.82, 0.77, 0.72), "base")
    m.loft([(0.1, -0.22, 0, 0.4, 0.5), (0.9, -0.4, 0, 0.34, 0.52), (1.38, -0.38, 0, 0.25, 0.49)], "muzzle")
    m.ellipsoid((1.31, -0.31, 0), (0.16, 0.19, 0.48), "nose")
    for side in (-1, 1):
        m.ellipsoid((1.39, -0.27, side * 0.31), (0.06, 0.07, 0.07), "socket")
        m.loft([(-0.53, 0.47, side * 0.54, 0.23, 0.21), (-0.67, 1.0, side * 1.03, 0.16, 0.15), (-0.85, 1.36, side * 1.42, 0.015, 0.015)], "horn", 8)
        m.ellipsoid((-0.72, 0.4, side * 0.7), (0.26, 0.13, 0.33), "ear")
    eyes(m, 0.18, 0.29, 0.62, 0.19)
    return m


def humanoid():
    m = Mesh()
    m.ellipsoid((-0.2, 0.28, 0), (0.73, 0.91, 0.72), "base")
    m.loft([(-0.45, -0.28, 0, 0.31, 0.53), (0.31, -0.46, 0, 0.26, 0.41)], "jaw")
    for side in (-1, 1):
        m.ellipsoid((0.45, 0.22, side * 0.4), (0.13, 0.23, 0.23), "socket")
        m.ellipsoid((0.52, 0.19, side * 0.42), (0.05, 0.08, 0.11), "eye")
        m.ellipsoid((0.12, -0.16, side * 0.61), (0.28, 0.18, 0.16), "brow")
    m.loft([(0.51, 0.04, 0, 0.23, 0.19), (0.79, -0.22, 0, 0.08, 0.1)], "nose", 7)
    m.ellipsoid((0.48, -0.46, 0), (0.06, 0.08, 0.26), "mouth")
    return m


def cyclops():
    m = Mesh()
    m.ellipsoid((-0.2, 0.18, 0), (0.79, 0.93, 0.75), "base")
    m.loft([(-0.4, -0.33, 0, 0.29, 0.5), (0.34, -0.45, 0, 0.23, 0.4)], "jaw")
    m.ellipsoid((0.49, 0.3, 0), (0.25, 0.4, 0.72), "socket")
    m.ellipsoid((0.73, 0.3, 0), (0.12, 0.23, 0.51), "eye")
    m.ellipsoid((0.75, 0.37, 0.23), (0.05, 0.08, 0.12), "tooth")
    m.loft([(0.45, -0.06, 0, 0.22, 0.21), (0.74, -0.27, 0, 0.07, 0.1)], "nose", 7)
    m.ellipsoid((0.52, -0.52, 0), (0.07, 0.06, 0.29), "mouth")
    return m


def arthropod():
    m = Mesh()
    m.ellipsoid((0, 0, 0), (1.03, 0.58, 0.93), "base")
    m.ellipsoid((-0.09, 0.32, 0), (0.93, 0.29, 0.88), "brow")
    for side in (-1, 1):
        m.loft([(0.55, -0.1, side * 0.44, 0.19, 0.19), (1.15, -0.63, side * 0.56, 0.03, 0.04)], "mandible", 7)
        for x, y in [(0.53, 0.3), (0.77, 0.13), (0.51, -0.03)]:
            m.ellipsoid((x, y, side * 0.69), (0.14, 0.11, 0.09), "eye")
    return m


def avian():
    m = Mesh()
    m.ellipsoid((-0.22, 0.1, 0), (0.75, 0.74, 0.64), "base")
    m.loft([(0.33, -0.12, 0, 0.29, 0.39), (1.32, -0.31, 0, 0.02, 0.05)], "beak")
    m.loft([(0.31, -0.38, 0, 0.08, 0.37), (1.05, -0.41, 0, 0.02, 0.08)], "jaw")
    eyes(m, 0.25, 0.24, 0.55, 0.17)
    return m


def generic():
    m = Mesh()
    m.ellipsoid((-0.13, 0.06, 0), (0.84, 0.78, 0.76), "base")
    m.loft([(0.31, -0.29, 0, 0.33, 0.47), (1.06, -0.36, 0, 0.13, 0.25)], "muzzle")
    m.ellipsoid((0.34, -0.58, 0), (0.42, 0.15, 0.43), "jaw")
    eyes(m, 0.31, 0.18, 0.65, 0.18)
    return m


def bear():
    m = Mesh()
    m.ellipsoid((-0.26, 0.07, 0), (0.98, 0.83, 0.88), "base")
    m.ellipsoid((0.63, -0.29, 0), (0.67, 0.37, 0.55), "muzzle")
    m.ellipsoid((1.18, -0.27, 0), (0.22, 0.17, 0.35), "nose")
    for side in (-1, 1):
        m.ellipsoid((-0.61, 0.75, side * 0.62), (0.32, 0.34, 0.22), "ear")
    eyes(m, 0.34, 0.23, 0.72, 0.16)
    return m


def moose():
    m = Mesh()
    m.ellipsoid((-0.28, 0.13, 0), (0.74, 0.87, 0.67), "base")
    m.loft([(0.05, -0.23, 0, 0.39, 0.5), (0.91, -0.49, 0, 0.33, 0.47), (1.61, -0.68, 0, 0.23, 0.39)], "muzzle")
    m.ellipsoid((1.53, -0.61, 0), (0.22, 0.13, 0.4), "nose")
    for side in (-1, 1):
        m.loft([(-0.49, 0.68, side * 0.5, 0.16, 0.18), (-0.58, 1.06, side * 1.02, 0.25, 0.34), (-0.78, 1.62, side * 1.34, 0.35, 0.1)], "horn", 8)
        m.ellipsoid((-0.56, 0.51, side * 0.75), (0.27, 0.14, 0.35), "ear")
    eyes(m, 0.31, 0.27, 0.58, 0.16)
    return m


def rhino():
    m = Mesh()
    m.ellipsoid((-0.24, 0.06, 0), (0.98, 0.78, 0.84), "base")
    m.loft([(0.22, -0.2, 0, 0.43, 0.59), (1.11, -0.35, 0, 0.35, 0.49), (1.47, -0.37, 0, 0.26, 0.43)], "muzzle")
    m.loft([(0.91, 0.05, 0, 0.32, 0.38), (1.23, 0.62, 0, 0.22, 0.21), (1.55, 1.19, 0, 0.01, 0.01)], "horn", 9)
    for side in (-1, 1):
        m.ellipsoid((-0.6, 0.56, side * 0.69), (0.25, 0.29, 0.15), "ear")
    eyes(m, 0.33, 0.23, 0.72, 0.15)
    return m


def elephant():
    m = Mesh()
    m.ellipsoid((-0.28, 0.13, 0), (0.91, 0.89, 0.78), "base")
    for side in (-1, 1):
        m.ellipsoid((-0.55, 0.03, side * 0.83), (0.26, 0.67, 0.53), "ear")
        m.loft([(0.56, -0.43, side * 0.46, 0.14, 0.13), (1.24, -0.64, side * 0.55, 0.01, 0.01)], "tooth", 7)
    m.loft([(0.52, -0.1, 0, 0.3, 0.29), (1.22, -0.47, 0, 0.21, 0.2), (1.23, -1.25, 0, 0.13, 0.13)], "muzzle", 9)
    eyes(m, 0.27, 0.31, 0.71, 0.16)
    return m


def alien():
    m = Mesh()
    m.ellipsoid((-0.35, 0.23, 0), (1.08, 1.05, 0.86), "base")
    m.ellipsoid((0.23, -0.56, 0), (0.41, 0.26, 0.43), "jaw")
    for side in (-1, 1):
        m.ellipsoid((0.47, 0.04, side * 0.54), (0.18, 0.43, 0.27), "socket")
        m.ellipsoid((0.55, 0.04, side * 0.6), (0.09, 0.3, 0.17), "eye")
    return m


def goblin():
    m = humanoid()
    for side in (-1, 1):
        m.loft([(-0.48, 0.06, side * 0.68, 0.22, 0.18), (-0.68, 0.3, side * 1.34, 0.01, 0.02)], "ear", 7)
        m.loft([(0.48, -0.43, side * 0.34, 0.13, 0.1), (0.58, -0.08, side * 0.37, 0.01, 0.01)], "tooth", 7)
    m.ellipsoid((0.74, -0.19, 0), (0.28, 0.2, 0.25), "nose")
    return m


def robot():
    m = Mesh()
    m.ellipsoid((-0.14, 0.08, 0), (0.85, 0.76, 0.77), "metal", rings=3, sides=8)
    m.ellipsoid((0.56, 0.1, 0), (0.18, 0.47, 0.68), "socket", rings=3, sides=8)
    for side in (-1, 1):
        m.ellipsoid((0.67, 0.22, side * 0.38), (0.09, 0.14, 0.17), "eye")
        m.ellipsoid((-0.89, -0.03, side * 0.3), (0.18, 0.27, 0.21), "metal", rings=3, sides=8)
    m.loft([(0.62, -0.43, 0, 0.13, 0.37), (0.68, -0.49, 0, 0.11, 0.34)], "jaw", 8)
    return m


def mushroom():
    m = Mesh()
    m.ellipsoid((0, 0.15, 0), (1.2, 0.42, 1.1), "cap")
    m.ellipsoid((0, -0.27, 0), (1.05, 0.18, 0.95), "gill")
    for x, z in [(-0.57, -0.28), (-0.18, 0.42), (0.23, -0.5), (0.63, 0.19)]:
        m.ellipsoid((x, 0.49, z), (0.14, 0.07, 0.14), "spot", rings=3, sides=7)
    return m


def clown():
    m = humanoid()
    m.ellipsoid((0.81, -0.18, 0), (0.19, 0.19, 0.23), "red")
    for side in (-1, 1):
        m.ellipsoid((-0.5, 0.6, side * 0.56), (0.31, 0.35, 0.29), "hair")
        m.ellipsoid((0.52, 0.23, side * 0.41), (0.14, 0.09, 0.2), "makeup")
    m.ellipsoid((0.52, -0.52, 0), (0.1, 0.08, 0.37), "red")
    return m


def anime():
    m = Mesh()
    m.ellipsoid((-0.19, 0.13, 0), (0.82, 0.94, 0.72), "base")
    m.ellipsoid((-0.26, 0.88, 0), (0.88, 0.21, 0.77), "hair")
    for side in (-1, 1):
        m.ellipsoid((0.46, 0.21, side * 0.38), (0.14, 0.29, 0.24), "eye")
        m.loft([(-0.68, 0.68, side * 0.48, 0.29, 0.21), (-0.8, -0.64, side * 0.54, 0.18, 0.18)], "hair", 7)
    m.ellipsoid((0.66, -0.41, 0), (0.06, 0.05, 0.17), "mouth")
    return m


def historical_moustache():
    m = humanoid()
    m.ellipsoid((-0.25, 0.92, 0), (0.77, 0.16, 0.69), "hair")
    m.ellipsoid((0.77, -0.31, 0), (0.08, 0.11, 0.23), "moustache")
    return m


def tank_turret():
    m = Mesh()
    m.ellipsoid((-0.21, 0.05, 0), (0.95, 0.54, 0.84), "metal", rings=3, sides=8)
    m.loft([(0.51, -0.06, 0, 0.22, 0.22), (1.67, -0.04, 0, 0.17, 0.17)], "metal", 9)
    m.ellipsoid((1.7, -0.04, 0), (0.11, 0.24, 0.24), "socket")
    return m


def tank_hull():
    m = Mesh()
    m.ellipsoid((0, 0, 0), (1.0, 0.57, 0.85), "metal", rings=3, sides=8)
    m.ellipsoid((-0.18, 0.43, 0), (0.58, 0.14, 0.58), "metal", rings=3, sides=8)
    return m


def tank_tread():
    m = Mesh()
    m.ellipsoid((0, 0, 0), (1.12, 0.36, 0.47), "tread", rings=4, sides=10)
    for x in (-0.8, -0.4, 0, 0.4, 0.8):
        m.ellipsoid((x, -0.28, 0.1), (0.08, 0.1, 0.4), "metal", rings=3, sides=7)
    return m


def rotor():
    m = Mesh()
    m.ellipsoid((0, 0, 0), (0.3, 0.13, 0.3), "metal")
    m.ellipsoid((0.0, 0.08, 0), (1.13, 0.045, 0.13), "metal", rings=3, sides=8)
    return m


def chainsaw():
    m = Mesh()
    m.loft([(-1.1, 0, 0, 0.14, 0.18), (-0.31, 0, 0, 0.15, 0.18)], "wood", 8)
    m.ellipsoid((0.08, 0, 0), (0.55, 0.35, 0.35), "metal", rings=4, sides=8)
    m.ellipsoid((1.12, 0, 0), (1.09, 0.19, 0.2), "metal", rings=3, sides=9)
    for x in (0.47, 0.83, 1.19, 1.55, 1.91):
        for sign in (-1, 1):
            m.loft([(x, sign * 0.11, 0, 0.09, 0.08), (x + 0.11, sign * 0.33, 0, 0.01, 0.01)], "tooth", 6)
    return m


def horn():
    m = Mesh()
    m.loft([(0, 0, 0, 0.28, 0.28), (0.13, 0.37, 0, 0.22, 0.21), (0.31, 0.82, 0.05, 0.13, 0.12), (0.54, 1.31, 0.11, 0.012, 0.012)], "horn", 8)
    return m


def claw():
    m = Mesh()
    m.loft([(0, 0, 0, 0.23, 0.2), (0.4, -0.08, 0, 0.19, 0.17), (0.83, -0.27, 0, 0.11, 0.1), (1.1, -0.59, 0, 0.01, 0.01)], "horn", 8)
    return m


def fang():
    m = Mesh()
    m.loft([(0, 0, 0, 0.17, 0.18), (0.06, -0.43, 0, 0.12, 0.12), (0.18, -0.98, 0, 0.01, 0.01)], "tooth", 8)
    return m


def wing():
    m = Mesh()
    m.loft([(0, 0, 0, 0.1, 0.11), (-0.72, 0.93, 0.05, 0.1, 0.1), (-1.45, 1.48, 0.02, 0.06, 0.07)], "rib", 6)
    for z in (-0.05, 0.05):
        a = m.vertex((0, 0, z))
        b = m.vertex((-0.7, 0.95, z))
        c = m.vertex((-1.68, 1.25, z))
        d = m.vertex((-1.25, -0.16, z))
        m.face(a, b, d, "membrane")
        m.face(b, c, d, "membrane")
    return m


def feather_wing():
    m = Mesh()
    m.loft([(0.15, -0.2, 0, 0.12, 0.13), (-0.62, 0.62, 0, 0.12, 0.11), (-1.42, 1.08, 0, 0.06, 0.07)], "rib", 7)
    for i in range(6):
        x = -0.14 - i * 0.25
        y = 0.03 + i * 0.17
        m.ellipsoid((x - 0.33, y - 0.2, 0), (0.48, 0.22, 0.09), "base", rings=4, sides=8)
    return m


def shell():
    m = Mesh()
    m.ellipsoid((0, 0.21, 0), (1.1, 0.55, 0.93), "shell")
    for x in (-0.46, 0, 0.46):
        m.loft([(x, 0.39, -0.84, 0.035, 0.04), (x, 0.67, 0, 0.04, 0.04), (x, 0.39, 0.84, 0.035, 0.04)], "ridge", 6)
    return m


def hoof():
    m = Mesh()
    m.loft([(-0.42, 0.22, 0, 0.21, 0.36), (-0.23, -0.22, 0, 0.16, 0.4), (0.48, -0.27, 0, 0.12, 0.4)], "hoof")
    return m


def weapon():
    m = Mesh()
    m.loft([(-0.8, 0, 0, 0.2, 0.22), (0.75, 0, 0, 0.2, 0.22)], "metal", 8)
    m.loft([(0.65, 0.0, 0, 0.24, 0.26), (1.3, 0.0, 0, 0.24, 0.26)], "metal", 8)
    m.loft([(-1.24, -0.09, 0, 0.32, 0.24), (-0.72, -0.06, 0, 0.23, 0.2)], "wood", 8)
    m.loft([(-0.58, -0.07, 0, 0.11, 0.15), (-0.5, -0.69, 0, 0.12, 0.13)], "wood", 7)
    m.ellipsoid((1.25, 0, 0), (0.06, 0.16, 0.17), "socket")
    return m


def sword():
    m = Mesh()
    m.loft([(-1.1, 0, 0, 0.12, 0.13), (-0.25, 0, 0, 0.12, 0.13)], "wood", 8)
    m.ellipsoid((-0.18, 0, 0), (0.12, 0.18, 0.58), "metal")
    m.loft([(0, 0, 0, 0.2, 0.26), (1.35, 0, 0, 0.12, 0.17), (1.9, 0, 0, 0.01, 0.01)], "metal", 8)
    return m


def club():
    m = Mesh()
    m.loft([(-1.15, 0, 0, 0.12, 0.12), (1.1, 0, 0, 0.19, 0.19)], "wood", 8)
    m.ellipsoid((1.0, 0, 0), (0.65, 0.42, 0.44), "wood")
    for y in (-0.24, 0.24):
        m.loft([(1.1, y, 0, 0.13, 0.13), (1.5, y * 1.6, 0, 0.01, 0.01)], "horn", 6)
    return m


def axe():
    m = Mesh()
    m.loft([(-1.2, 0, 0, 0.11, 0.11), (1.1, 0, 0, 0.15, 0.15)], "wood", 8)
    m.ellipsoid((0.9, 0.1, 0), (0.45, 0.62, 0.24), "metal")
    m.loft([(0.91, 0.0, 0, 0.55, 0.23), (1.72, 0.12, 0, 0.9, 0.035)], "metal", 8)
    return m


def spear():
    m = Mesh()
    m.loft([(-1.65, 0, 0, 0.1, 0.1), (1.05, 0, 0, 0.1, 0.1)], "wood", 8)
    m.loft([(0.92, 0, 0, 0.23, 0.2), (1.5, 0, 0, 0.17, 0.13), (2.15, 0, 0, 0.01, 0.01)], "metal", 8)
    return m


def bow():
    m = Mesh()
    m.loft([(-0.55, -0.85, 0, 0.08, 0.1), (0.2, -0.34, 0, 0.1, 0.1), (0.38, 0, 0, 0.1, 0.11), (0.2, 0.34, 0, 0.1, 0.1), (-0.55, 0.85, 0, 0.08, 0.1)], "wood", 6)
    m.loft([(-0.55, -0.85, 0, 0.02, 0.02), (-0.63, 0, 0, 0.02, 0.02), (-0.55, 0.85, 0, 0.02, 0.02)], "metal", 5)
    m.loft([(-0.55, 0, 0, 0.035, 0.04), (0.75, 0, 0, 0.035, 0.04), (1.05, 0, 0, 0.01, 0.01)], "wood", 6)
    return m


def paw():
    m = Mesh()
    m.ellipsoid((-0.1, 0, 0), (0.53, 0.24, 0.42), "base")
    for z in (-0.29, -0.09, 0.11, 0.31):
        m.ellipsoid((0.37, -0.04, z), (0.21, 0.17, 0.105), "base")
        m.loft([(0.51, -0.05, z, 0.09, 0.07), (0.79, -0.18, z, 0.01, 0.01)], "horn", 6)
    return m


for name, builder in {
    "skull_canid": canid,
    "skull_reptile": reptile,
    "skull_worm": worm_maw,
    "skull_feline": feline,
    "skull_bovine": bovine,
    "skull_humanoid": humanoid,
    "skull_cyclops": cyclops,
    "skull_arthropod": arthropod,
    "skull_avian": avian,
    "skull_generic": generic,
    "skull_bear": bear,
    "skull_moose": moose,
    "skull_rhino": rhino,
    "skull_elephant": elephant,
    "skull_alien": alien,
    "skull_goblin": goblin,
    "skull_robot": robot,
    "skull_mushroom": mushroom,
    "skull_clown": clown,
    "skull_anime": anime,
    "skull_hitler": historical_moustache,
    "skull_tank": tank_turret,
    "part_horn": horn,
    "part_claw": claw,
    "part_fang": fang,
    "part_wing": wing,
    "part_wing_feather": feather_wing,
    "part_shell": shell,
    "part_hoof": hoof,
    "part_weapon": weapon,
    "part_weapon_sword": sword,
    "part_weapon_club": club,
    "part_weapon_axe": axe,
    "part_weapon_spear": spear,
    "part_weapon_bow": bow,
    "part_weapon_chainsaw": chainsaw,
    "part_paw": paw,
    "part_tread": tank_tread,
    "part_tank_hull": tank_hull,
    "part_rotor": rotor,
}.items():
    builder().save(out / f"{name}.json")
