import { useEffect, useState } from 'react'
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

/// Surfaces that exist in the shell but have nothing behind them yet. They are
/// rendered dim and inert on purpose: a placeholder styled like a live control
/// reads as broken, and one styled like nothing at all reads as missing.
const PLANNED = {
  'Projects and sessions': ['Projects', 'Sessions'],
  'Workspace details': ['Files', 'Diff', 'Terminal', 'Tests'],
} as const

/// The status bar is the only live data in the shell, so its colour carries
/// meaning. `checking` is deliberately not a state the broker can return — it
/// is the gap before the first answer, and it must not look like a verdict.
const STATE_TONE: Record<ComponentState | 'checking', string> = {
  ready: 'text-ok',
  stopped: 'text-warn',
  missing: 'text-bad',
  unavailable: 'text-content-dim',
  checking: 'text-content-dim',
}

function Rail({ label, heading }: { label: keyof typeof PLANNED; heading: string }) {
  return (
    <aside
      aria-label={label}
      className="flex flex-col gap-1 border-edge bg-surface-raised px-5 py-6"
    >
      <h2 className="mb-3 text-[11px] font-semibold tracking-[0.12em] text-content-muted uppercase">
        {heading}
      </h2>
      {PLANNED[label].map((item) => (
        <span
          key={item}
          className="rounded-md px-2 py-1.5 text-sm text-content-dim select-none"
        >
          {item}
        </span>
      ))}
      <span className="mt-auto pt-6 text-[11px] leading-relaxed text-content-dim">
        Not wired yet.
      </span>
    </aside>
  )
}

function App() {
  const [status, setStatus] = useState<RuntimeStatus | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    getRuntimeStatus().then(setStatus).catch(() => setError('Broker unavailable'))
  }, [])

  return (
    <div className="grid min-h-screen grid-cols-[232px_minmax(520px,1fr)_248px] grid-rows-[1fr_auto] bg-surface font-sans">
      <div className="col-span-3 flex items-center gap-2.5 border-b border-edge bg-surface-deep px-5 py-2.5">
        <span className="size-2 rounded-full bg-brand" aria-hidden="true" />
        <span className="text-[13px] font-semibold tracking-tight text-content">
          RHODIZ Harness
        </span>
      </div>

      <Rail label="Projects and sessions" heading="Projects and sessions" />

      <main className="border-x border-edge px-10 py-9">
        <header className="max-w-2xl">
          <p className="text-[11px] font-semibold tracking-[0.14em] text-brand uppercase">
            Windows Desktop
          </p>
          <h1 className="mt-2.5 text-[32px] leading-tight font-semibold tracking-tight text-content">
            Harness workspace
          </h1>
          <p className="mt-3 text-sm leading-relaxed text-content-muted">
            Local renderer. Runtime authority remains inside the managed WSL2 stack.
          </p>
        </header>

        <section
          aria-label="Conversation"
          className="mt-10 max-w-2xl rounded-xl border border-edge bg-surface-overlay p-6"
        >
          <h2 className="text-sm font-semibold text-content">Conversation</h2>
          <p className="mt-2 text-sm leading-relaxed text-content-muted">
            Connect a Project and open a coding Session to begin.
          </p>
        </section>
      </main>

      <Rail label="Workspace details" heading="Workspace" />

      <footer
        aria-label="Runtime status"
        className="col-span-3 flex flex-wrap items-center gap-x-5 gap-y-1.5 border-t border-edge bg-surface-deep px-5 py-2.5 text-[11px]"
      >
        <span className={error ? 'text-bad' : 'text-content-muted'}>
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
      </footer>
    </div>
  )
}

export default App
