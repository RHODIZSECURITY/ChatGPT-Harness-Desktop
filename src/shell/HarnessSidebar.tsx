import { SidebarLayouts } from '@opal/layouts'
import RhodizMark from './RhodizMark'

/// Surfaces that exist but have nothing behind them yet. Rendered dim and
/// inert on purpose: a placeholder styled like a live control reads as broken,
/// one styled like nothing at all reads as missing.
const PLANNED = ['Projects', 'Sessions']

export default function HarnessSidebar() {
  return (
    <SidebarLayouts.Root>
      <SidebarLayouts.Header renderAppLogo={() => RhodizMark} showLogoWhenFolded>
        <span className="px-1 text-sm font-semibold text-text-05">RHODIZ Harness</span>
      </SidebarLayouts.Header>

      <SidebarLayouts.Body scrollKey="harness-sidebar">
        <div aria-label="Projects and sessions">
          <SidebarLayouts.Section title="Projects and sessions" disabled>
            <div className="flex flex-col gap-1">
              {PLANNED.map((item) => (
                <span key={item} className="px-2 py-1.5 text-sm text-text-02 select-none">
                  {item}
                </span>
              ))}
            </div>
          </SidebarLayouts.Section>
        </div>
      </SidebarLayouts.Body>

      <SidebarLayouts.Footer>
        <span className="px-2 text-[11px] text-text-02">Not wired yet.</span>
      </SidebarLayouts.Footer>
    </SidebarLayouts.Root>
  )
}
