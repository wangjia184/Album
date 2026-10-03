export function normalizePath(wild: string | null | undefined): string {
  if (typeof wild !== 'string') return ''
  return toSegments(wild).join('/')
}

export function toSegments(path: string): string[] {
  return path.split('/').filter((segment) => segment.length > 0)
}

export function albumHref(segments: string[]): string {
  if (segments.length === 0) return '#/album'
  return `#/album/${segments.map(encodeURIComponent).join('/')}`
}

export function prefixHref(segments: string[], lastIndex: number): string {
  return albumHref(segments.slice(0, lastIndex + 1))
}

export function childHref(parentPath: string, name: string): string {
  return albumHref([...toSegments(parentPath), name])
}