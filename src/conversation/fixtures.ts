import type { Packet, PacketObject, Placement } from './protocol'

/** Build a packet at a position, so tests read as transcripts rather than shapes. */
export function at(placement: Placement, obj: PacketObject): Packet {
  return { placement, obj }
}

/** Turn `turn`, tab 0 — the common case. */
export function turn(turn_index: number, obj: PacketObject): Packet {
  return at({ turn_index }, obj)
}
