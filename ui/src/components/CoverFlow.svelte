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

  // Box geometry the adaptive scale is computed against (see CSS in app.css).
  const BOX_H = 384

  let ro: ResizeObserver | null = null

  // Adaptive maximize: active photo scales up to fill the available root
  // height (capped so it never clips through .swiper's overflow:hidden).
  $effect(() => {
    if (rootEl === null || cancelled) return
    const el = rootEl
    const apply = (): void => {
      const s = Math.min(1.5, Math.max(1, (el.clientHeight - 8) / BOX_H))
      el.style.setProperty('--cover-scale', String(Math.round(s * 100) / 100))
    }
    apply()
    ro = new ResizeObserver(apply)
    ro.observe(el)
    return () => {
      ro?.disconnect()
      ro = null
    }
  })

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
    swiper?.destroy(true, false)
    swiper = null
    swiper = new Swiper(rootEl, {
      modules: [EffectCoverflow],
      effect: 'coverflow',
      initialSlide: initial,
      // Fixed-width slides: the box never scales with the viewport, so side
      // photos stay close to the center and nothing overflows the container.
      slidesPerView: 'auto',
      centeredSlides: true,
      loop: false,
      grabCursor: true,
      coverflowEffect: {
        rotate: 60,
        stretch: 0,
        depth: 140,
        modifier: 1,
        slideShadows: false,
      },
      on: {
        slideChange(s) {
          if (cancelled || !swiperReady) return
          activeIdx = s.activeIndex
          // Either edge: re-center the window on the SAME photo so more
          // (future or past) queue slots become swipeable.
          if (activeIdx === 0 || activeIdx === items.length - 1) {
            void rebuildTo(activeIdx)
          }
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
      ro?.disconnect()
      ro = null
      swiper?.destroy(true, false)
      swiper = null
    }
  })
</script>

<div
  class="flex min-h-0 flex-1 flex-col items-center justify-center gap-3"
  data-testid="coverflow"
>
  {#if status === 'loading'}
    <span class="loading loading-dots"></span>
  {:else if status === 'empty'}
    <span class="text-sm opacity-60">暂无照片</span>
  {:else if status === 'error'}
    <div class="alert alert-error py-2 text-sm" role="alert">
      <span>照片列表加载失败：{error}</span>
    </div>
  {:else if status === 'ready'}
    <div
      class="swiper h-[min(600px,calc(100dvh-14rem))] w-full"
      bind:this={rootEl}
      data-testid="coverflow-swiper"
    >
      <div class="swiper-wrapper">
        {#each items as item, i (i)}
          <!-- width pinned via `.swiper .cover-slide` !important rule in app.css:
               inline style gets stripped by swiper.destroy's style cleanup and
               swiper.css `.swiper-slide{width:100%}` would otherwise win. -->
          <div class="swiper-slide cover-slide" data-testid="cover-slide">
            <button
              type="button"
              class="cover-btn block h-96 w-full cursor-pointer"
              aria-label={item.path}
              onclick={() => onSlideClick(i)}
            >
              <div class="h-full w-full overflow-hidden rounded-lg">
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