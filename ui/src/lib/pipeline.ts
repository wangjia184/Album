export const SLOT_W = 112
export const GAP = 8

export function thumbCapacity(viewportWidth: number): number {
  return Math.max(1, Math.floor((viewportWidth + GAP) / (SLOT_W + GAP)))
}

export function splitMediaPath(path: string): { dir: string; file: string } {
  const cut = path.lastIndexOf('/')
  if (cut < 0) return { dir: '', file: path }
  return { dir: path.slice(0, cut), file: path.slice(cut + 1) }
}