<script lang="ts">
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
  const canPrev = $derived(index > 0)
  const canNext = $derived(index < items.length - 1)

  let infoOpen = $state(false)

  $effect(() => {
    infoOpen = false
    void index
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
    <header class="flex shrink-0 items-center gap-2 border-b border-base-300 p-2">
      <span class="min-w-0 flex-1 truncate px-2 text-sm opacity-80">{item.name}</span>
      <button
        type="button"
        class="btn btn-circle btn-sm btn-ghost"
        aria-label={infoOpen ? '关闭信息面板' : '打开信息面板'}
        aria-pressed={infoOpen}
        onclick={() => (infoOpen = !infoOpen)}
      >
        <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
          <circle cx="12" cy="12" r="9" />
          <path d="M12 11v5M12 8h.01" />
        </svg>
      </button>
      <button
        type="button"
        class="btn btn-circle btn-sm btn-ghost"
        aria-label="关闭"
        onclick={onclose}
      >
        <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
          <path d="M18 6 6 18M6 6l12 12" />
        </svg>
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
          onclick={() => onnavigate(-1)}
        >
          ←
        </button>
        <button
          type="button"
          class="btn btn-circle btn-sm btn-ghost absolute right-2 top-1/2 -translate-y-1/2"
          disabled={!canNext}
          aria-label="下一张"
          onclick={() => onnavigate(1)}
        >
          →
        </button>
      </div>

      {#if infoOpen}
        <MediaInfoPanel />
      {/if}
    </div>
  </div>
{/if}