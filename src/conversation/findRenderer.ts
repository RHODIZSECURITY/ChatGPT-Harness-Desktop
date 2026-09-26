import type { ReactElement } from 'react'
import type { PacketGroup } from './grouping'
import { type Packet, PacketType, isOneOf } from './protocol'
import {
  BashToolRenderer,
  CodingAgentRenderer,
  CustomToolRenderer,
  ErrorRenderer,
  FileReaderRenderer,
  MessageTextRenderer,
  ReasoningRenderer,
  type RendererProps,
} from './renderers'

export type PacketRenderer = (props: RendererProps) => ReactElement | null

/**
 * A dispatch entry: the packet types that claim a group, and who renders it.
 *
 * Adapted from onyx-foss's `findRenderer`
 * (`web/src/app/app/message/messageComponents/renderMessageComponent.tsx`,
 * MIT), which spells the same decision out as a run of `if` statements. A
 * table instead, because the order is the behaviour: it is the only thing
 * deciding a mixed group, so it should be a value a test can read rather than
 * control flow a test has to re-derive.
 */
interface DispatchEntry {
  claims: readonly PacketType[]
  render: PacketRenderer
}

/**
 * Ordered most specific to most general.
 *
 * Message text comes first, as it does upstream: the answer is what the turn
 * is for, and a stray tool packet sharing its position must not capture it.
 * The composite agents follow, because their groups legitimately contain
 * other tools' packets. Reasoning comes last of the content renderers since
 * it accompanies work rather than replacing it, and a bare failure last of
 * all.
 */
export const DISPATCH: readonly DispatchEntry[] = [
  {
    claims: [PacketType.MESSAGE_START, PacketType.MESSAGE_DELTA, PacketType.MESSAGE_END],
    render: MessageTextRenderer,
  },
  {
    claims: [
      PacketType.CODING_AGENT_START,
      PacketType.CODING_AGENT_THINKING_DELTA,
      PacketType.CODING_AGENT_FINAL,
    ],
    render: CodingAgentRenderer,
  },
  {
    claims: [PacketType.BASH_TOOL_START, PacketType.BASH_TOOL_DELTA],
    render: BashToolRenderer,
  },
  {
    claims: [PacketType.FILE_READER_START, PacketType.FILE_READER_RESULT],
    render: FileReaderRenderer,
  },
  {
    claims: [
      PacketType.CUSTOM_TOOL_START,
      PacketType.CUSTOM_TOOL_ARGS,
      PacketType.CUSTOM_TOOL_DELTA,
    ],
    render: CustomToolRenderer,
  },
  {
    claims: [
      PacketType.REASONING_START,
      PacketType.REASONING_DELTA,
      PacketType.REASONING_DONE,
    ],
    render: ReasoningRenderer,
  },
  { claims: [PacketType.ERROR], render: ErrorRenderer },
]

/**
 * The renderer for a group, or `null` when it carries nothing to show.
 *
 * `null` is a real answer rather than a gap: a group of only control packets
 * — `stop`, `section_end`, `top_level_branching` — has no content, and
 * inventing an empty frame for it would put blank rows in the transcript.
 */
export function findRenderer(group: PacketGroup): PacketRenderer | null {
  for (const entry of DISPATCH) {
    if (group.packets.some((packet: Packet) => isOneOf(packet, entry.claims))) {
      return entry.render
    }
  }
  return null
}
