<script lang="ts">
  import { listDir, type ListResponse } from '../lib/api'
  import Breadcrumb from '../components/Breadcrumb.svelte'
  import FolderGrid from '../components/FolderGrid.svelte'
  import MediaList from '../components/MediaList.svelte'

  let { params = {} }: { params?: Record<string, string | null> } = $props()

  const path = $derived(normalizePath(params.wild))

  let data = $state<ListResponse | null>(null)
  let loading = $state(true)
  let error = $state<string | null>(null)

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

  function normalizePath(wild: string | null | undefined): string {
    if (typeof wild !== 'string') return ''
    return wild.split('/').filter((segment) => segment.length > 0).join('/')
  }

  $effect(() => {
    let cancelled = false
    loading = true
    error = null
    data = null
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
  <Breadcrumb {path} />

  {#if loading}
    <div class="flex justify-center py-8">
      <span class="loading loading-spinner loading-lg"></span>
    </div>
  {:else if error !== null}
    <div class="alert alert-error" role="alert">
      <span>加载失败：{error}</span>
    </div>
  {:else}
    {#if dirs.length > 0}
      <FolderGrid {dirs} {path} />
    {/if}
    {#if empty}
      <div class="alert alert-info" role="status">
        <span>空文件夹</span>
      </div>
    {/if}
    {#if media.length > 0}
      <MediaList items={media} rel={path} />
    {/if}
  {/if}
</div>