import type { IconProps } from '@opal/types'
// The application icon itself, imported from where Tauri bundles it so the
// sidebar, the title bar and the taskbar cannot drift apart. There is no
// vector of the mark in this repository; when one exists it replaces this
// file and nothing that renders it has to change.
import markUrl from '../../src-tauri/icons/32x32.png'

/**
 * The RHODIZ mark, in the shape `SidebarLayouts.Header`'s `renderAppLogo`
 * expects.
 *
 * `IconProps` is declared over `SVGProps`, and this renders an `<img>`,
 * so the SVG-specific attributes are deliberately not forwarded: passing
 * `strokeWidth` to an image would be silently meaningless. Only `size` and
 * the accessible name apply.
 */
export default function RhodizMark({ size = 28 }: IconProps) {
  return (
    <img
      src={markUrl}
      width={size}
      height={size}
      alt=""
      aria-hidden="true"
      draggable={false}
      className="select-none"
    />
  )
}
