#!/usr/bin/env python3
"""Generate original ZnunitAICAD icons with Python 3 and Pillow.

Run: python packaging/icons.py. No fonts or external artwork are used.
Normal application builds consume committed outputs and need no Python.
"""

from io import BytesIO
from pathlib import Path
import struct

from PIL import Image, ImageDraw


ROOT = Path(__file__).resolve().parent.parent
DEST = ROOT / "assets" / "app-icon"
SIZES = (16, 20, 24, 32, 40, 48, 64, 128, 256, 512, 1024)
LINUX_SIZES = (16, 24, 32, 48, 64, 128, 256, 512)
ICO_SIZES = (16, 20, 24, 32, 40, 48, 64, 128, 256)
SCALE = 3

# Original monoline lettering in one-em units; no third-party glyphs.
GLYPHS = {
    "Z": (0.72, [[(0, 0), (0.72, 0), (0, 1), (0.72, 1)]]),
    "n": (0.58, [[(0, 1), (0, 0.3)], [(0, 0.48), (0.18, 0.3), (0.4, 0.3), (0.58, 0.48), (0.58, 1)]]),
    "u": (0.58, [[(0, 0.3), (0, 0.83), (0.15, 1), (0.4, 1), (0.58, 0.83), (0.58, 0.3)], [(0.58, 0.83), (0.58, 1)]]),
    "i": (0.12, [[(0.06, 0.38), (0.06, 1)], [(0.06, 0.05), (0.06, 0.08)]]),
    "t": (0.48, [[(0.2, 0.05), (0.2, 0.84), (0.34, 1), (0.48, 1)], [(0, 0.33), (0.46, 0.33)]]),
    "A": (0.72, [[(0, 1), (0.36, 0), (0.72, 1)], [(0.15, 0.62), (0.57, 0.62)]]),
    "I": (0.4, [[(0, 0), (0.4, 0)], [(0.2, 0), (0.2, 1)], [(0, 1), (0.4, 1)]]),
    "C": (0.68, [[(0.68, 0.12), (0.5, 0), (0.2, 0), (0, 0.2), (0, 0.8), (0.2, 1), (0.5, 1), (0.68, 0.88)]]),
    "D": (0.68, [[(0, 1), (0, 0), (0.4, 0), (0.68, 0.24), (0.68, 0.76), (0.4, 1), (0, 1)]]),
}


class Artwork:
    """Draw the same geometry into an antialiased bitmap and an SVG."""

    def __init__(self):
        self.image = Image.new("RGBA", (1024 * SCALE, 1024 * SCALE))
        self.draw = ImageDraw.Draw(self.image)
        self.svg = []

    def rect(self, box, radius, color):
        self.draw.rounded_rectangle(tuple(v * SCALE for v in box), radius * SCALE, fill=color)
        x, y, right, bottom = box
        self.svg.append(f'<rect x="{x}" y="{y}" width="{right-x}" height="{bottom-y}" rx="{radius}" fill="{color}"/>')

    def polygon(self, points, color):
        self.draw.polygon([(x * SCALE, y * SCALE) for x, y in points], fill=color)
        coords = " ".join(f"{x:g},{y:g}" for x, y in points)
        self.svg.append(f'<polygon points="{coords}" fill="{color}"/>')

    def line(self, points, color, width):
        scaled = [(round(x * SCALE), round(y * SCALE)) for x, y in points]
        self.draw.line(scaled, fill=color, width=round(width * SCALE), joint="curve")
        radius = width * SCALE / 2
        for x, y in scaled:
            self.draw.ellipse((x-radius, y-radius, x+radius, y+radius), fill=color)
        coords = " ".join(f"{x:g},{y:g}" for x, y in points)
        self.svg.append(f'<polyline points="{coords}" fill="none" stroke="{color}" stroke-width="{width:g}" stroke-linecap="round" stroke-linejoin="round"/>')

    def finish(self):
        return self.image.resize((1024, 1024), Image.Resampling.LANCZOS)


def artwork(full):
    art = Artwork()
    art.rect((0, 0, 1023, 1023), 214, "#101e36")
    if full:
        for p in (256, 512, 768):
            art.line([(p, 100), (p, 714)], "#1c304a", 2)
            art.line([(110, p), (914, p)], "#1c304a", 2)
        art.line([(140, 320), (140, 140), (320, 140)], "#304c67", 6)
        art.line([(884, 564), (884, 744), (704, 744)], "#304c67", 6)
        x0, x1, y0, y1, band = 258, 766, 207, 652, 100
    else:
        x0, x1, y0, y1, band = 244, 780, 252, 772, 120
    art.polygon([(x0, y0), (x1, y0), (x1, y0+band), (x0+band*1.5, y1-band), (x0, y1-band), (x1-band*1.5, y0+band), (x0, y0+band)], "#31dfdc")
    art.polygon([(x0, y0), (x1, y0), (x1, y0+band), (x0, y0+band)], "#f3f7ff")
    art.polygon([(x0, y1-band), (x1, y1-band), (x1, y1), (x0, y1)], "#f3f7ff")
    if full:
        name, em, gap = "ZnunitAICAD", 100, 0.17
        total = sum(GLYPHS[ch][0] for ch in name) + gap * (len(name)-1)
        cursor = (1024 - total * em) / 2
        for index, char in enumerate(name):
            width, paths = GLYPHS[char]
            for path in paths:
                art.line([(cursor+x*em, 802+y*em) for x, y in path], "#f3f7ff" if index < 6 else "#31dfdc", 9)
            cursor += (width + gap) * em
    return art


def png_bytes(image):
    data = BytesIO()
    image.save(data, format="PNG", optimize=True)
    return data.getvalue()


def write_ico(images, path):
    chunks = [png_bytes(images[size]) for size in ICO_SIZES]
    offset = 6 + 16 * len(chunks)
    directory = bytearray(struct.pack("<HHH", 0, 1, len(chunks)))
    for size, data in zip(ICO_SIZES, chunks):
        dim = size if size < 256 else 0
        directory.extend(struct.pack("<BBBBHHII", dim, dim, 0, 0, 1, 32, len(data), offset))
        offset += len(data)
    path.write_bytes(bytes(directory) + b"".join(chunks))


def write_icns(images, path):
    # PNG-backed macOS icon slots, including Retina slots.
    slots = ((b"icp4", 16), (b"icp5", 32), (b"icp6", 64), (b"ic07", 128),
             (b"ic08", 256), (b"ic09", 512), (b"ic10", 1024),
             (b"ic11", 32), (b"ic12", 64), (b"ic13", 256), (b"ic14", 512))
    blocks = []
    for tag, size in slots:
        data = png_bytes(images[size])
        blocks.append(tag + struct.pack(">I", len(data)+8) + data)
    body = b"".join(blocks)
    path.write_bytes(b"icns" + struct.pack(">I", len(body)+8) + body)


def main():
    DEST.mkdir(parents=True, exist_ok=True)
    full, compact = artwork(True), artwork(False)
    svg = ('<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" viewBox="0 0 1024 1024">\n'
           '<title>ZnunitAICAD</title>\n<desc>Original Z drafting mark and original outlined lettering. MIT OR Apache-2.0.</desc>\n'
           + "\n".join(full.svg) + "\n</svg>\n")
    (DEST / "ZnunitAICAD.svg").write_text(svg, encoding="utf-8")
    masters = {True: full.finish(), False: compact.finish()}
    images, mac_images = {}, {}
    for size in SIZES:
        image = masters[size >= 128].resize((size, size), Image.Resampling.LANCZOS)
        images[size] = image
        image.save(DEST / f"ZnunitAICAD-{size}.png", optimize=True)
        # Preserve the existing 47 px margin on a 512 px macOS canvas.
        inner = round(size * 418 / 512)
        mac = Image.new("RGBA", (size, size))
        mark = masters[inner >= 128].resize((inner, inner), Image.Resampling.LANCZOS)
        mac.paste(mark, ((size-inner)//2, (size-inner)//2))
        mac_images[size] = mac
    for size in (64, 256, 1024):
        images[size].save(DEST / f"cadcraft-{size}.png", optimize=True)
    mac_images[512].save(DEST / "cadcraft-macos-512.png", optimize=True)
    mac_images[512].save(DEST / "ZnunitAICAD-macos-512.png", optimize=True)
    for size in LINUX_SIZES:
        folder = DEST / "hicolor" / f"{size}x{size}" / "apps"
        folder.mkdir(parents=True, exist_ok=True)
        images[size].save(folder / "ai.storyteller.cadcraft.png", optimize=True)
    write_ico(images, DEST / "cadcraft.ico")
    write_icns(mac_images, DEST / "cadcraft.icns")
    (DEST / "ZnunitAICAD.ico").write_bytes((DEST / "cadcraft.ico").read_bytes())
    (DEST / "ZnunitAICAD.icns").write_bytes((DEST / "cadcraft.icns").read_bytes())
    print(f"ZnunitAICAD: {len(SIZES)} PNG sizes, SVG, Windows ICO and macOS ICNS written to {DEST}")


if __name__ == "__main__":
    main()
