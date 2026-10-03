<script lang="ts">
  import { replace, router } from 'svelte-spa-router'
  import { listDir, type ListResponse } from '../lib/api'
  import { mediaHref, normalizePath } from '../lib/path'
  import Breadcrumb from '../components/Breadcrumb.svelte'
  import Gallery from '../components/Gallery.svelte'
  import Lightbox from '../components/Lightbox.svelte'

  let { params = {} }: { params?: Record<string, string | null> } = $props()

  const path = $derived(normalizePath(params.wild))

  const requestedMedia = $derived.by(() => {
    const qs = router.querystring
    const name = new URLSearchParams(qs || '').get('m') ?? null
    return name
  })

  let data = $state<ListResponse | null>(null)
  let loading = $state(true)
  let error = $state<string | null>(null)
  let lightboxIndex = $state<number | null>(null)
  let syncing = $state(false)

  const dirs = $derived(
    data === null
      ? []
      : data.children.filter((child) => child.kind === 'dir'),
  )
  const media = $derived(
    data === null
      ? []
      : data.children.filter(
          (child) => child.kind === 'image' || child.kind === 'video',
        ),
  )
  const empty = $derived(data !== null && dirs.length === 0 && media.length === 0)
  const rootName = $derived(data?.rootName ?? '')

  function findMediaIndex(name: string | null): number {
    if (name === null) return -1
    return media.findIndex((child) => child.name === name)
  }

  function syncUrl(name: string | null): void {
    if (syncing) return
    syncing = true
    replace(mediaHref(path, name))
    // replace() resolves after the next tick; clear flag after the router applies
    requestAnimationFrame(() => {
      syncing = false
    })
  }

  function openLightbox(index: number): void {
    lightboxIndex = index
    const item = media[index]
    if (item !== undefined) syncUrl(item.name)
  }

  function closeLightbox(): void {
    lightboxIndex = null
    syncUrl(null)
  }

  function navigateLightbox(delta: number): void {
    if (lightboxIndex === null) return
    const next = lightboxIndex + delta
    if (next >= 0 && next < media.length) {
      lightboxIndex = next
      const item = media[next]
      if (item !== undefined) syncUrl(item.name)
    }
  }

  // Load directory when path changes.
  $effect(() => {
    let cancelled = false
    loading = true
    error = null
    data = null
    lightboxIndex = null
    listDir(path).then(
      (res) => {
        if (cancelled) return
        data = res
        loading = false
      },
      (err: unknown) => {
        if (cancelled) return
        error = err instanceof Error ? err.message : String(err)
        loading = false
      },
    )
    return () => {
      cancelled = true
    }
  })

  // Reconcile `?m=` against the loaded media list.
  $effect(() => {
    void requestedMedia
    const list = data
    if (list === null) return
    const idx = findMediaIndex(requestedMedia)
    if (idx >= 0) {
      lightboxIndex = idx
    } else {
      lightboxIndex = null
    }
  })
</script>

<div class="flex flex-col gap-4">
  <Breadcrumb {path} {rootName} />

  {#if loading}
    <div class="flex justify-center py-8">
      <span class="loading loading-spinner loading-lg"></span>
    </div>
  {:else if error !== null}
    <div class="alert alert-error" role="alert">
      <span>加载失败：{error}</span>
    </div>
  {:else}
    {#if empty}
      <div class="alert alert-info" role="status">
        <span>空文件夹</span>
      </div>
    {/if}
    {#if !empty}
      <Gallery {dirs} {media} rel={path} onselect={openLightbox} />
    {/if}
  {/if}
</div>

{#if lightboxIndex !== null}
  <Lightbox
    items={media}
    index={lightboxIndex}
    rel={path}
    onclose={closeLightbox}
    onnavigate={navigateLightbox}
  />
{/if}