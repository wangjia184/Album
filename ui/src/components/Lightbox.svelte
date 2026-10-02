<script lang="ts">
  import { fileUrl, type ListedChild } from '../lib/api'

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
  <div class="modal modal-open" role="dialog" aria-modal="true" aria-label={item.name}>
    <div class="modal-box relative w-auto max-w-[96vw] p-4">
      <button
        type="button"
        class="btn btn-circle btn-sm btn-ghost absolute right-2 top-2 z-10"
        aria-label="关闭"
        onclick={onclose}
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          class="h-5 w-5"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          aria-hidden="true"
        >
          <path d="M18 6 6 18M6 6l12 12" />
        </svg>
      </button>

      <div class="flex items-center justify-center">
        {#if item.kind === 'video'}
          <video
            src={url}
            controls
            playsinline
            class="max-h-[75vh] max-w-full rounded-lg bg-black"
          ></video>
        {:else}
          <img
            src={url}
            alt={item.name}
            class="max-h-[75vh] max-w-full rounded-lg object-contain"
          />
        {/if}
      </div>

      <div class="mt-3 flex items-center gap-3">
        <button
          type="button"
          class="btn btn-sm"
          disabled={!canPrev}
          aria-label="上一张"
          onclick={() => onnavigate(-1)}
        >
          ←
        </button>
        <span class="min-w-0 flex-1 truncate text-center text-sm opacity-80"
          >{item.name}</span
        >
        <button
          type="button"
          class="btn btn-sm"
          disabled={!canNext}
          aria-label="下一张"
          onclick={() => onnavigate(1)}
        >
          →
        </button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button type="submit" aria-label="关闭" onclick={onclose}>close</button>
    </form>
  </div>
{/if}