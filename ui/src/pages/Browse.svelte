<script lang="ts">
  import { listDir, type ListResponse } from '../lib/api'
  import { normalizePath } from '../lib/path'
  import Breadcrumb from '../components/Breadcrumb.svelte'
  import Gallery from '../components/Gallery.svelte'
  import Lightbox from '../components/Lightbox.svelte'

  let { params = {} }: { params?: Record<string, string | null> } = $props()

  const path = $derived(normalizePath(params.wild))

  let data = $state<ListResponse | null>(null)
  let loading = $state(true)
  let error = $state<string | null>(null)
  let lightboxIndex = $state<number | null>(null)

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

  function openLightbox(index: number): void {
    lightboxIndex = index
  }

  function closeLightbox(): void {
    lightboxIndex = null
  }

  function navigateLightbox(delta: number): void {
    if (lightboxIndex === null) return
    const next = lightboxIndex + delta
    if (next >= 0 && next < media.length) lightboxIndex = next
  }

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