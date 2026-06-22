#!/usr/bin/env python3
"""Generate Amentia brand assets and exact SVG preview wrappers."""

from __future__ import annotations

import base64
from pathlib import Path

try:
  from PIL import Image, ImageDraw
except ModuleNotFoundError as exc:
  raise SystemExit("Brand asset generation requires Pillow.") from exc


ROOT = Path(__file__).resolve().parents[1]
BRAND_DIR = ROOT / "docs" / "brand"
ICON_SVG = BRAND_DIR / "amentia-blue-symbol-icon-source.svg"
ICON_PNG = BRAND_DIR / "amentia-blue-symbol-icon-candidate.png"
LOCKUP_SVG = BRAND_DIR / "amentia-wordmark-lockup-source.svg"
LOCKUP_REFERENCE_PNG = BRAND_DIR / "amentia-wordmark-lockup-reference.png"

BORDER = "#e8edf3"
RGBA_BLUE = (47, 130, 243, 255)
RGBA_BORDER = (232, 237, 243, 255)
RGBA_WHITE = (255, 255, 255, 255)
RGBA_TRANSPARENT = (255, 255, 255, 0)

ICON_SIZE = 1254
SCALE = 4

TILE = (112, 100, 1142, 1130)
TILE_RADIUS = 214
TILE_STROKE = 5

MARK_WIDTH = 220
MARK_HEIGHT = 168
MARK_OUTER = ((110, 0), (220, 168), (0, 168))
MARK_NEGATIVE = (
  (110, 68),
  (67, 140),
  (103, 140),
  (118, 168),
  (170, 168),
  (148, 140),
  (121, 140),
  (105, 111),
  (154, 111),
  (135, 86),
  (122, 86),
)
ICON_MARK_BOX = (318, 388, 936, 860)


def transformed_points(
  points: tuple[tuple[int, int], ...],
  box: tuple[int, int, int, int],
) -> tuple[tuple[int, int], ...]:
  x0, y0, x1, y1 = box
  scale_x = (x1 - x0) / MARK_WIDTH
  scale_y = (y1 - y0) / MARK_HEIGHT
  return tuple((round(x0 + x * scale_x), round(y0 + y * scale_y)) for x, y in points)


def scaled_box(box: tuple[int, int, int, int]) -> tuple[int, int, int, int]:
  return tuple(value * SCALE for value in box)


def scaled_points(points: tuple[tuple[int, int], ...]) -> list[tuple[int, int]]:
  return [(x * SCALE, y * SCALE) for x, y in points]


def write_text(path: Path, text: str) -> None:
  path.write_text(text, encoding="utf-8", newline="\n")


def png_data_uri(path: Path) -> str:
  return "data:image/png;base64," + base64.b64encode(path.read_bytes()).decode("ascii")


def draw_mark(draw: ImageDraw.ImageDraw, box: tuple[int, int, int, int]) -> None:
  draw.polygon(scaled_points(transformed_points(MARK_OUTER, box)), fill=RGBA_BLUE)
  draw.polygon(scaled_points(transformed_points(MARK_NEGATIVE, box)), fill=RGBA_WHITE)


def write_icon_svg() -> None:
  write_text(
    ICON_SVG,
    f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1254 1254" role="img" aria-label="Amentia blue app icon">
  <image data-preview-source="{ICON_PNG.name}" href="{png_data_uri(ICON_PNG)}" width="1254" height="1254" preserveAspectRatio="xMidYMid meet"/>
</svg>
""",
  )


def write_icon_png() -> None:
  image = Image.new("RGBA", (ICON_SIZE * SCALE, ICON_SIZE * SCALE), RGBA_TRANSPARENT)
  draw = ImageDraw.Draw(image)
  draw.rounded_rectangle(
    scaled_box(TILE),
    radius=TILE_RADIUS * SCALE,
    fill=RGBA_WHITE,
    outline=RGBA_BORDER,
    width=TILE_STROKE * SCALE,
  )
  draw_mark(draw, ICON_MARK_BOX)
  image = image.resize((ICON_SIZE, ICON_SIZE), Image.Resampling.LANCZOS)
  image.save(ICON_PNG)


def write_lockup_svg() -> None:
  if not LOCKUP_REFERENCE_PNG.exists():
    raise FileNotFoundError(f"Missing approved lockup reference: {LOCKUP_REFERENCE_PNG}")
  write_text(
    LOCKUP_SVG,
    f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1000 310" role="img" aria-label="Amentia wordmark lockup">
  <image data-preview-source="{LOCKUP_REFERENCE_PNG.name}" href="{png_data_uri(LOCKUP_REFERENCE_PNG)}" width="1000" height="310" preserveAspectRatio="xMidYMid meet"/>
</svg>
""",
  )


def main() -> None:
  BRAND_DIR.mkdir(parents=True, exist_ok=True)
  write_icon_png()
  write_icon_svg()
  write_lockup_svg()
  print(f"Generated {ICON_SVG.relative_to(ROOT)}")
  print(f"Generated {ICON_PNG.relative_to(ROOT)}")
  print(f"Generated {LOCKUP_SVG.relative_to(ROOT)}")


if __name__ == "__main__":
  main()
