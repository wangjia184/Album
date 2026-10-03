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

export interface MediaExif {
  datetime?: string | null
  make?: string | null
  model?: string | null
  fNumber?: string | null
  exposure?: string | null
  iso?: string | null
  focal?: string | null
  lens?: string | null
  xResolution?: string | null
}

export interface MediaMeta {
  path: string
  name: string
  size?: number | null
  modified?: number | null
  format?: string | null
  width?: number | null
  height?: number | null
  exif?: MediaExif | null
}

function encodeSegments(rel: string): string {
  return rel
    .split('/')
    .filter((segment) => segment.length > 0)
    .map(encodeURIComponent)
    .join('/')
}

async function fetchJson(url: string): Promise<unknown> {
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
  return res.json()
}

export async function listDir(rel: string): Promise<ListResponse> {
  const encoded = encodeSegments(rel)
  const url = encoded.length > 0 ? `/api/fs/list/${encoded}` : '/api/fs/list'
  return (await fetchJson(url)) as ListResponse
}

export async function fetchMeta(rel: string, name: string): Promise<MediaMeta> {
  const parts = [...rel.split('/').filter((s) => s.length > 0), name]
    .map(encodeURIComponent)
    .join('/')
  return (await fetchJson(`/api/fs/meta/${parts}`)) as MediaMeta
}

export async function fetchDirThumbs(folderRel: string, n = 3): Promise<string[]> {
  const encoded = encodeSegments(folderRel)
  const body = (await fetchJson(
    `/api/fs/thumbs/${encoded}?n=${encodeURIComponent(String(n))}`,
  )) as { images?: unknown }
  if (!Array.isArray(body.images)) return []
  return body.images.filter((image): image is string => typeof image === 'string')
}

export function fileUrl(rel: string, name: string): string {
  const parts = [
    ...rel.split('/').filter((segment) => segment.length > 0),
    ...name.split('/').filter((segment) => segment.length > 0),
  ].map(encodeURIComponent)
  return `/api/fs/file/${parts.join('/')}`
}