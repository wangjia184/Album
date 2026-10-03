<script lang="ts">
  import Icon from '@iconify/svelte/dist/OfflineIcon.svelte'

  let {
    onzoomin,
    onzoomout,
    ononeone,
    onautofit,
    onrotateccw,
    onrotatecw,
    showRotate,
  }: {
    onzoomin: () => void
    onzoomout: () => void
    ononeone: () => void
    onautofit: () => void
    onrotateccw: () => void
    onrotatecw: () => void
    showRotate: boolean
  } = $props()
</script>

<div class="flex items-center justify-center gap-2">
  <span class="tooltip" data-tip="放大">
    <button
      type="button"
      class="btn btn-circle btn-sm btn-ghost"
      aria-label="放大"
      onclick={onzoomin}
    >
      <Icon icon="ph:magnifying-glass-plus" class="h-5 w-5" />
    </button>
  </span>
  <span class="tooltip" data-tip="缩小">
    <button
      type="button"
      class="btn btn-circle btn-sm btn-ghost"
      aria-label="缩小"
      onclick={onzoomout}
    >
      <Icon icon="ph:magnifying-glass-minus" class="h-5 w-5" />
    </button>
  </span>
  <span class="tooltip" data-tip="1:1 原始尺寸">
    <button
      type="button"
      class="btn btn-circle btn-sm btn-ghost"
      aria-label="1:1"
      onclick={ononeone}
    >
      <Icon icon="ph:selection" class="h-5 w-5" />
    </button>
  </span>
  <span class="tooltip" data-tip="自适应视口">
    <button
      type="button"
      class="btn btn-circle btn-sm btn-ghost"
      aria-label="AutoFit"
      onclick={onautofit}
    >
      <Icon icon="ph:frame-corners" class="h-5 w-5" />
    </button>
  </span>
  {#if showRotate}
    <span class="tooltip" data-tip="左旋 90°">
      <button
        type="button"
        class="btn btn-circle btn-sm btn-ghost"
        aria-label="左旋"
        onclick={onrotateccw}
      >
        <Icon icon="ph:arrow-counter-clockwise" class="h-5 w-5" />
      </button>
    </span>
    <span class="tooltip" data-tip="右旋 90°">
      <button
        type="button"
        class="btn btn-circle btn-sm btn-ghost"
        aria-label="右旋"
        onclick={onrotatecw}
      >
        <Icon icon="ph:arrow-clockwise" class="h-5 w-5" />
      </button>
    </span>
  {/if}
</div>