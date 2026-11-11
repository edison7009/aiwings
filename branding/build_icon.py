"""Builds the Wings banner and app icon from the sticker wordmark (branding/source/wings-sticker.png).

Layout follows the EchoBird macOS icon: 1024 canvas, a rounded square of 816 px inset 104 px with
~198 px corners, a white-to-light-grey vertical gradient, a hairline grey edge and a soft drop shadow.
The source's background (a baked-in checkerboard) is dropped: the coloured parts and the black tagline are
cut out and given a fresh white die-cut border and drop shadow on a truly transparent background.

Run: python3 branding/build_icon.py   (needs Pillow)
"""
from pathlib import Path
from PIL import Image, ImageChops, ImageDraw, ImageFilter

ROOT = Path(__file__).resolve().parent
SRC = ROOT / "source" / "wings-sticker.png"
OUT_ICON = ROOT / "wings-icon-1024.png"
OUT_MARK = ROOT / "wings-banner.png"

CANVAS, INSET, RADIUS = 1024, 104, 198
TOP, BOTTOM, EDGE = (255, 255, 255), (233, 237, 241), (226, 230, 234)


def ink_mask(src: Image.Image) -> Image.Image:
    """Coloured or dark pixels (letters, outlines, shadow, wing outline, tagline), holes filled."""
    h, s, v = src.convert("HSV").split()
    # Saturation means little for near-black pixels, so colour also needs some brightness.
    colour = ImageChops.multiply(s.point(lambda p: 255 if p > 60 else 0), v.point(lambda p: 255 if p > 80 else 0))
    dark = v.point(lambda p: 255 if p < 110 else 0)
    mask = ImageChops.lighter(colour, dark)
    # Opening drops stray specks (JPEG-like noise) before the holes are filled.
    mask = mask.filter(ImageFilter.MinFilter(5)).filter(ImageFilter.MaxFilter(5))
    # Fill enclosed holes (white highlights, the wing's white body): flood the outside, keep the rest.
    padded = Image.new("L", (mask.width + 2, mask.height + 2), 0)
    padded.paste(mask, (1, 1))
    ImageDraw.floodfill(padded, (0, 0), 128, thresh=0)
    filled = padded.point(lambda p: 0 if p == 128 else 255).crop((1, 1, mask.width + 1, mask.height + 1))
    return filled


def sticker(src: Image.Image) -> Image.Image:
    """The wordmark on a clean white die-cut border with a soft shadow, transparent outside."""
    mask = ink_mask(src)
    content = mask.filter(ImageFilter.MaxFilter(5))  # take the anti-aliased outline edge along
    # Die-cut border: grow the art ~20 px, closing the notches between the letters and the tagline
    # (grow ~42 px, shrink ~21 px). Done at quarter size, where the filters are cheap, then smoothed.
    q = mask.resize((mask.width // 4, mask.height // 4), Image.BILINEAR).point(lambda p: 255 if p > 40 else 0)
    q = q.filter(ImageFilter.MaxFilter(21)).filter(ImageFilter.MinFilter(11))
    border = q.resize(mask.size, Image.BICUBIC).filter(ImageFilter.GaussianBlur(4)).point(lambda p: 255 if p > 128 else 0)
    border = ImageChops.lighter(border, mask.filter(ImageFilter.MaxFilter(21)))
    border = border.filter(ImageFilter.GaussianBlur(1))  # smooth edge
    pad = 60
    w, h = src.width + pad * 2, src.height + pad * 2
    out = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    shadow = Image.new("RGBA", (w, h), (20, 28, 40, 0))
    shadow.putalpha(Image.new("L", (w, h), 0))
    sa = Image.new("L", (w, h), 0)
    sa.paste(border.point(lambda p: p * 0.22), (pad, pad + 10))
    shadow.putalpha(sa.filter(ImageFilter.GaussianBlur(14)))
    out = Image.alpha_composite(out, shadow)
    white = Image.new("RGBA", (w, h), (255, 255, 255, 0))
    wa = Image.new("L", (w, h), 0)
    wa.paste(border, (pad, pad))
    white.putalpha(wa)
    out = Image.alpha_composite(out, white)
    art = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    art.paste(src.convert("RGBA"), (pad, pad))
    ca = Image.new("L", (w, h), 0)
    ca.paste(content, (pad, pad))
    art.putalpha(ca)
    out = Image.alpha_composite(out, art)
    return out.crop(out.getbbox())


def rounded_mask(size: int, box, radius: int) -> Image.Image:
    # Drawn 4x larger, then reduced, for smooth corners.
    k = 4
    big = Image.new("L", (size * k, size * k), 0)
    ImageDraw.Draw(big).rounded_rectangle([c * k for c in box], radius * k, fill=255)
    return big.resize((size, size), Image.LANCZOS)


def icon(mark: Image.Image) -> Image.Image:
    c = Image.new("RGBA", (CANVAS, CANVAS), (0, 0, 0, 0))
    box = (INSET, INSET, CANVAS - INSET - 1, CANVAS - INSET - 1)
    # Soft drop shadow under the tile.
    sh = rounded_mask(CANVAS, (box[0], box[1] + 14, box[2], box[3] + 14), RADIUS).point(lambda p: p * 0.16)
    shadow = Image.new("RGBA", c.size, (16, 26, 40, 0))
    shadow.putalpha(sh.filter(ImageFilter.GaussianBlur(18)))
    c = Image.alpha_composite(c, shadow)
    # Hairline edge, then the gradient face inset by 3 px.
    edge = Image.new("RGBA", c.size, EDGE + (0,))
    edge.putalpha(rounded_mask(CANVAS, box, RADIUS))
    c = Image.alpha_composite(c, edge)
    grad = Image.new("RGBA", c.size)
    gd = ImageDraw.Draw(grad)
    for y in range(CANVAS):
        t = min(max((y - INSET) / (CANVAS - 2 * INSET), 0), 1)
        gd.line([(0, y), (CANVAS, y)], fill=tuple(round(a + (b - a) * t) for a, b in zip(TOP, BOTTOM)) + (255,))
    inner = (box[0] + 3, box[1] + 3, box[2] - 3, box[3] - 3)
    grad.putalpha(rounded_mask(CANVAS, inner, RADIUS - 3))
    c = Image.alpha_composite(c, grad)
    # The sticker, 84% of the tile's width, centred.
    tile = CANVAS - 2 * INSET
    w = round(tile * 0.84)
    h = round(mark.height * w / mark.width)
    m = mark.resize((w, h), Image.LANCZOS)
    c.alpha_composite(m, ((CANVAS - w) // 2, (CANVAS - h) // 2 + 6))
    return c


if __name__ == "__main__":
    mark = sticker(Image.open(SRC).convert("RGB"))
    mark.save(OUT_MARK)
    icon(mark).save(OUT_ICON)
    print("wrote", OUT_MARK.name, mark.size, "and", OUT_ICON.name)
