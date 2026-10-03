<script lang="ts">
  import type { ListedChild } from '../lib/api'

  let {
    item,
    url,
    class: className = '',
    onselect,
    onload,
  }: {
    item: ListedChild
    url: string
    class?: string
    onselect: () => void
    onload: (width: number, height: number) => void
  } = $props()

  let loaded = $state(false)
  let imgEl: HTMLImageElement | undefined = $state()

  function handleLoad(event: Event): void {
    const img = event.currentTarget as HTMLImageElement
    loaded = true
    onload(img.naturalWidth, img.naturalHeight)
  }

  $effect(() => {
    void url
    loaded = false
  })

  $effect(() => {
    const el = imgEl
    if (el?.complete && el.naturalWidth > 0) {
      loaded = true
      onload(el.naturalWidth, el.naturalHeight)
    }
  })
</script>

<button
  type="button"
  class="group relative block h-full w-full overflow-hidden rounded-lg text-left focus:outline-none focus-visible:ring-2 focus-visible:ring-primary {className}"
  onclick={onselect}
  aria-label="打开 {item.name}"
>
  {#if item.kind === 'video'}
    <div class="bg-neutral text-neutral-content flex h-full w-full flex-col items-center justify-center gap-2 p-2 opacity-50 transition-opacity group-hover:opacity-100">
      <svg
        xmlns="http://www.w3.org/2000/svg"
        class="h-10 w-10"
        viewBox="0 0 24 24"
        fill="currentColor"
        aria-hidden="true"
      >
        <path d="M8 5v14l11-7L8 5z" />
      </svg>
      <span class="line-clamp-2 text-center text-xs opacity-80"
        >{item.name}</span
      >
    </div>
  {:else}
    <div class="bg-base-200 relative h-full w-full">
      {#if !loaded}
        <div class="absolute inset-0 flex items-center justify-center">
          <span class="loading loading-spinner"></span>
        </div>
      {/if}
      <img
        bind:this={imgEl}
        src={url}
        alt={item.name}
        loading="lazy"
        decoding="async"
        class="h-full w-full object-cover transition-opacity duration-300 group-hover:scale-[1.02] {loaded
          ? 'opacity-50 group-hover:opacity-100'
          : 'opacity-0'}"
        onload={handleLoad}
      />
    </div>
  {/if}
</button>