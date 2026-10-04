<script lang="ts">
  import { fetchDirThumbs, fileUrl } from '../lib/api'
  import { childHref, toSegments } from '../lib/path'

  let { name, parentPath, frameWidth }: {
    name: string
    parentPath: string
    frameWidth: number
  } = $props()

  let thumbs = $state<string[]>([])

  const folderRel = $derived([...toSegments(parentPath), name].join('/'))
  const hasThumbs = $derived(thumbs.length > 0)
  const gridClass = $derived(
    thumbs.length === 2
      ? 'grid grid-cols-2 gap-[2px]'
      : 'grid grid-cols-2 grid-rows-2 gap-[2px]',
  )

  function reveal(event: Event): void {
    ;(event.currentTarget as HTMLImageElement).style.opacity = '1'
  }

  $effect(() => {
    let cancelled = false
    void fetchDirThumbs(folderRel, 4)
      .then((images) => {
        if (!cancelled) thumbs = images
      })
      .catch(() => {
        /* 失败保持 [] → 文件夹图标回退 */
      })
    return () => {
      cancelled = true
    }
  })
</script>

<a
  href={childHref(parentPath, name)}
  class="group relative block h-full w-full overflow-hidden rounded-lg border border-base-300 bg-base-200 opacity-75 transition hover:bg-base-300 hover:opacity-100"
  style:aspect-ratio="4 / 3"
>
  {#if thumbs.length === 1}
    <img
      src={fileUrl(folderRel, thumbs[0], true)}
      alt=""
      aria-hidden="true"
      loading="lazy"
      decoding="async"
      class="absolute inset-0 h-full w-full object-cover opacity-0 transition-opacity duration-500"
      onload={reveal}
    />
  {:else if hasThumbs}
    <div class="absolute inset-0 {gridClass}">
      {#each thumbs.slice(0, 4) as image (image)}
        <img
          src={fileUrl(folderRel, image, true)}
          alt=""
          aria-hidden="true"
          loading="lazy"
          decoding="async"
          class="h-full w-full object-cover opacity-0 transition-opacity duration-500"
          onload={reveal}
        />
      {/each}
    </div>
  {/if}
  <div
    class="absolute inset-x-0 bottom-0 z-20 px-3 pb-3 pt-10 {hasThumbs
      ? 'bg-gradient-to-t from-black/75 via-black/40 to-transparent text-white'
      : ''}"
  >
    <div class="flex items-center gap-2 min-w-0">
      <svg
        xmlns="http://www.w3.org/2000/svg"
        class="h-5 w-5 shrink-0"
        viewBox="0 0 24 24"
        fill="currentColor"
        aria-hidden="true"
      >
        <path
          d="M3 6a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V6z"
        />
      </svg>
      <span class="truncate font-medium">{name}</span>
    </div>
  </div>
</a>