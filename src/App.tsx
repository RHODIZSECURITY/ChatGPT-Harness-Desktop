import { useEffect, useState } from 'react'
import { Provider as TooltipProvider } from '@radix-ui/react-tooltip'
import { LineItemButton, Text } from '@opal/components'
import { SvgCheckSquare, SvgCode, SvgFiles, SvgTerminal } from '@opal/icons'
import { RootLayout, SidebarStateProvider } from '@opal/layouts'
import { RouterProvider } from './design/next-shim/navigation'
import Conversation from './shell/Conversation'
import HarnessSidebar from './shell/HarnessSidebar'
import { ThemeProvider } from './theme/ThemeProvider'
import { getRuntimeStatus } from './runtime/bridge'
import type { ComponentState, RuntimeStatus } from './runtime/types'

const COMPONENTS: Array<[keyof Omit<RuntimeStatus, 'platform'>, string]> = [
  ['wsl2', 'WSL2'],
  ['docker', 'Docker'],
  ['core', 'Harness Core'],
  ['route', 'RHODIZ MCP Route'],
  ['memory', 'RHODIZ Memory MCP'],
  ['providers', 'Providers'],
]

/// The status bar is the only live data in the shell, so its colour carries
/// meaning. `checking` is deliberately not a state the broker can return — it
/// is the gap before the first answer, and must not look like a verdict.
const STATE_TONE: Record<ComponentState | 'checking', string> = {
  ready: 'text-status-text-success-05',
  stopped: 'text-status-text-warning-05',
  missing: 'text-status-text-error-05',
  unavailable: 'text-text-02',
  checking: 'text-text-02',
}

const WORKSPACE = [
  { title: 'Files', icon: SvgFiles },
  { title: 'Diff', icon: SvgCode },
  { title: 'Terminal', icon: SvgTerminal },
  { title: 'Tests', icon: SvgCheckSquare },
]

function Shell() {
  const [status, setStatus] = useState<RuntimeStatus | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    getRuntimeStatus().then(setStatus).catch(() => setError('Broker unavailable'))
  }, [])

  return (
    <RootLayout.Root>
      <HarnessSidebar />

      <RootLayout.App>
        <RootLayout.MainContent>
          <div className="flex h-full flex-col">
            {/* A thin strip, not a page header. The transcript is what this
                window is for, and a permanent block of descriptive copy above
                every conversation takes the space the conversation should have
                — which is why neither of the tools this shell is modelled on
                has one.

                The heading survives the trim. `as` matters beyond styling:
                Text renders a span by default, and a workspace title that is
                not a heading is invisible to anyone navigating this window by
                landmark. What goes is the label and the subtitle: the status
                bar already states where runtime authority lives, and states it
                from the broker rather than from a sentence that cannot be
                wrong. */}
            <header className="flex shrink-0 items-center border-b border-border-01 px-10 py-3">
              <Text as="h1" font="main-ui-body" color="text-04">
                Harness workspace
              </Text>
            </header>
            <div className="min-h-0 flex-1">
              <Conversation core={status?.core.state ?? 'checking'} />
            </div>
          </div>
        </RootLayout.MainContent>

        <RootLayout.Footer>
          <div
            aria-label="Runtime status"
            // No `h-full`: Opal's footer is `shrink-0` with automatic height,
            // so a full-height child inside it cannot grow when the row wraps —
            // the second line renders past the bottom of a window that
            // RootLayout has locked to the viewport, and is simply clipped.
            className="flex flex-wrap items-center gap-x-5 gap-y-1.5 border-t border-border-01 px-5 py-2 font-figure-small-label"
          >
            <span className={error ? 'text-status-text-error-05' : 'text-text-03'}>
              {error ?? (status ? 'Broker: ' + status.platform : 'Checking broker…')}
            </span>
            {COMPONENTS.map(([key, label]) => {
              const state = status?.[key].state ?? 'checking'
              return (
                <span
                  key={key}
                  data-state={state}
                  className={
                    'before:mr-1.5 before:inline-block before:size-1.5 before:rounded-full ' +
                    'before:align-middle before:bg-current before:content-[""] ' +
                    STATE_TONE[state]
                  }
                >
                  {label}: {state}
                </span>
              )
            })}
          </div>
        </RootLayout.Footer>
      </RootLayout.App>

      <RootLayout.RightPanel>
        <div
          aria-label="Workspace details"
          className="h-full w-62 border-l border-border-01 px-3 py-6"
        >
          <div className="px-2 pb-2">
            <Text font="figure-small-label" color="text-03">
              Workspace
            </Text>
          </div>
          {WORKSPACE.map(({ title, icon }) => (
            <LineItemButton
              key={title}
              title={title}
              icon={icon}
              // The default preset is "headline", sized for a full-width list.
              // In a 248px panel it renders the icon at headline scale and the
              // row wraps, stacking the glyph above its own label.
              sizePreset="main-ui"
              disabled
            />
          ))}
        </div>
      </RootLayout.RightPanel>
    </RootLayout.Root>
  )
}

export default function App() {
  return (
    // Opal's Tooltip is Radix's, used directly and without a Provider of its
    // own, so mounting one is the consuming application's job — onyx does the
    // same in its own wrapper. Without it every component that can show a
    // tooltip throws on first render, including SidebarTab and LineItemButton.
    // ThemeProvider sits outermost because it writes to `<html>`, not to the
    // tree: everything below it renders against whichever token set is in
    // place, and nothing below it needs to know which one that is.
    <ThemeProvider>
      <TooltipProvider delayDuration={400}>
        <RouterProvider>
          <SidebarStateProvider>
            <Shell />
          </SidebarStateProvider>
        </RouterProvider>
      </TooltipProvider>
    </ThemeProvider>
  )
}
