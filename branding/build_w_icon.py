"""Builds the app icon and the tray icon from the single-letter W sticker (branding/source/wings-w.png).

App icon: the W on the EchoBird-style white tile (same tile as build_icon.py), for the desktop, Dock and
taskbar. Tray icon: the W alone on a transparent background, trimmed to its edges, for the menu bar /
notification area.

Run: python3 branding/build_w_icon.py   (needs Pillow), then: npx tauri icon branding/wings-app-icon-1024.png
"""
from pathlib import Path
from PIL import Image
from build_icon import CANVAS, INSET, icon

ROOT = Path(__file__).resolve().parent
SRC = ROOT / "source" / "wings-w.png"
OUT_APP = ROOT / "wings-app-icon-1024.png"
OUT_TRAY = ROOT.parent / "src-tauri" / "icons" / "tray.png"


def trimmed(src: Image.Image) -> Image.Image:
    """Crop to the sticker: pixels that are more than faint haze."""
    alpha = src.getchannel("A").point(lambda p: 255 if p > 24 else 0)
    return src.crop(alpha.getbbox())


if __name__ == "__main__":
    w = trimmed(Image.open(SRC).convert("RGBA"))
    icon(w).save(OUT_APP)
    # Square canvas so the tray shows it unstretched; 64 px covers 2x menu bars and 125-200% Windows scaling.
    side = max(w.size)
    sq = Image.new("RGBA", (side, side), (0, 0, 0, 0))
    sq.alpha_composite(w, ((side - w.width) // 2, (side - w.height) // 2))
    sq.resize((64, 64), Image.LANCZOS).save(OUT_TRAY)
    print("wrote", OUT_APP.name, "and", OUT_TRAY.name, "from", w.size)
