import { type Packet, type PacketObject, PacketType, isOneOf } from './protocol'

/**
 * One renderable unit of the transcript: every packet that shares a position.
 *
 * Adapted from onyx-foss `groupPacketsByTurnIndex` in
 * `web/src/app/app/services/packetUtils.ts` (MIT).
 */
export interface PacketGroup {
  turn_index: number
  tab_index: number
  packets: Packet[]
}

/**
 * Group packets by `(turn_index, tab_index)`, ordered by turn and then by tab.
 *
 * Grouping rather than sorting is the point: a stream interleaves packets
 * from tools running at the same time, so a flat ordering would shuffle two
 * unrelated tool outputs into one another. Packets keep their arrival order
 * within a group, which is what makes the deltas concatenable.
 */
export function groupPackets(packets: Packet[]): PacketGroup[] {
  const groups = new Map<string, PacketGroup>()

  for (const packet of packets) {
    const turn_index = packet.placement.turn_index
    const tab_index = packet.placement.tab_index ?? 0
    const key = `${turn_index}-${tab_index}`
    let group = groups.get(key)
    if (group === undefined) {
      group = { turn_index, tab_index, packets: [] }
      groups.set(key, group)
    }
    group.packets.push(packet)
  }

  return Array.from(groups.values()).sort((a, b) =>
    a.turn_index !== b.turn_index
      ? a.turn_index - b.turn_index
      : a.tab_index - b.tab_index,
  )
}

/**
 * The assistant text a group carries, start and deltas concatenated in order.
 *
 * `message_start` may already carry content — the backend sends whatever it
 * had buffered when it opened the message — so it is part of the text, not a
 * marker to skip.
 */
export function textContent(packets: Packet[]): string {
  let text = ''
  for (const packet of packets) {
    if (isOneOf(packet, [PacketType.MESSAGE_START, PacketType.MESSAGE_DELTA])) {
      text += packet.obj.content
    }
  }
  return text
}

/**
 * The reasoning a group carries, concatenated in arrival order.
 */
export function reasoningContent(packets: Packet[]): string {
  let text = ''
  for (const packet of packets) {
    if (isOneOf(packet, [PacketType.REASONING_DELTA])) {
      text += packet.obj.reasoning
    }
  }
  return text
}

/**
 * Whether the stream has ended — by completion or by cancellation.
 *
 * `stop` is the only packet that means the turn is over. `message_end` and
 * `section_end` close their own scope and say nothing about the rest.
 */
export function isStreamComplete(packets: Packet[]): boolean {
  return packets.some((packet) => packet.obj.type === PacketType.STOP)
}

/**
 * The failure a group carries, if any.
 *
 * An `error` packet can arrive inside any group — the turn failed partway
 * through a tool, not instead of it — so every renderer asks this rather than
 * the dispatcher routing errors away from the work they interrupted. An
 * error with no message still returns a string: a failure that renders as
 * nothing is indistinguishable from success.
 */
export function groupError(packets: Packet[]): string | null {
  for (const packet of packets) {
    if (isOneOf(packet, [PacketType.ERROR])) {
      return packet.obj.message ?? 'The turn failed without reporting a reason.'
    }
  }
  return null
}

/**
 * Find the latest unresolved approval request in a packet stream, if any.
 *
 * An approval request requires human intervention and pauses transcript
 * advance until resolved.
 */
export function activeApprovalRequest(packets: Packet[]): Extract<PacketObject, { type: typeof PacketType.APPROVAL_REQUEST }> | null {
  for (let i = packets.length - 1; i >= 0; i--) {
    const packet = packets[i]
    if (packet && isOneOf(packet, [PacketType.APPROVAL_REQUEST])) {
      return packet.obj
    }
  }
  return null
}
