import type { IconProps } from '@opal/types'
// The mark on alpha, trimmed to its own bounds and squared — not the packaged
// application icon. `src-tauri/icons/64x64.png` is a fully opaque tile, correct
// for a taskbar where the platform draws no backdrop, and wrong here: rendered
// in the sidebar it is a hard-edged rectangle darker than the surface behind
// it, which reads as a box in the dark theme and as a black square in the
// light one.
//
// 112px source for a 28px box: 4x, so the glyph stays crisp on a 2x display
// and still has headroom on a 3x one.
import markUrl from './rhodiz-mark.png'

// The mark itself is silver, drawn for a dark backdrop; on the light theme's
// surface it washes out. So the backdrop travels with it instead of being
// inherited — the same two values the packaged icon is built from, which is
// what keeps the sidebar chip and the taskbar icon the same object. Fixed on
// purpose: an identity that changes colour with the theme is two identities.
const CHIP = 'linear-gradient(135deg, #161B23 0%, #090B0F 100%)'

export default function RhodizMark({ size = 28 }: IconProps) {
  return (
    <span
      aria-hidden="true"
      className="inline-flex shrink-0 items-center justify-center rounded-lg"
      style={{ width: size, height: size, background: CHIP, padding: size * 0.12 }}
    >
      <img
        src={markUrl}
        alt=""
        draggable={false}
        className="size-full select-none"
      />
    </span>
  )
}
