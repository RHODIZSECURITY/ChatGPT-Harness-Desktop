import { render, screen } from '@testing-library/react'
import { beforeEach, expect, test, vi } from 'vitest'

vi.mock('./runtime/bridge', () => ({ getRuntimeStatus: vi.fn() }))

import App from './App'
import { getRuntimeStatus } from './runtime/bridge'
import type { RuntimeStatus } from './runtime/types'

const ready: RuntimeStatus = {
  platform: 'windows',
  wsl2: { state: 'ready' },
  docker: { state: 'ready' },
  core: { state: 'ready' },
  route: { state: 'ready' },
  memory: { state: 'ready' },
  providers: { state: 'ready' },
}

beforeEach(() => vi.clearAllMocks())
test('renders the three desktop work areas and live runtime states', async () => {
  vi.mocked(getRuntimeStatus).mockResolvedValue(ready)
  render(<App />)

  // By role, not by text: Opal's Text renders a span unless told otherwise,
  // so this is what keeps the workspace title reachable by landmark rather
  // than merely looking like a title.
  expect(screen.getByRole('heading', { name: 'Harness workspace' })).toBeInTheDocument()

  // The sidebar and workspace rows are the design system's own disabled
  // controls now, not spans dimmed by hand, so unavailability is announced
  // rather than merely drawn.
  //
  // Asserted as aria-disabled and not as a disabled button on purpose: Opal
  // renders these as a div carrying `aria-disabled`, with no `role`. That is
  // weaker than a real control — a screen reader will not call it a button —
  // but it is what the library exposes, and a test that claimed otherwise
  // would be describing a component this project does not have.
  for (const label of ['Projects', 'Sessions', 'Files', 'Terminal']) {
    const row = screen.getByText(label).closest('[aria-disabled]')
    expect(row, `${label} is not rendered as a disabled control`).not.toBeNull()
    expect(row).toHaveAttribute('aria-disabled', 'true')
  }
  expect(screen.getByLabelText('Projects and sessions')).toBeInTheDocument()
  expect(screen.getByLabelText('Conversation')).toBeInTheDocument()
  expect(screen.getByLabelText('Workspace details')).toBeInTheDocument()
  expect(await screen.findByText('Broker: windows')).toBeInTheDocument()
  expect(screen.getByText('RHODIZ MCP Route: ready')).toBeInTheDocument()
  expect(screen.getByText('RHODIZ Memory MCP: ready')).toBeInTheDocument()
})

test('fails closed in the UI when the broker cannot be reached', async () => {
  vi.mocked(getRuntimeStatus).mockRejectedValue(new Error('offline'))
  render(<App />)
  expect(await screen.findByText('Broker unavailable')).toBeInTheDocument()
  expect(screen.getByText('Harness Core: checking')).toBeInTheDocument()
})
