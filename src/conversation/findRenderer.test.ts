import { expect, test } from 'vitest'
import { DISPATCH, findRenderer } from './findRenderer'
import { groupPackets } from './grouping'
import { PacketType } from './protocol'
import { turn } from './fixtures'
import {
  BashToolRenderer,
  ErrorRenderer,
  MessageTextRenderer,
  ReasoningRenderer,
} from './renderers'

/** Route the one group these packets form. */
function route(...objs: Parameters<typeof turn>[1][]) {
  const [group] = groupPackets(objs.map((obj) => turn(0, obj)))
  return findRenderer(group!)
}

test('the answer wins a group it shares with a tool', () => {
  // Order is the whole behaviour of the table: a stray tool packet landing at
  // the message's position must not capture the turn's answer.
  expect(
    route(
      { type: PacketType.BASH_TOOL_START, cmd: 'ls' },
      { type: PacketType.MESSAGE_DELTA, content: 'done' },
    ),
  ).toBe(MessageTextRenderer)
})

test('reasoning yields to the work it accompanies', () => {
  expect(
    route(
      { type: PacketType.REASONING_DELTA, reasoning: 'consider' },
      { type: PacketType.BASH_TOOL_START, cmd: 'ls' },
    ),
  ).toBe(BashToolRenderer)
})

test('reasoning alone still renders', () => {
  expect(route({ type: PacketType.REASONING_START })).toBe(ReasoningRenderer)
})

test('a bare failure routes to the error renderer', () => {
  expect(route({ type: PacketType.ERROR, message: 'broke' })).toBe(ErrorRenderer)
})

test('a group of only control packets renders nothing', () => {
  // Returning a renderer here would put an empty row in the transcript for
  // every turn boundary.
  expect(
    route(
      { type: PacketType.STOP, stop_reason: 'finished' },
      { type: PacketType.SECTION_END },
      { type: PacketType.TOP_LEVEL_BRANCHING, num_parallel_branches: 2 },
    ),
  ).toBeNull()
})

test('no packet type is claimed by two renderers', () => {
  // A type in two entries would make the dispatch order decide something the
  // table does not say out loud.
  const seen = new Set<string>()
  for (const entry of DISPATCH) {
    for (const claim of entry.claims) {
      expect(seen.has(claim)).toBe(false)
      seen.add(claim)
    }
  }
})
