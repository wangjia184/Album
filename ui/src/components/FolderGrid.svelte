<script lang="ts">
  import type { ListedChild } from '../lib/api'

  let { dirs, path }: { dirs: ListedChild[]; path: string } = $props()

  function hrefFor(name: string): string {
    const rel = [...path.split('/').filter((s) => s.length > 0), name]
    return `#/album/${rel.map(encodeURIComponent).join('/')}`
  }
</script>

<div class="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4">
  {#each dirs as dir (dir.name)}
    <a
      href={hrefFor(dir.name)}
      class="card border border-base-300 bg-base-200 transition-colors hover:bg-base-300"
    >
      <div class="card-body p-4">
        <div class="flex items-center gap-2">
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
          <span class="truncate font-medium">{dir.name}</span>
        </div>
      </div>
    </a>
  {/each}
</div>