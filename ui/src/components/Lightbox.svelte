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
  const pageUrl = $derived(window.location.href)
  const canPrev = $derived(index > 0)
  const canNext = $derived(index < items.length - 1)

  let infoOpen = $state(false)
  let copied = $state(false)
  let copyTimer: ReturnType<typeof setTimeout> | undefined

  async function copyLink(): Promise<void> {
    if (pageUrl === '') return
    let ok = false
    try {
      if (navigator.clipboard !== undefined && window.isSecureContext) {
        await navigator.clipboard.writeText(pageUrl)
        ok = true
      }
    } catch {
      ok = false
    }
    if (!ok) {
      // HTTP / non-secure contexts: fallback textarea + execCommand
      const ta = document.createElement('textarea')
      ta.value = pageUrl
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
      }, 2000)
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
      <span class="relative flex items-center">
        {#if copied}
          <span
            class="absolute top-full right-1/2 z-10 mt-2 flex items-center gap-1.5 whitespace-nowrap rounded-lg bg-base-100 px-2.5 py-1.5 text-sm shadow-lg ring-1 ring-base-300"
            role="status"
          >
            <span class="flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-success text-success-content">
              <Icon icon="ph:check-bold" class="h-3 w-3" />
            </span>
            Url Copied
          </span>
        {/if}
        <span class="tooltip tooltip-bottom" data-tip="复制链接">
          <button
            type="button"
            class="btn btn-circle btn-sm btn-ghost"
            aria-label="复制链接"
            onclick={copyLink}
          >
            <Icon icon="ph:link-simple" class="h-5 w-5" />
          </button>
        </span>
      </span>
      <span class="tooltip tooltip-bottom" data-tip={infoOpen ? '关闭信息' : '媒体信息'}>
        <button
          type="button"
          class="btn btn-circle btn-sm btn-ghost"
          aria-label={infoOpen ? '关闭信息面板' : '打开信息面板'}
          aria-pressed={infoOpen}
          onclick={() => (infoOpen = !infoOpen)}
        >
          <Icon icon="ph:list-bullets" class="h-5 w-5" />
        </button>
      </span>
      <span class="tooltip tooltip-bottom" data-tip="关闭">
        <button
          type="button"
          class="btn btn-circle btn-sm btn-ghost"
          aria-label="关闭"
          onclick={onclose}
        >
          <Icon icon="ph:x" class="h-5 w-5" />
        </button>
      </span>
    </header>

    <div class="flex min-h-0 flex-1">
      <div class="relative min-w-0 flex-1">
        {#if item.kind === 'video'}
          <VideoViewport {url} title={item.name} />
        {:else}
          <PhotoViewport {url} alt={item.name} />
        {/if}

        <span class="tooltip tooltip-right absolute left-2 top-1/2 -translate-y-1/2">
          <button
            type="button"
            class="btn btn-circle btn-sm btn-ghost"
            disabled={!canPrev}
            aria-label="上一张"
            onclick={() => onnavigate(-1)}
          >
            <Icon icon="ph:caret-left" class="h-5 w-5" />
          </button>
        </span>
        <span class="tooltip tooltip-left absolute right-2 top-1/2 -translate-y-1/2">
          <button
            type="button"
            class="btn btn-circle btn-sm btn-ghost"
            disabled={!canNext}
            aria-label="下一张"
            onclick={() => onnavigate(1)}
          >
            <Icon icon="ph:caret-right" class="h-5 w-5" />
          </button>
        </span>
      </div>

      {#if infoOpen}
        <MediaInfoPanel {item} {rel} />
      {/if}
    </div>
  </div>
{/if}