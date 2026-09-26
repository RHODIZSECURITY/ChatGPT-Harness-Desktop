import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { expect, test } from 'vitest'
import Link from './link'
import { RouterProvider, useRouter, usePathname } from './navigation'

function Probe() {
  const router = useRouter()
  return (
    <div>
      <output>{usePathname()}</output>
      <button onClick={() => router.push('/pushed')}>push</button>
      <button onClick={() => router.replace('/replaced')}>replace</button>
      <button onClick={() => router.back()}>back</button>
    </div>
  )
}

const at = () => screen.getByRole('status').textContent

test('push adds to the stack and back pops it', async () => {
  const user = userEvent.setup()
  render(
    <RouterProvider initialPath="/start">
      <Probe />
    </RouterProvider>,
  )
  expect(at()).toBe('/start')

  await user.click(screen.getByRole('button', { name: 'push' }))
  expect(at()).toBe('/pushed')

  await user.click(screen.getByRole('button', { name: 'back' }))
  expect(at()).toBe('/start')
})

test('back at the root is a no-op rather than an empty pathname', async () => {
  // The sidebar reads usePathname() on every render to decide which tab is
  // active. Popping the last entry would hand it undefined and blank the
  // highlight, so the floor is part of the contract and not an accident.
  const user = userEvent.setup()
  render(
    <RouterProvider initialPath="/only">
      <Probe />
    </RouterProvider>,
  )
  await user.click(screen.getByRole('button', { name: 'back' }))
  expect(at()).toBe('/only')
})

test('replace swaps the current entry instead of stacking on it', async () => {
  const user = userEvent.setup()
  render(
    <RouterProvider initialPath="/start">
      <Probe />
    </RouterProvider>,
  )
  await user.click(screen.getByRole('button', { name: 'push' }))
  await user.click(screen.getByRole('button', { name: 'replace' }))
  expect(at()).toBe('/replaced')

  // If replace had stacked, this would land on '/pushed'.
  await user.click(screen.getByRole('button', { name: 'back' }))
  expect(at()).toBe('/start')
})

test('the hooks are inert outside a provider rather than throwing', () => {
  // Opal calls these from presentational components. A test rendering one in
  // isolation must not have to know a router exists.
  render(<Probe />)
  expect(at()).toBe('/')
})

test('Link routes in memory and never lets the webview navigate', async () => {
  const user = userEvent.setup()
  render(
    <RouterProvider initialPath="/start">
      <Link href="/target">go</Link>
      <Probe />
    </RouterProvider>,
  )
  const link = screen.getByRole('link', { name: 'go' })
  // A real anchor: focus, middle-click and the accessibility tree all depend
  // on it being one, so the href has to be present and correct.
  expect(link).toHaveAttribute('href', '/target')

  await user.click(link)
  expect(at()).toBe('/target')
})

test('Link honours replace, and a handler that prevents default wins', async () => {
  const user = userEvent.setup()
  render(
    <RouterProvider initialPath="/start">
      <Link href="/swapped" replace>
        swap
      </Link>
      <Link href="/blocked" onClick={(event) => event.preventDefault()}>
        blocked
      </Link>
      <Probe />
    </RouterProvider>,
  )
  await user.click(screen.getByRole('link', { name: 'swap' }))
  expect(at()).toBe('/swapped')

  await user.click(screen.getByRole('link', { name: 'blocked' }))
  expect(at()).toBe('/swapped')
})

test('a modified click is left to the platform', async () => {
  const user = userEvent.setup()
  render(
    <RouterProvider initialPath="/start">
      <Link href="/target">go</Link>
      <Probe />
    </RouterProvider>,
  )
  await user.keyboard('{Control>}')
  await user.click(screen.getByRole('link', { name: 'go' }))
  await user.keyboard('{/Control}')
  expect(at()).toBe('/start')
})
