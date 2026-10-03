export const SLOT_W = 112
export const GAP = 8

export function thumbCapacity(viewportWidth: number): number {
  return Math.max(1, Math.floor((viewportWidth + GAP) / (SLOT_W + GAP)))
}

export function windowSlots(
  history: string[],
  upcoming: string[],
  capacity: number,
): (string | null)[] {
  const mid = Math.floor((capacity - 1) / 2)
  const ext = [...history, ...upcoming]
  const start = history.length - 1 - mid
  const slots: (string | null)[] = []
  for (let i = 0; i < capacity; i++) {
    const idx = start + i
    slots.push(idx >= 0 && idx < ext.length ? ext[idx] : null)
  }
  return slots
}

export function splitMediaPath(path: string): { dir: string; file: string } {
  const cut = path.lastIndexOf('/')
  if (cut < 0) return { dir: '', file: path }
  return { dir: path.slice(0, cut), file: path.slice(cut + 1) }
}