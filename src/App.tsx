import { useEffect, useState } from 'react'
import { RootLayout, SidebarStateProvider } from '@opal/layouts'
import { RouterProvider } from './design/next-shim/navigation'
import Conversation from './shell/Conversation'
import HarnessSidebar from './shell/HarnessSidebar'
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

const WORKSPACE = ['Files', 'Diff', 'Terminal', 'Tests']

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
            <header className="shrink-0 px-10 pt-9 pb-2">
              <p className="text-[11px] font-semibold tracking-[0.14em] text-action-text-link-05 uppercase">
                Windows Desktop
              </p>
              <h1 className="mt-2.5 text-[32px] leading-tight font-semibold tracking-tight text-text-05">
                Harness workspace
              </h1>
              <p className="mt-3 max-w-2xl text-sm leading-relaxed text-text-03">
                Local renderer. Runtime authority remains inside the managed WSL2 stack.
              </p>
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
            className="flex flex-wrap items-center gap-x-5 gap-y-1.5 border-t border-border-01 px-5 py-2 text-[11px]"
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
          className="h-full w-62 border-l border-border-01 px-5 py-6"
        >
          <h2 className="mb-3 text-[11px] font-semibold tracking-[0.12em] text-text-03 uppercase">
            Workspace
          </h2>
          <div className="flex flex-col gap-1">
            {WORKSPACE.map((item) => (
              <span key={item} className="px-2 py-1.5 text-sm text-text-02 select-none">
                {item}
              </span>
            ))}
            <span className="px-2 pt-6 text-[11px] text-text-02">Not wired yet.</span>
          </div>
        </div>
      </RootLayout.RightPanel>
    </RootLayout.Root>
  )
}

export default function App() {
  return (
    <RouterProvider>
      <SidebarStateProvider>
        <Shell />
      </SidebarStateProvider>
    </RouterProvider>
  )
}
