<script lang="ts">
  import { GAP } from '../lib/grid'
  import FolderTile from './FolderTile.svelte'

  let { dirs, rel, cols, frameWidth }: {
    dirs: { name: string }[]
    rel: string
    cols: number
    frameWidth: number
  } = $props()
</script>

<div
  class="grid"
  style:gap="{GAP}px"
  style:grid-template-columns="repeat({cols}, {frameWidth}px)"
>
  {#each dirs as dir (dir.name)}
    <FolderTile name={dir.name} parentPath={rel} {frameWidth} />
  {/each}
</div>