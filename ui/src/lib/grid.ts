export const GAP = 12
export const DESKTOP_TILE_TARGET = 240

export function columnCount(width: number): number {
  if (width <= 0) return 0
  if (width < 640) return 2
  if (width < 1024) return 3
  return Math.min(5, Math.max(4, Math.round(width / DESKTOP_TILE_TARGET)))
}

export function frameWidthPx(width: number, cols: number): number {
  if (width <= 0 || cols <= 0) return 0
  const usable = width - (cols - 1) * GAP
  return Math.max(1, Math.floor(usable / cols))
}