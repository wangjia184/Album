<script lang="ts">
  import { fetchMeta, type ListedChild, type MediaMeta } from '../lib/api'

  let {
    item,
    rel,
  }: {
    item: ListedChild | null
    rel: string
  } = $props()

  let status = $state('idle')
  let meta = $state<MediaMeta | null | undefined>(null)

  function formatBytes(bytes: number | null | undefined): string {
    if (bytes === null || bytes === undefined) return '—'
    if (bytes < 1024) return `${bytes} B`
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
  }

  function formatDate(secs: number | null | undefined): string {
    if (secs === null || secs === undefined) return '—'
    const d = new Date(secs * 1000)
    return d.toLocaleString()
  }

  $effect(() => {
    const name = item?.name
    if (name === undefined || name === null) {
      status = 'idle'
      meta = null
      return
    }
    let cancelled = false
    status = 'loading'
    meta = null
    fetchMeta(rel, name).then(
      (res) => {
        if (cancelled) return
        meta = res
        status = 'ready'
      },
      () => {
        if (cancelled) return
        status = 'error'
      },
    )
    return () => {
      cancelled = true
    }
  })
</script>

<aside class="h-full w-72 shrink-0 overflow-y-auto border-l border-base-300 bg-base-100 p-4">
  <h2 class="mb-3 text-sm font-semibold opacity-70">媒体信息</h2>

  {#if status === 'idle'}
    <p class="text-xs opacity-50">未选择媒体。</p>
  {:else if status === 'loading'}
    <div class="flex justify-center py-6">
      <span class="loading loading-spinner"></span>
    </div>
  {:else if status === 'error'}
    <div class="alert alert-error py-2 text-xs" role="alert">
      <span>无法读取媒体信息</span>
    </div>
  {:else if status === 'ready' && meta !== null}
    {@const current = meta!}
    <dl class="space-y-2 text-sm">
      <div class="flex items-baseline justify-between gap-2">
        <dt class="shrink-0 opacity-70">文件名</dt>
        <dd class="truncate text-right">{current.name}</dd>
      </div>
      <div class="flex items-baseline justify-between gap-2">
        <dt class="shrink-0 opacity-70">格式</dt>
        <dd class="text-right">{current.format ?? '—'}</dd>
      </div>
      <div class="flex items-baseline justify-between gap-2">
        <dt class="shrink-0 opacity-70">大小</dt>
        <dd class="text-right">{formatBytes(current.size)}</dd>
      </div>
      <div class="flex items-baseline justify-between gap-2">
        <dt class="shrink-0 opacity-70">尺寸</dt>
        <dd class="text-right"
          >{current.width !== null && current.width !== undefined
            ? `${current.width} × ${current.height ?? '?'}`
            : '—'}</dd
        >
      </div>
      <div class="flex items-baseline justify-between gap-2">
        <dt class="shrink-0 opacity-70">修改时间</dt>
        <dd class="text-right">{formatDate(current.modified)}</dd>
      </div>
    </dl>

    <h3 class="mb-2 mt-5 text-sm font-semibold opacity-70">EXIF</h3>
    {#if current.exif === null || current.exif === undefined}
      <p class="text-xs opacity-50">无 EXIF 信息。</p>
    {:else}
      <dl class="space-y-2 text-sm">
        {#if current.exif.datetime}
          <div class="flex items-baseline justify-between gap-2">
            <dt class="shrink-0 opacity-70">拍摄时间</dt>
            <dd class="text-right">{current.exif.datetime}</dd>
          </div>
        {/if}
        {#if current.exif.make || current.exif.model}
          <div class="flex items-baseline justify-between gap-2">
            <dt class="shrink-0 opacity-70">相机</dt>
            <dd class="text-right">{[current.exif.make, current.exif.model].filter(Boolean).join(' ')}</dd>
          </div>
        {/if}
        {#if current.exif.fNumber}
          <div class="flex items-baseline justify-between gap-2">
            <dt class="shrink-0 opacity-70">光圈</dt>
            <dd class="text-right">f/{current.exif.fNumber}</dd>
          </div>
        {/if}
        {#if current.exif.exposure}
          <div class="flex items-baseline justify-between gap-2">
            <dt class="shrink-0 opacity-70">快门</dt>
            <dd class="text-right">{current.exif.exposure}s</dd>
          </div>
        {/if}
        {#if current.exif.iso}
          <div class="flex items-baseline justify-between gap-2">
            <dt class="shrink-0 opacity-70">ISO</dt>
            <dd class="text-right">{current.exif.iso}</dd>
          </div>
        {/if}
        {#if current.exif.focal}
          <div class="flex items-baseline justify-between gap-2">
            <dt class="shrink-0 opacity-70">焦距</dt>
            <dd class="text-right">{current.exif.focal}mm</dd>
          </div>
        {/if}
        {#if current.exif.lens}
          <div class="flex items-baseline justify-between gap-2">
            <dt class="shrink-0 opacity-70">镜头</dt>
            <dd class="text-right">{current.exif.lens}</dd>
          </div>
        {/if}
        {#if !current.exif.datetime && !current.exif.make && !current.exif.model && !current.exif.fNumber && !current.exif.exposure && !current.exif.iso && !current.exif.focal && !current.exif.lens}
          <p class="text-xs opacity-50">无可用 EXIF 字段。</p>
        {/if}
      </dl>
    {/if}
  {/if}
</aside>