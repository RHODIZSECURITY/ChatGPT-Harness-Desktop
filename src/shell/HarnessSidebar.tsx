import { SidebarTab, Text } from '@opal/components'
import { SvgFolder, SvgPlayCircle } from '@opal/icons'
import { SidebarLayouts, useSidebarFolded } from '@opal/layouts'
import RhodizMark from './RhodizMark'
import ThemeControl from './ThemeControl'

/// Surfaces the shell names but has nothing behind yet. Rendered through the
/// design system's own disabled state rather than dimmed by hand: a row that
/// merely looks quiet is indistinguishable from one that is broken, and
/// `disabled` is what suppresses the click and reaches assistive technology.
const PLANNED = [
  { label: 'Projects', icon: SvgFolder },
  { label: 'Sessions', icon: SvgPlayCircle },
]

/**
 * The product name under the logo.
 *
 * A component rather than inline markup, because `useSidebarFolded` reads the
 * context `SidebarRoot` installs — a hook called in `HarnessSidebar` itself
 * sits outside that provider and would always report `false`.
 *
 * Opal hides the sidebar *body* when folded but leaves the header's content
 * slot and the footer alone, so anything wider than the 3.25rem rail has to
 * remove itself. This used to be a permanently disabled `SidebarTab`, which
 * folded into an empty row: a tab collapses to its icon, and a product name
 * has none.
 */
function Wordmark() {
  const folded = useSidebarFolded()
  if (folded) return null
  return (
    <Text font="main-ui-action" color="text-04" wordWrap="whitespace-nowrap">
      RHODIZ Harness
    </Text>
  )
}

export default function HarnessSidebar() {
  return (
    // `foldable` is what turns on the fold button, the Cmd/Ctrl+E shortcut and
    // the width transition. Opal defaults it off, so without this the sidebar
    // collapses on a narrow window and nowhere else.
    <SidebarLayouts.Root foldable>
      <SidebarLayouts.Header renderAppLogo={() => RhodizMark} showLogoWhenFolded>
        <Wordmark />
      </SidebarLayouts.Header>

      <SidebarLayouts.Body scrollKey="harness-sidebar">
        <div aria-label="Projects and sessions">
          <SidebarLayouts.Section title="Projects and sessions" disabled>
            {PLANNED.map(({ label, icon }) => (
              <SidebarTab key={label} icon={icon} disabled>
                {label}
              </SidebarTab>
            ))}
          </SidebarLayouts.Section>
        </div>
      </SidebarLayouts.Body>
      {/* Opal ships this footer and nothing here had used it. The theme is a
          window-level preference, not a piece of the conversation, so it sits
          at the bottom of the chrome rather than in the transcript's header —
          where both of the tools this shell is modelled on put it. */}
      <SidebarLayouts.Footer>
        <div className="pb-3">
          <ThemeControl />
        </div>
      </SidebarLayouts.Footer>
    </SidebarLayouts.Root>
  )
}
