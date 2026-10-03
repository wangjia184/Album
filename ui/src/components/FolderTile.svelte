<script lang="ts">
  import { childHref } from '../lib/path'

  let { name, parentPath, frameWidth }: {
    name: string
    parentPath: string
    frameWidth: number
  } = $props()
</script>

<a
  href={childHref(parentPath, name)}
  class="flex h-full w-full flex-col items-start justify-end gap-2 rounded-lg border border-base-300 bg-base-200 p-3 opacity-50 transition hover:bg-base-300 hover:opacity-100"
  style:aspect-ratio="4 / 3"
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
</a>