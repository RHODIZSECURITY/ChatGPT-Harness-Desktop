import { SidebarTab } from '@opal/components'
import { SvgFolder, SvgPlayCircle } from '@opal/icons'
import { SidebarLayouts } from '@opal/layouts'
import RhodizMark from './RhodizMark'

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
    </SidebarLayouts.Root>
  )
}
