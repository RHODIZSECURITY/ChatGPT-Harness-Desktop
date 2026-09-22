import { expect, test } from 'vitest'
import { at, turn } from './fixtures'
import {
  groupError,
  groupPackets,
  isStreamComplete,
  reasoningContent,
  textContent,
} from './grouping'
import { PacketType } from './protocol'

test('tools that ran in parallel stay in separate groups', () => {
  // Same turn, different tab: two tools running at once. Merging them would
  // interleave one command's output into the other's.
  const groups = groupPackets([
    at({ turn_index: 1, tab_index: 1 }, { type: PacketType.BASH_TOOL_START, cmd: 'b' }),
    at({ turn_index: 1, tab_index: 0 }, { type: PacketType.BASH_TOOL_START, cmd: 'a' }),
  ])

  expect(groups).toHaveLength(2)
  expect(groups.map((group) => group.tab_index)).toEqual([0, 1])
})

test('groups are ordered by turn regardless of arrival order', () => {
  const groups = groupPackets([
    turn(2, { type: PacketType.MESSAGE_DELTA, content: 'second' }),
    turn(0, { type: PacketType.MESSAGE_DELTA, content: 'first' }),
  ])

  expect(groups.map((group) => group.turn_index)).toEqual([0, 2])
})

test('a missing tab_index is tab 0, not a group of its own', () => {
  const groups = groupPackets([
    turn(0, { type: PacketType.MESSAGE_DELTA, content: 'a' }),
    at({ turn_index: 0, tab_index: 0 }, { type: PacketType.MESSAGE_DELTA, content: 'b' }),
  ])

  expect(groups).toHaveLength(1)
})

test('deltas concatenate in arrival order, starting from the start packet', () => {
  // `message_start` carries whatever the backend had buffered when it opened
  // the message. Treating it as a marker drops the first words of the answer.
  const [group] = groupPackets([
    turn(0, { type: PacketType.MESSAGE_START, id: 'm1', content: 'Pro' }),
    turn(0, { type: PacketType.MESSAGE_DELTA, content: 'vision' }),
    turn(0, { type: PacketType.MESSAGE_DELTA, content: 'ing.' }),
    turn(0, { type: PacketType.MESSAGE_END }),
  ])

  expect(textContent(group!.packets)).toBe('Provisioning.')
})

test('reasoning concatenates separately from the answer', () => {
  const packets = [
    turn(0, { type: PacketType.REASONING_DELTA, reasoning: 'weigh ' }),
    turn(0, { type: PacketType.MESSAGE_DELTA, content: 'answer' }),
    turn(0, { type: PacketType.REASONING_DELTA, reasoning: 'options' }),
  ]

  expect(reasoningContent(packets)).toBe('weigh options')
  expect(textContent(packets)).toBe('answer')
})

test('a failure always yields a string, even with no message', () => {
  // A failure that renders as nothing is indistinguishable from success.
  expect(groupError([turn(0, { type: PacketType.ERROR })])).toMatch(/failed/)
  expect(groupError([turn(0, { type: PacketType.ERROR, message: 'no runtime' })])).toBe(
    'no runtime',
  )
  expect(groupError([turn(0, { type: PacketType.MESSAGE_END })])).toBeNull()
})

test('only stop ends the stream', () => {
  // `message_end` and `section_end` close their own scope. Reading either as
  // the end of the turn would stop the transcript mid-tool.
  expect(isStreamComplete([turn(0, { type: PacketType.MESSAGE_END })])).toBe(false)
  expect(isStreamComplete([turn(0, { type: PacketType.SECTION_END })])).toBe(false)
  expect(isStreamComplete([turn(0, { type: PacketType.STOP })])).toBe(true)
})
