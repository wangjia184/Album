<script lang="ts">
  import Icon from '@iconify/svelte/dist/OfflineIcon.svelte'
  import { fileUrl, type ListedChild } from '../lib/api'
  import MediaInfoPanel from './MediaInfoPanel.svelte'
  import PhotoViewport from './PhotoViewport.svelte'
  import VideoViewport from './VideoViewport.svelte'

  let {
    items,
    index,
    rel,
    onclose,
    onnavigate,
  }: {
    items: ListedChild[]
    index: number
    rel: string
    onclose: () => void
    onnavigate: (delta: number) => void
  } = $props()

  const item = $derived(index >= 0 && index < items.length ? items[index] : null)
  const url = $derived(item === null ? '' : fileUrl(rel, item.name))
  const absoluteUrl = $derived(item === null ? '' : new URL(url, window.location.origin).href)
  const canPrev = $derived(index > 0)
  const canNext = $derived(index < items.length - 1)

  let infoOpen = $state(false)
  let copied = $state(false)
  let copyTimer: ReturnType<typeof setTimeout> | undefined

  async function copyLink(): Promise<void> {
    if (absoluteUrl === '') return
    let ok = false
    try {
      if (navigator.clipboard !== undefined && window.isSecureContext) {
        await navigator.clipboard.writeText(absoluteUrl)
        ok = true
      }
    } catch {
      ok = false
    }
    if (!ok) {
      // HTTP / non-secure contexts: fallback textarea + execCommand
      const ta = document.createElement('textarea')
      ta.value = absoluteUrl
      ta.setAttribute('readonly', '')
      ta.style.position = 'fixed'
      ta.style.left = '-9999px'
      document.body.appendChild(ta)
      ta.select()
      try {
        ok = document.execCommand('copy')
      } catch {
        ok = false
      }
      ta.remove()
    }
    copied = ok
    if (ok) {
      if (copyTimer !== undefined) clearTimeout(copyTimer)
      copyTimer = setTimeout(() => {
        copied = false
      }, 1000)
    }
  }

  $effect(() => {
    return () => {
      if (copyTimer !== undefined) clearTimeout(copyTimer)
    }
  })

  $effect(() => {
    void index
    copied = false
  })

  $effect(() => {
    const onKeyDown = (event: KeyboardEvent): void => {
      if (event.key === 'Escape') {
        event.preventDefault()
        onclose()
      } else if (event.key === 'ArrowLeft') {
        event.preventDefault()
        onnavigate(-1)
      } else if (event.key === 'ArrowRight') {
        event.preventDefault()
        onnavigate(1)
      }
    }
    window.addEventListener('keydown', onKeyDown)
    const previousOverflow = document.body.style.overflow
    document.body.style.overflow = 'hidden'
    return () => {
      window.removeEventListener('keydown', onKeyDown)
      document.body.style.overflow = previousOverflow
    }
  })
</script>

{#if item !== null}
  <div
    class="fixed inset-0 z-50 flex flex-col bg-base-100"
    role="dialog"
    aria-modal="true"
    aria-label={item.name}
  >
    <header class="flex shrink-0 items-center gap-1 border-b border-base-300 p-2">
      <span class="min-w-0 flex-1 truncate px-2 text-sm opacity-80">{item.name}</span>
      <span class="tooltip {copied ? 'tooltip-open' : ''}" data-tip="Url Copied">
        <button
          type="button"
          class="btn btn-circle btn-sm btn-ghost"
          aria-label="复制链接"
          title="复制链接"
          onclick={copyLink}
        >
          <Icon icon="ph:link-simple" class="h-5 w-5" />
        </button>
      </span>
      <button
        type="button"
        class="btn btn-circle btn-sm btn-ghost"
        aria-label={infoOpen ? '关闭信息面板' : '打开信息面板'}
        aria-pressed={infoOpen}
        title={infoOpen ? '关闭信息' : '媒体信息'}
        onclick={() => (infoOpen = !infoOpen)}
      >
        <Icon icon="ph:camera" class="h-5 w-5" />
      </button>
      <button
        type="button"
        class="btn btn-circle btn-sm btn-ghost"
        aria-label="关闭"
        title="关闭"
        onclick={onclose}
      >
        <Icon icon="ph:x" class="h-5 w-5" />
      </button>
    </header>

    <div class="flex min-h-0 flex-1">
      <div class="relative min-w-0 flex-1">
        {#if item.kind === 'video'}
          <VideoViewport {url} title={item.name} />
        {:else}
          <PhotoViewport {url} alt={item.name} />
        {/if}

        <button
          type="button"
          class="btn btn-circle btn-sm btn-ghost absolute left-2 top-1/2 -translate-y-1/2"
          disabled={!canPrev}
          aria-label="上一张"
          title="上一张"
          onclick={() => onnavigate(-1)}
        >
          <Icon icon="ph:caret-left" class="h-5 w-5" />
        </button>
        <button
          type="button"
          class="btn btn-circle btn-sm btn-ghost absolute right-2 top-1/2 -translate-y-1/2"
          disabled={!canNext}
          aria-label="下一张"
          title="下一张"
          onclick={() => onnavigate(1)}
        >
          <Icon icon="ph:caret-right" class="h-5 w-5" />
        </button>
      </div>

      {#if infoOpen}
        <MediaInfoPanel />
      {/if}
    </div>
  </div>
{/if}