<script lang="ts">
  let {
    onzoomin,
    onzoomout,
    ononeone,
    onautofit,
  }: {
    onzoomin: () => void
    onzoomout: () => void
    ononeone: () => void
    onautofit: () => void
  } = $props()
</script>

<div class="flex items-center justify-center gap-2">
  <button
    type="button"
    class="btn btn-circle btn-sm btn-ghost"
    aria-label="放大"
    title="放大"
    onclick={onzoomin}
  >
    <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
      <circle cx="11" cy="11" r="7" />
      <path d="M21 21l-4.3-4.3M11 8v6M8 11h6" />
    </svg>
  </button>
  <button
    type="button"
    class="btn btn-circle btn-sm btn-ghost"
    aria-label="缩小"
    title="缩小"
    onclick={onzoomout}
  >
    <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
      <circle cx="11" cy="11" r="7" />
      <path d="M21 21l-4.3-4.3M8 11h6" />
    </svg>
  </button>
  <button
    type="button"
    class="btn btn-sm btn-ghost"
    aria-label="1:1"
    title="1:1 原始尺寸"
    onclick={ononeone}
  >
    1:1
  </button>
  <button
    type="button"
    class="btn btn-sm btn-ghost"
    aria-label="AutoFit"
    title="自适应视口"
    onclick={onautofit}
  >
    AutoFit
  </button>
</div>