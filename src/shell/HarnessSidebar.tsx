import { SidebarTab } from '@opal/components'
import { SvgFolder, SvgPlayCircle } from '@opal/icons'
import { SidebarLayouts } from '@opal/layouts'
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

export default function HarnessSidebar() {
  return (
    <SidebarLayouts.Root>
      <SidebarLayouts.Header renderAppLogo={() => RhodizMark} showLogoWhenFolded>
        <SidebarTab disabled>RHODIZ Harness</SidebarTab>
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
