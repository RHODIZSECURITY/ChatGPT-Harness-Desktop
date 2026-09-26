# Brand masters

The RHODIZ mark, in the two forms the rest of the repository generates from.

| File | What it is |
| --- | --- |
| `rhodiz-mark-1024-transparent.png` | The mark alone, on alpha. No background. |
| `rhodiz-mark-1024.png` | The application icon master: the mark above, composed on the icon tile. |

## Where they came from

Both are derived from the brand original supplied by RHODIZ — a 1254×1254 RGB
render with **no alpha channel**, whose dark backdrop is part of the raster
rather than a separate layer. That original is not an icon: measured on it, the
backdrop is a photographic gradient that is brightest in the top-right corner
(per-corner mean luma 32.3 / 43.9 / 15.9 / 16.3), and the mark sits 32 px above
the canvas centre, with the ring itself 941 × 883 rather than round and its
centre 50 px high.

Two edits were made, and only two:

1. **The backdrop was separated from the mark.** The background is estimated as
   a second-order polynomial surface fitted over the pixels that are dark and
   not blue, then subtracted under an additive model — the glow is light *added*
   over a dark surface, so subtraction recovers it rather than clipping it.
   Alpha is the union of the glow's magnitude, the solid body's luminance, and a
   morphological closing of the brightest mass, so the chrome stays opaque while
   the halo stays graded. Colour is un-premultiplied and rescaled by a measured
   1.041 so the brightest chrome lands at 253 rather than clipping. Fit residual:
   σ = 8.72 over a background whose mean is `[19, 23, 30]`.

2. **The mark was re-composed on a controlled tile.** A flat radial from
   `#161B23` to `#090B0F` — the same family as the original backdrop, without its
   texture, its corner hotspot or its banding. The mark is placed by its
   *alpha-weighted centroid*, not by its bounding box: the trace that drops
   below the ring is thin and light, and centring the box would push the heavy
   ring visibly high. It fills 82% of the canvas, which leaves the safe margin
   Windows tiles and macOS both expect.

Nothing was redrawn, recoloured or cropped. The mark is the one that was
supplied.

## Regenerating the icon set

```
node_modules/.bin/tauri icon assets/brand/rhodiz-mark-1024.png -o src-tauri/icons
```

Then discard `src-tauri/icons/{ios,android}/`: the generator emits them
unconditionally and this application ships neither.

The taskbar sizes are not left as downsamples of the master. 16/24/32/48/64 are
re-composed at their native resolution — mark at 90% fill, +25% contrast, a 1.2px
unsharp — because a mark built from parallel hairlines loses them to a single
Lanczos pass, and those are the sizes a user actually looks at. `icon.ico`
carries them as distinct layers alongside 128 and 256 from the master;
`32x32.png` and `64x64.png` are the same renders.

## What is still missing

There is no vector of the mark. Every asset here is a raster derived from a
raster, so the smallest sizes are as good as resampling can make them and no
better. A drawn glyph — ring and stem only, no circuit detail — would read
better at 16 px than anything derivable from this source, and an SVG would let
the sidebar tint the mark to the current theme instead of showing a tile. Both
need the original vector artwork, which this repository does not have.
