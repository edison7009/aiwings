"""Joins record.cjs frames into a looping GIF with one shared palette (no colour flicker between frames).

Usage: python3 make_gif.py <framesDir> <out.gif> [fps]   (needs Pillow)
"""
import sys
from pathlib import Path
from PIL import Image

src, out = Path(sys.argv[1]), Path(sys.argv[2])
fps = int(sys.argv[3]) if len(sys.argv) > 3 else 15
files = sorted(src.glob("*.png"))
frames = [Image.open(f).convert("RGB") for f in files]
# One palette for the whole clip, taken from a strip of frames that shows every element.
sample = Image.new("RGB", (frames[0].width, frames[0].height * 4))
for i, k in enumerate((0.15, 0.45, 0.7, 0.85)):
    sample.paste(frames[int(len(frames) * k)], (0, frames[0].height * i))
palette = sample.quantize(colors=255, method=Image.Quantize.MEDIANCUT)
pal = [f.quantize(palette=palette, dither=Image.Dither.NONE) for f in frames]
pal[0].save(out, save_all=True, append_images=pal[1:], duration=round(1000 / fps), loop=0, optimize=True, disposal=1)
print(f"{len(frames)} frames → {out} ({out.stat().st_size // 1024} KB)")
