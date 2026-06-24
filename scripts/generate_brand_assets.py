#!/usr/bin/env python3
"""Generate Amentia brand assets and lightweight SVG preview wrappers."""

from __future__ import annotations

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
RGBA_BORDER = (232, 237, 243, 255)
RGBA_WHITE = (255, 255, 255, 255)
RGBA_TRANSPARENT = (255, 255, 255, 0)

ICON_DESIGN_SIZE = 1254
ICON_OUTPUT_SIZE = 2048
SCALE = 4

TILE = (28, 28, 1226, 1226)
TILE_RADIUS = 246
TILE_STROKE = 5

ICON_MARK_BOX = (306, 376, 948, 868)
REFERENCE_MARK_PADDING = 2
REFERENCE_BLUE_MINIMUM = 120
REFERENCE_BLUE_RED_DELTA = 35
REFERENCE_BLUE_GREEN_DELTA = 5
REFERENCE_BACKGROUND_CHROMA_FLOOR = 3
REFERENCE_ANTIALIAS_ALPHA_GAIN = 14


def scaled_box(box: tuple[int, int, int, int]) -> tuple[int, int, int, int]:
  return tuple(value * SCALE for value in box)


def write_text(path: Path, text: str) -> None:
  path.write_text(text, encoding="utf-8", newline="\n")


def reference_mark_bounds(image: Image.Image) -> tuple[int, int, int, int]:
  left = image.width
  top = image.height
  right = 0
  bottom = 0
  pixels = image.load()
  for y in range(image.height):
    for x in range(image.width):
      red, green, blue, alpha = pixels[x, y]
      if alpha == 0:
        continue
      if (
        blue >= REFERENCE_BLUE_MINIMUM
        and blue >= red + REFERENCE_BLUE_RED_DELTA
        and blue >= green + REFERENCE_BLUE_GREEN_DELTA
      ):
        left = min(left, x)
        top = min(top, y)
        right = max(right, x + 1)
        bottom = max(bottom, y + 1)

  if right == 0 or bottom == 0:
    raise RuntimeError(f"Approved lockup reference is missing a blue mark: {LOCKUP_REFERENCE_PNG}")
  return left, top, right, bottom


def expanded_box(
  box: tuple[int, int, int, int],
  image_size: tuple[int, int],
  padding: int,
) -> tuple[int, int, int, int]:
  left, top, right, bottom = box
  width, height = image_size
  return (
    max(0, left - padding),
    max(0, top - padding),
    min(width, right + padding),
    min(height, bottom + padding),
  )


def reference_mark_image() -> Image.Image:
  if not LOCKUP_REFERENCE_PNG.exists():
    raise FileNotFoundError(f"Missing approved lockup reference: {LOCKUP_REFERENCE_PNG}")
  reference = Image.open(LOCKUP_REFERENCE_PNG).convert("RGBA")
  bounds = expanded_box(
    reference_mark_bounds(reference),
    reference.size,
    REFERENCE_MARK_PADDING,
  )
  mark = reference.crop(bounds)
  pixels = mark.load()
  for y in range(mark.height):
    for x in range(mark.width):
      red, green, blue, alpha = pixels[x, y]
      blue_chroma = blue - max(red, green)
      if alpha == 0 or blue_chroma <= REFERENCE_BACKGROUND_CHROMA_FLOOR:
        pixels[x, y] = (255, 255, 255, 0)
        continue
      extracted_alpha = min(255, max(0, blue_chroma * REFERENCE_ANTIALIAS_ALPHA_GAIN))
      pixels[x, y] = (red, green, blue, extracted_alpha)
  return mark


def write_icon_svg() -> None:
  write_text(
    ICON_SVG,
    f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {ICON_OUTPUT_SIZE} {ICON_OUTPUT_SIZE}" role="img" aria-label="Amentia blue app icon">
  <image data-preview-source="{ICON_PNG.name}" href="{ICON_PNG.name}" width="{ICON_OUTPUT_SIZE}" height="{ICON_OUTPUT_SIZE}" preserveAspectRatio="xMidYMid meet"/>
</svg>
""",
  )


def write_icon_png() -> None:
  image = Image.new(
    "RGBA",
    (ICON_DESIGN_SIZE * SCALE, ICON_DESIGN_SIZE * SCALE),
    RGBA_TRANSPARENT,
  )
  draw = ImageDraw.Draw(image)
  draw.rounded_rectangle(
    scaled_box(TILE),
    radius=TILE_RADIUS * SCALE,
    fill=RGBA_WHITE,
    outline=RGBA_BORDER,
    width=TILE_STROKE * SCALE,
  )
  mark = reference_mark_image().resize(
    (
      (ICON_MARK_BOX[2] - ICON_MARK_BOX[0]) * SCALE,
      (ICON_MARK_BOX[3] - ICON_MARK_BOX[1]) * SCALE,
    ),
    Image.Resampling.LANCZOS,
  )
  image.alpha_composite(mark, (ICON_MARK_BOX[0] * SCALE, ICON_MARK_BOX[1] * SCALE))
  image = image.resize((ICON_OUTPUT_SIZE, ICON_OUTPUT_SIZE), Image.Resampling.LANCZOS)
  image.save(ICON_PNG)


def write_lockup_svg() -> None:
  if not LOCKUP_REFERENCE_PNG.exists():
    raise FileNotFoundError(f"Missing approved lockup reference: {LOCKUP_REFERENCE_PNG}")
  write_text(
    LOCKUP_SVG,
    f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1000 310" role="img" aria-label="Amentia wordmark lockup">
  <image data-preview-source="{LOCKUP_REFERENCE_PNG.name}" href="{LOCKUP_REFERENCE_PNG.name}" width="1000" height="310" preserveAspectRatio="xMidYMid meet"/>
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
