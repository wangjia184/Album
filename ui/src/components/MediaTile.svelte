<script lang="ts">
  import type { ListedChild } from '../lib/api'

  let { item, url }: { item: ListedChild; url: string } = $props()
</script>

{#if item.kind === 'video'}
  <div class="bg-neutral text-neutral-content aspect-square rounded-lg">
    <div class="flex h-full flex-col items-center justify-center gap-2 p-2">
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
  </div>
{:else}
  <div class="bg-base-200 overflow-hidden rounded-lg">
    <img
      src={url}
      alt={item.name}
      loading="lazy"
      decoding="async"
      class="aspect-square h-full w-full object-cover"
    />
  </div>
{/if}