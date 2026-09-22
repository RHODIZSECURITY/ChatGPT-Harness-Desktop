import { useCallback, useEffect, useState } from 'react'
import { Provider as TooltipProvider } from '@radix-ui/react-tooltip'
import { Button, LineItemButton, Text } from '@opal/components'
import { SvgCheckSquare, SvgCode, SvgColumn, SvgFiles, SvgTerminal } from '@opal/icons'
import { RootLayout, SidebarStateProvider } from '@opal/layouts'
import { RouterProvider } from './design/next-shim/navigation'
import Conversation from './shell/Conversation'
import HarnessSidebar from './shell/HarnessSidebar'
import {
  readPanelFlag,
  SIDEBAR_FOLDED_KEY,
  WORKSPACE_OPEN_KEY,
  writePanelFlag,
} from './shell/panelState'
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

/// The panel's open width. Kept here rather than in a class because the
/// closed state is `0` and Tailwind cannot transition between two utilities —
/// one property, animated, with both ends written in the same place.
const WORKSPACE_WIDTH = '15.5rem'

function Shell() {
  const [status, setStatus] = useState<RuntimeStatus | null>(null)
  const [error, setError] = useState<string | null>(null)
  // Read once, lazily: the first render must already be the right width, or
  // the panel animates itself shut on every launch.
  const [workspaceOpen, setWorkspaceOpen] = useState(() =>
    readPanelFlag(WORKSPACE_OPEN_KEY, true)
  )

  useEffect(() => {
    getRuntimeStatus().then(setStatus).catch(() => setError('Broker unavailable'))
  }, [])

  const toggleWorkspace = useCallback(() => {
    setWorkspaceOpen((open) => {
      writePanelFlag(WORKSPACE_OPEN_KEY, !open)
      return !open
    })
  }, [])
  const workspaceLabel = workspaceOpen ? 'Hide workspace panel' : 'Show workspace panel'

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
            <header className="flex shrink-0 items-center justify-between gap-4 border-b border-border-01 px-10 py-3">
              <Text as="h1" font="main-ui-body" color="text-04">
                Harness workspace
              </Text>
              {/* The panel's control sits in the transcript's own strip, not
                  inside the panel: a button that disappears with the thing it
                  reopens is a one-way door. Mirrors where the sidebar keeps
                  its fold button, at the other end of the same row. */}
              <Button
                icon={SvgColumn}
                prominence="tertiary"
                size="md"
                aria-label={workspaceLabel}
                tooltip={workspaceLabel}
                tooltipSide="bottom"
                onClick={toggleWorkspace}
              />
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
        {/* RootLayout's panel slot is `shrink-0` and takes its width from this
            child, so the width transition has to live here rather than on the
            slot. The content stays mounted: `display: none` cannot be
            animated, and unmounting would discard the panel's scroll position
            every time it is closed. `inert` is what takes the hidden content
            out of the tab order and the accessibility tree — `overflow-hidden`
            only stops it being seen. */}
        <div
          className="h-full overflow-hidden transition-[width] duration-200 ease-in-out motion-reduce:transition-none"
          style={{ width: workspaceOpen ? WORKSPACE_WIDTH : '0rem' }}
          inert={!workspaceOpen}
        >
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
        </div>
      </RootLayout.RightPanel>
    </RootLayout.Root>
  )
}

export default function App() {
  // The sidebar's fold state belongs to Opal's provider, which owns the
  // Cmd/Ctrl+E shortcut as well as the button. It offers exactly these two
  // hooks for persistence, so the shell supplies storage and nothing else.
  const [sidebarFolded] = useState(() => readPanelFlag(SIDEBAR_FOLDED_KEY, false))
  const persistSidebar = useCallback((folded: boolean) => {
    writePanelFlag(SIDEBAR_FOLDED_KEY, folded)
  }, [])

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
          <SidebarStateProvider
            defaultFolded={sidebarFolded}
            onFoldedChange={persistSidebar}
          >
            <Shell />
          </SidebarStateProvider>
        </RouterProvider>
      </TooltipProvider>
    </ThemeProvider>
  )
}
