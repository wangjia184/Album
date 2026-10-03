export type ChildKind = 'dir' | 'image' | 'video' | 'other'

export interface ListedChild {
  name: string
  kind: ChildKind
}

export interface ListResponse {
  path: string
  rootName: string
  children: ListedChild[]
}

function encodeSegments(rel: string): string {
  return rel
    .split('/')
    .filter((segment) => segment.length > 0)
    .map(encodeURIComponent)
    .join('/')
}

export async function listDir(rel: string): Promise<ListResponse> {
  const encoded = encodeSegments(rel)
  const url = encoded.length > 0 ? `/api/fs/list/${encoded}` : '/api/fs/list'
  const res = await fetch(url)
  if (!res.ok) {
    let detail = ''
    try {
      const body: unknown = await res.json()
      if (typeof body === 'object' && body !== null && 'error' in body) {
        const code = (body as { error: unknown }).error
        if (typeof code === 'string') detail = ` ${code}`
      }
    } catch {
      detail = ''
    }
    throw new Error(`${res.status}${detail}`)
  }
  return (await res.json()) as ListResponse
}

export function fileUrl(rel: string, name: string): string {
  const parts = [
    ...rel.split('/').filter((segment) => segment.length > 0),
    name,
  ].map(encodeURIComponent)
  return `/api/fs/file/${parts.join('/')}`
}