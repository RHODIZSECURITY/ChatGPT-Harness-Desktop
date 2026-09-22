import type { IconProps } from '@opal/types'

/**
 * The RHODIZ mark, in Opal's icon shape so it can be handed to
 * `SidebarLayouts.Header`'s `renderAppLogo`.
 *
 * The two colours are the ones decoded out of `src-tauri/icons/32x32.png`, so
 * the sidebar and the taskbar show the same mark rather than two that merely
 * look related. `currentColor` is deliberately not used: a brand mark that
 * inherits the text colour stops being the brand mark.
 */
export default function RhodizMark({ size = 28, ...props }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 32 32"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      {...props}
    >
      <circle cx="16" cy="16" r="13" stroke="#22cde3" strokeWidth="2.5" />
      <path d="M16 9v14" stroke="#ffc62f" strokeWidth="2.5" strokeLinecap="round" />
      <path d="M11 13.5h10" stroke="#22cde3" strokeWidth="2.5" strokeLinecap="round" />
    </svg>
  )
}
