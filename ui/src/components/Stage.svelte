<script lang="ts">
  import { fileUrl } from '../lib/api'

  let { current, onopen }: { current: string | null; onopen: (path: string) => void } = $props()

  const FADE_MS = 400

  let shownSrc = $state<string | null>(null)
  let visible = $state(false)
  let failed = $state(false)
  let seq = 0

  $effect(() => {
    const url = current === null ? null : fileUrl('', current)
    seq += 1
    const mySeq = seq
    failed = false
    if (url === null) {
      shownSrc = null
      visible = false
      return
    }
    const pre = new Image()
    pre.onload = () => {
      if (mySeq !== seq) return
      // TODO(follow-up): same-aspect fancy transition
      if (shownSrc !== null) {
        visible = false
        setTimeout(() => {
          if (mySeq !== seq) return
          shownSrc = url
          visible = true
        }, FADE_MS)
      } else {
        shownSrc = url
        visible = true
      }
    }
    pre.onerror = () => {
      if (mySeq !== seq) return
      failed = true
    }
    pre.src = url
  })

  function openCurrent(): void {
    if (current !== null && !failed) onopen(current)
  }
</script>

<div class="flex min-h-0 flex-1 items-center justify-center overflow-hidden">
  {#if failed}
    <p class="text-sm opacity-50">图片加载失败</p>
  {:else if shownSrc !== null}
    <button
      type="button"
      class="flex h-full w-full cursor-pointer items-center justify-center"
      aria-label="打开这张照片"
      onclick={openCurrent}
    >
      <img
        data-testid="stage-img"
        src={shownSrc}
        alt=""
        class="max-h-full max-w-full object-contain transition-opacity duration-[400ms]"
        style:opacity={visible ? '1' : '0'}
      />
    </button>
  {/if}
</div>