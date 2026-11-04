"""Örnek çizimi üretir: examples/ornek-flans-braket.dxf

Tamamen sentetiktir (gerçek bir müşteri çizimi değildir). Plaka PLAKA katmanında,
parçalar PARCA katmanında; bazı parçaların yanında adet yazısı vardır.
Çalıştırmak için: python3 examples/uret.py  (ezdxf gerekir)
"""
import math
import ezdxf

doc = ezdxf.new("R2010")
doc.units = ezdxf.units.MM
msp = doc.modelspace()
for name, col in [("PLAKA", 4), ("PARCA", 7), ("YAZI", 3)]:
    doc.layers.add(name, color=col)
P = {"layer": "PARCA"}


def text(s, x, y):
    msp.add_text(s, height=25, dxfattribs={"layer": "YAZI"}).set_placement((x, y))


def circle(x, y, r):
    msp.add_circle((x, y), r, dxfattribs=P)


def poly(pts, closed=True):
    msp.add_lwpolyline(pts, close=closed, dxfattribs=P)


# plaka: 3000 × 1500 (iki adet çizildi)
for i in range(2):
    msp.add_lwpolyline([(0, -2000 - i * 1700), (3000, -2000 - i * 1700), (3000, -500 - i * 1700), (0, -500 - i * 1700)], close=True, dxfattribs={"layer": "PLAKA"})

x = 0
# flanş: Ø400, ortada Ø220 delik, 8 cıvata deliği (4 adet)
circle(x + 200, 200, 200)
circle(x + 200, 200, 110)
for k in range(8):
    a = k * math.pi / 4
    circle(x + 200 + 155 * math.cos(a), 200 + 155 * math.sin(a), 11)
text("FLANS 4 AD", x + 120, 200)
x += 500

# çerçeve: 900 × 600, içinde 700 × 400 boşluk ve köşe yuvarlatmalı (yay) iç kontur (2 adet)
poly([(x, 0), (x + 900, 0), (x + 900, 600), (x, 600)])
msp.add_lwpolyline([(x + 150, 150, 0), (x + 750, 150, 0.4142), (x + 800, 200, 0), (x + 800, 450, 0.4142),
                    (x + 750, 500, 0), (x + 150, 500, 0.4142), (x + 100, 450, 0), (x + 100, 200, 0.4142)],
                   format="xyb", close=True, dxfattribs=P)
text("CERCEVE 2 AD", x + 350, 50)
x += 1000

# L braket, çizgi + yaylardan oluşan kontur (6 adet)
l = [(x, 0), (x + 300, 0), (x + 300, 60), (x + 60, 60), (x + 60, 250), (x, 250), (x, 0)]
for a, b in zip(l, l[1:]):
    msp.add_line(a, b, dxfattribs=P)
circle(x + 30, 200, 9)
circle(x + 250, 30, 9)
text("6 AD", x + 120, 120)
x += 400

# üçgen takviye (10 adet)
poly([(x, 0), (x + 220, 0), (x, 180)])
text("x10", x + 20, 30)
x += 300

# pul: Ø90, Ø40 delik (16 adet)
circle(x + 45, 45, 45)
circle(x + 45, 45, 20)
text("16 ADET", x, 110)
x += 200

# kanal kapağı: uzun, iki ucunda yarım daire (3 adet)
msp.add_lwpolyline([(x, 0, 0), (x + 900, 0, 1), (x + 900, 120, 0), (x, 120, 1)], format="xyb", close=True, dxfattribs=P)
circle(x + 60, 60, 25)
circle(x + 840, 60, 25)
text("3 AD", x + 400, 50)
x += 1100

# bağlantı plakası: dikdörtgen, eliptik delik ve spline kenar (5 adet)
pts = [(x, 0), (x + 350, 0), (x + 350, 180)]
for a, b in zip(pts, pts[1:]):
    msp.add_line(a, b, dxfattribs=P)
msp.add_spline([(x + 350, 180), (x + 250, 230), (x + 120, 160), (x, 200)], dxfattribs=P)
msp.add_line((x, 200), (x, 0), dxfattribs=P)
msp.add_ellipse((x + 175, 90), major_axis=(70, 0), ratio=0.5, dxfattribs=P)
text("5 AD", x + 150, 20)

doc.saveas(__file__.rsplit("/", 1)[0] + "/ornek-flans-braket.dxf")
print("yazıldı")
