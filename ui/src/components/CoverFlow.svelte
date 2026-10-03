<script lang="ts">
  import { onMount, tick } from 'svelte'
  import { push } from 'svelte-spa-router'
  import Swiper from 'swiper'
  import { EffectCoverflow } from 'swiper/modules'
  import 'swiper/css'
  import 'swiper/css/effect-coverflow'
  import { fetchQueue, fileUrl, type QueueItem } from '../lib/api'
  import { splitMediaPath } from '../lib/pipeline'
  import { mediaHref } from '../lib/path'

  // Mode rule: no reuse with Stage/FilmStrip — this component owns its own
  // minimal pipeline (bootstrap / idle advance / edge rebuild), talking to
  // the server API only.
  const LIMIT = 5 // 2 left + center + 2 right
  const MID = Math.floor((LIMIT - 1) / 2)
  const IDLE_MS = 5_000
  const MAX_EMPTY_RETRIES = 30

  type Status = 'loading' | 'ready' | 'empty' | 'error'

  let rootEl = $state.raw<HTMLDivElement | null>(null)
  let status = $state<Status>('loading')
  let items = $state<QueueItem[]>([])
  let activeIdx = $state(MID)
  let error = $state('')

  let swiper: Swiper | null = null
  let swiperReady = false
  let cancelled = false
  let inFlight = false
  let rebuilding = false
  let idleTimer: ReturnType<typeof setTimeout> | undefined
  let retryTimer: ReturnType<typeof setTimeout> | undefined

  const current = $derived(items[activeIdx] ?? null)

  function resetIdle(): void {
    if (cancelled) return
    clearTimeout(idleTimer)
    idleTimer = setTimeout(() => {
      void advance()
    }, IDLE_MS)
  }

  async function fetchCentered(offset: number): Promise<boolean> {
    const resp = await fetchQueue(offset, LIMIT)
    if (cancelled || resp.images.length === 0) return false
    items = resp.images
    return true
  }

  function initSwiper(initial: number): void {
    if (rootEl === null || cancelled) return
    swiperReady = false
    swiper?.destroy(true)
    swiper = null
    swiper = new Swiper(rootEl, {
      modules: [EffectCoverflow],
      effect: 'coverflow',
      initialSlide: initial,
      slidesPerView: 3,
      centeredSlides: true,
      loop: false,
      grabCursor: true,
      coverflowEffect: {
        rotate: 50,
        stretch: 0,
        depth: 100,
        modifier: 1,
        slideShadows: false,
      },
      on: {
        slideChange(s) {
          if (cancelled || !swiperReady) return
          activeIdx = s.activeIndex
          // Back edge: re-center the window on the SAME photo so the user
          // can keep swiping; forward edge is advance()'s job.
          if (activeIdx === 0) void rebuildTo(0)
        },
        touchStart() {
          resetIdle()
        },
      },
    })
    activeIdx = swiper.activeIndex
    swiperReady = true
  }

  async function rebuildTo(idx: number): Promise<void> {
    const target = items[idx]
    if (target === undefined) return
    if (cancelled || inFlight || rebuilding) return
    rebuilding = true
    inFlight = true
    try {
      if (await fetchCentered(target.index)) {
        await tick()
        initSwiper(MID) // same photo, now at the middle slot
      }
    } catch (err: unknown) {
      console.warn('cover rebuild failed', err)
    } finally {
      inFlight = false
      rebuilding = false
      if (!cancelled) resetIdle()
    }
  }

  async function advance(): Promise<void> {
    if (cancelled) return
    if (inFlight || rebuilding) {
      resetIdle()
      return
    }
    if (items.length === 0) return
    if (activeIdx >= items.length - 1) {
      // Forward edge: jump via a fresh window centered on the NEXT photo.
      const nextOffset = (current?.index ?? 0) + 1
      inFlight = true
      try {
        if (await fetchCentered(nextOffset)) {
          await tick()
          initSwiper(MID)
        }
      } catch (err: unknown) {
        console.warn('cover advance failed', err)
      } finally {
        inFlight = false
        if (!cancelled) resetIdle()
      }
    } else {
      swiper?.slideNext()
      resetIdle()
    }
  }

  function onSlideClick(i: number): void {
    resetIdle()
    const item = items[i]
    if (item === undefined) return
    if (i === activeIdx) {
      const { dir, file } = splitMediaPath(item.path)
      push(mediaHref(dir, file))
    } else {
      swiper?.slideTo(i)
    }
  }

  async function bootstrap(): Promise<void> {
    const offset0 = Math.floor(Math.random() * 1_000_000)
    let attempts = 0
    for (;;) {
      if (cancelled) return
      try {
        const resp = await fetchQueue(offset0, LIMIT)
        if (cancelled) return
        if (resp.images.length > 0) {
          items = resp.images
          activeIdx = MID
          status = 'ready'
          await tick()
          initSwiper(MID)
          resetIdle()
          return
        }
        if (resp.done) {
          status = 'empty'
          return
        }
        attempts += 1
        if (attempts > MAX_EMPTY_RETRIES) {
          status = 'error'
          error = '扫描超时'
          return
        }
        await new Promise<void>((resolve) => {
          retryTimer = setTimeout(resolve, 1000)
        })
      } catch (err: unknown) {
        if (cancelled) return
        status = 'error'
        error = err instanceof Error ? err.message : String(err)
        return
      }
    }
  }

  onMount(() => {
    const onActivity = (): void => {
      resetIdle()
    }
    document.addEventListener('mousemove', onActivity, { passive: true })
    document.addEventListener('click', onActivity, { passive: true })
    void bootstrap()
    return () => {
      cancelled = true
      document.removeEventListener('mousemove', onActivity)
      document.removeEventListener('click', onActivity)
      clearTimeout(idleTimer)
      clearTimeout(retryTimer)
      swiper?.destroy(true)
      swiper = null
    }
  })
</script>

<div class="flex min-h-0 flex-1 flex-col items-center justify-center gap-3" data-testid="coverflow">
  {#if status === 'loading'}
    <span class="loading loading-dots"></span>
  {:else if status === 'empty'}
    <span class="text-sm opacity-60">暂无照片</span>
  {:else if status === 'error'}
    <div class="alert alert-error py-2 text-sm" role="alert">
      <span>照片列表加载失败：{error}</span>
    </div>
  {:else if status === 'ready'}
    <div class="w-full max-w-4xl" bind:this={rootEl} data-testid="coverflow-swiper">
      <div class="swiper-wrapper">
        {#each items as item, i (i)}
          <div class="swiper-slide" data-testid="cover-slide">
            <button
              type="button"
              class="block h-60 w-full cursor-pointer"
              aria-label={item.path}
              onclick={() => onSlideClick(i)}
            >
              <div
                class="h-full w-full p-2 {i === activeIdx
                  ? 'bg-white ring-1 ring-base-300 shadow-[0_12px_40px_rgba(0,0,0,0.55)]'
                  : ''}"
              >
                <img
                  src={fileUrl('', item.path)}
                  alt=""
                  class="h-full w-full object-contain"
                />
              </div>
            </button>
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>