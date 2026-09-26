import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, expect, test, vi } from 'vitest'

vi.mock('../runtime/bridge', () => ({ getRuntimeStatus: vi.fn() }))

import App from '../App'
import ComposerControlBelt from './ComposerControlBelt'
import { getRuntimeStatus } from '../runtime/bridge'
import type { RuntimeStatus } from '../runtime/types'

const ready: RuntimeStatus = {
  platform: 'windows',
  wsl2: { state: 'ready' },
  docker: { state: 'ready' },
  core: { state: 'ready' },
  route: { state: 'ready' },
  memory: { state: 'ready' },
  providers: { state: 'ready' },
}

beforeEach(() => {
  vi.clearAllMocks()
  localStorage.clear()
})

test('the belt fails closed to checking before the broker answers', () => {
  render(<ComposerControlBelt status={null} />)

  expect(screen.getByRole('group', { name: 'Composer controls' })).toBeInTheDocument()
  expect(screen.getByRole('group', { name: 'Runtime context' })).toBeInTheDocument()
  // No status yet is the gap before the first answer, not a verdict — every
  // indicator reads `checking`, never ready or unavailable.
  for (const label of ['Core', 'Route', 'Memory', 'Providers']) {
    const indicator = screen.getByText(`${label}: checking`)
    expect(indicator).toHaveAttribute('data-state', 'checking')
  }
})

test('the belt mirrors live broker states with the footer tone contract', () => {
  const status: RuntimeStatus = {
    ...ready,
    core: { state: 'ready' },
    route: { state: 'stopped', detail: 'route flap' },
    memory: { state: 'missing' },
    providers: { state: 'ready' },
  }
  render(<ComposerControlBelt status={status} />)

  expect(screen.getByText('Core: ready')).toHaveAttribute('data-state', 'ready')
  const route = screen.getByText('Route: stopped')
  expect(route).toHaveAttribute('data-state', 'stopped')
  // The broker's own words travel with the indicator: a state without its
  // detail is a verdict without its reason.
  expect(route).toHaveAttribute('title', 'route flap')
  expect(screen.getByText('Memory: missing')).toHaveAttribute('data-state', 'missing')
})

test('the selectors stay disabled on an honest placeholder until Core exposes options', () => {
  render(<ComposerControlBelt status={ready} />)

  // Disabled is the contract: Core does not publish a model or role list
  // over `runtime_status`, so inventing choices would be a UI that promises
  // what nothing will honour.
  const model = screen.getByRole('combobox', { name: 'Model' })
  const role = screen.getByRole('combobox', { name: 'Role' })
  expect(model).toBeDisabled()
  expect(role).toBeDisabled()
  expect(model).toHaveTextContent('Core default')
  expect(role).toHaveTextContent('Default')
})

test('exposed options enable the selectors, and only listed values pass through', async () => {
  const user = userEvent.setup()
  const onModelChange = vi.fn()
  render(
    <ComposerControlBelt
      status={ready}
      models={['opus', 'sonnet']}
      roles={['coder', 'reviewer']}
      selectedModel="opus"
      selectedRole="stale-role"
      onModelChange={onModelChange}
    />,
  )

  // The anchored value shows; the stale one falls back to the placeholder
  // instead of displaying a role nothing will honour.
  const model = screen.getByRole('combobox', { name: 'Model' })
  expect(model).toBeEnabled()
  expect(model).toHaveTextContent('opus')
  expect(screen.getByRole('combobox', { name: 'Role' })).toHaveTextContent('Default')

  await user.click(model)
  await user.click(screen.getByRole('option', { name: 'sonnet' }))
  expect(onModelChange).toHaveBeenCalledWith('sonnet')
})

test('the shell mounts the belt above the composer with live broker states', async () => {
  vi.mocked(getRuntimeStatus).mockResolvedValue(ready)
  render(<App />)

  expect(screen.getByRole('group', { name: 'Composer controls' })).toBeInTheDocument()
  expect(await screen.findByText('Core: ready')).toBeInTheDocument()
  // The belt names the same runtime the footer states in full: two strips,
  // one verdict. The footer keeps its own long labels; the belt abbreviates.
  expect(screen.getByText('RHODIZ MCP Route: ready')).toBeInTheDocument()
  expect(screen.getByText('Route: ready')).toBeInTheDocument()
})
