import type { IconProps } from '@opal/types'

/**
 * "Follow the system", as a contrast disc.
 *
 * Lives here and not in `src/design/opal/icons` because that tree is vendored
 * and covered by a digest in PROVENANCE.md; a checksum over files we add to
 * attests to nothing. It is written to Opal's own conventions all the same —
 * 16-unit box, `currentColor`, 1.5 stroke — so it sits beside `SvgSun` and
 * `SvgMoon` in a row without looking imported from somewhere else.
 *
 * A half-filled circle rather than a monitor: the three states this control
 * cycles through are one idea (how much light), and a screen glyph would say
 * "display settings" instead. Opal ships neither, and its `SvgCircle` is a
 * plain ring with no fill, so there was nothing to reuse.
 *
 * The disc spans 2 to 14 — the same bounds as `SvgMoon`'s body — so the icon
 * does not change weight as the preference changes.
 */
const SvgThemeAuto = ({ size, ...props }: IconProps) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 16 16"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    stroke="currentColor"
    {...props}
  >
    {/* Drawn before the outline so the stroke closes over the fill's flat edge. */}
    <path d="M8 2.75A5.25 5.25 0 0 0 8 13.25Z" fill="currentColor" stroke="none" />
    <circle cx={8} cy={8} r={5.25} strokeWidth={1.5} />
  </svg>
)

export default SvgThemeAuto
