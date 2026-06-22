# Amentia Brand Direction

Amentia's current identity direction is a refined geometric blue mark on a
clean white rounded-square tile with transparent outer corners. The baseline
blue is `#2F82F3`.

The mark should feel precise, lightweight, local-first, and quietly capable. It
should avoid mascots, generic AI symbols, circuits, chat bubbles, gradients, and
decorative extra marks. The useful detail is the subtle inner cut, which makes
the shape custom without hurting small-size legibility.

Current candidate:

- `amentia-blue-symbol-icon-source.svg`
- `amentia-blue-symbol-icon-candidate.png`
- `amentia-wordmark-lockup-source.svg`
- `amentia-wordmark-lockup-reference.png`

Source of truth:

- Regenerate the packaged icon assets with
  `python scripts/generate_brand_assets.py` so the PNG icon and editable SVG
  reference stay aligned.
- Use `amentia-wordmark-lockup-reference.png` as the approved visual reference
  for the wordmark lockup. The lockup SVG is structural and may render with
  different fonts on machines without the preferred typeface.
- The packaging script derives the macOS `Amentia.icns` Dock icon from the PNG
  candidate and rejects opaque square corners during app bundle creation.
