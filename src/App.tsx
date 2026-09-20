import { useEffect, useState } from 'react'
import './App.css'
import { getRuntimeStatus } from './runtime/bridge'
import type { RuntimeStatus } from './runtime/types'

const COMPONENTS: Array<[keyof Omit<RuntimeStatus, 'platform'>, string]> = [
  ['wsl2', 'WSL2'],
  ['docker', 'Docker'],
  ['core', 'Harness Core'],
  ['route', 'RHODIZ MCP Route'],
  ['memory', 'RHODIZ Memory MCP'],
  ['providers', 'Providers'],
]

function App() {
  const [status, setStatus] = useState<RuntimeStatus | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    getRuntimeStatus().then(setStatus).catch(() => setError('Broker unavailable'))
  }, [])

  return (
    <div className="shell">
      <aside className="rail" aria-label="Projects and sessions">
        <strong>RHODIZ Harness</strong>
        <p>Projects</p>
        <p>Sessions</p>
      </aside>      <main className="workspace">
        <header>
          <p className="eyebrow">Windows Desktop</p>
          <h1>Harness workspace</h1>
          <p>Local renderer. Runtime authority remains inside the managed WSL2 stack.</p>
        </header>
        <section className="conversation" aria-label="Conversation">
          <h2>Conversation</h2>
          <p>Connect a Project and open a coding Session to begin.</p>
        </section>
      </main>
      <aside className="rail" aria-label="Workspace details">
        <h2>Workspace</h2>
        <p>Files</p><p>Diff</p><p>Terminal</p><p>Tests</p>
      </aside>
      <footer aria-label="Runtime status">
        <span>{error ?? (status ? 'Broker: ' + status.platform : 'Checking broker…')}</span>
        {COMPONENTS.map(([key, label]) => (
          <span key={key}>{label}: {status?.[key].state ?? 'checking'}</span>
        ))}
      </footer>
    </div>
  )
}

export default App