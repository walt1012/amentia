# Amentia Brand Direction

Amentia's current identity direction is the approved blue geometric A mark and
wordmark lockup in `amentia-wordmark-lockup-reference.png`, with a clean white
rounded-square macOS app tile for the Dock icon. The tile should fill the icon
frame and keep only the minimal transparent corner mask required for rounded app
presentation.

The mark should feel precise, lightweight, local-first, and quietly capable. It
should avoid mascots, generic AI symbols, circuits, chat bubbles, heavy
decorative gradients, and extra marks. The useful detail is the subtle inner
cut, which makes the shape custom without hurting small-size legibility.

Current candidate:

- `amentia-blue-symbol-icon-source.svg`
- `amentia-blue-symbol-icon-candidate.png`
- `amentia-wordmark-lockup-source.svg`
- `amentia-wordmark-lockup-reference.png`

Source of truth:

- Regenerate the packaged icon assets with
  `python scripts/generate_brand_assets.py` so the packaged PNG icon and SVG
  preview wrapper stay aligned.
- Use `amentia-wordmark-lockup-reference.png` as the approved visual reference
  for the wordmark lockup. The lockup SVG must embed that PNG directly rather
  than approximating the wordmark with system fonts.
- The app icon mark is extracted from the approved lockup reference rather than
  redrawn with a second polygon recipe. The generated icon must not paste the
  reference image's white crop background as a visible plate.
- The packaging script derives the macOS `Amentia.icns` Dock icon from the PNG
  candidate and rejects opaque square corners or excessive transparent padding
  during app bundle creation.
- The generated app icon PNG uses a 2048 px source so the packaged ICNS has a
  clean high-resolution master while preserving the approved 1254-unit design
  proportions.
