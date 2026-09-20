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

  expect(screen.getByRole('heading', { name: 'Harness workspace' })).toBeInTheDocument()
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
