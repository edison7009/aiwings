"""Builds the app icon and the tray icon from the single-letter W sticker (branding/source/wings-w.png).

App icon: the W on the EchoBird-style white tile (same tile as build_icon.py). The macOS one keeps Apple's
grid margin and shadow; the full one fills the canvas (Windows desktop/taskbar, where a margin just makes it
look small). Tray icon: the full tile scaled down, so the W has a clean white edge on any taskbar colour.

Run: python3 branding/build_w_icon.py   (needs Pillow), then:
  npx tauri icon branding/wings-app-icon-full-1024.png
  npx tauri icon branding/wings-app-icon-1024.png -o <tmp>  and copy <tmp>/icon.icns into src-tauri/icons
"""
from pathlib import Path
from PIL import Image
from build_icon import CANVAS, INSET, icon

ROOT = Path(__file__).resolve().parent
SRC = ROOT / "source" / "wings-w.png"
OUT_APP = ROOT / "wings-app-icon-1024.png"
OUT_FULL = ROOT / "wings-app-icon-full-1024.png"
OUT_TRAY = ROOT.parent / "src-tauri" / "icons" / "tray.png"


def trimmed(src: Image.Image) -> Image.Image:
    """Crop to the sticker: pixels that are more than faint haze."""
    alpha = src.getchannel("A").point(lambda p: 255 if p > 24 else 0)
    return src.crop(alpha.getbbox())


if __name__ == "__main__":
    w = trimmed(Image.open(SRC).convert("RGBA"))
    icon(w).save(OUT_APP)
    full = icon(w, inset=8, shadow=False)
    full.save(OUT_FULL)
    # 64 px covers 2x menu bars and 125-200% Windows scaling.
    full.resize((64, 64), Image.LANCZOS).save(OUT_TRAY)
    print("wrote", OUT_APP.name, OUT_FULL.name, "and", OUT_TRAY.name, "from", w.size)
