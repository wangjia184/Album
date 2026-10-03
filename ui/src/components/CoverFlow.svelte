<script lang="ts">
  import { onMount } from 'svelte'
  // CoverFlow — Step 1: static model verification.
  // Everything visible is a pure function of d = k - p (see spec):
  //   phi(d), x/z on the circle segment, theta(d) radial.
  // No animation, no photos, no queue fetch — those come in later steps.
  // Render layer: CSS 3D (perspective + transform3d), one div per virtual slot.

  // CoverFlow arc model (spec): slots stand on a circle segment in the
  // ground plane, cover planes radial to the circle. Single variable d = k-p.
  //   phi  = d * DELTA
  //   x    = R * sin(phi),  z = -R * (1 - cos(phi))
  //   R    = (S/2) * cot(DELTA/2)   <- adjacent edge-mids coincide exactly
  //                                  (both dock equations collapse to this R)
  const POS_DELTA = 10 // deg per slot for POSITION — gentle curvature only
                       // (deflection ≈ (S/2)·sinΔ·STACK ≈ 40px: reads nearly
                       // straight like iPad CF, still recedes in depth)
  const WING_ANGLE = 38 // deg — ALL wings face the same moderate angle (iPad
                        // side covers show art, they are not edge-on slivers;
                        // uniform width + fixed pitch = natural overlap)
  const STACK = 0.55 // pull wing positions inward so each successive cover
                     // occludes part of the previous one (≈30px+ at the
                     // straight-line pitch)
  const THETA_CAP = 60 // deg safety — projection fold-back guard (unused at 38)
  const M = 3 // slots each side -> 2M+1 = 7 virtual slots
  // Playhead: the ONE animated state. Everything on screen is f(k - p).
  // Basic motion: ease p from n to n+1, rest, repeat. Drag/queue later.
  let p = $state(0)
  const MOVE_MS = 700
  const PAUSE_MS = 1800

  function easeInOutCubic(t: number): number {
    return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2
  }

  const SLOT_MIN = 240
  const SLOT_MAX = 660 // square side cap; also bounded by stage width - 80

  let stageH = $state(0)
  let stageW = $state(0)
  const S = $derived(
    Math.max(SLOT_MIN, Math.min(SLOT_MAX, stageH - 40, stageW - 80)),
  )

  interface SlotXf {
    theta: number // deg, = +phi (radial/normal alignment)
    tx: number
    tz: number
    phiDeg: number
  }

  function f(d: number, side: number): SlotXf {
    const rad = (deg: number): number => (deg * Math.PI) / 180
    const phi = rad(d * POS_DELTA)
    const R = side / 2 / Math.tan(rad(POS_DELTA) / 2) * STACK
    const theta = Math.sign(d) * Math.min(Math.abs(d), 1) * WING_ANGLE
    // Depth bias must scale with S AND stay continuous in d: wings parked at
    // |d|>=1 sit fully behind the center plane (inner edge z ≤ 0, no plane
    // intersection), while fractional d ramps smoothly — a step at d===0
    // would teleport cards 200+px in one frame mid-animation.
    const backFull = (side / 2) * Math.sin(rad(WING_ANGLE)) + 24
    const back = backFull * Math.min(Math.abs(d), 1)
    return {
      theta,
      tx: R * Math.sin(phi),
      tz: -R * (1 - Math.cos(phi)) - back,
      phiDeg: d * POS_DELTA,
    }
  }

  const indices = $derived(
    // Virtual slots follow the playhead: k = round(p) + i. At the mid-point
    // of a move the window shifts by one — only the two edge slivers
    // (|d| ≈ 3.5, clipped) are recycled; interior slots keep their k, so
    // their f(k-p) stays continuous through the hand-off.
    Array.from({ length: 2 * M + 1 }, (_, i) => Math.round(p) - M + i),
  )

  let moveRaf: number | null = null
  let loopTimer: ReturnType<typeof setTimeout> | undefined
  let animating = false
  let cancelled = false

  function startMove(): void {
    if (cancelled) return
    const from = Math.round(p)
    const to = from + 1
    const t0 = performance.now()
    animating = true
    const frame = (now: number): void => {
      if (cancelled) return
      const t = Math.min(1, (now - t0) / MOVE_MS)
      p = from + (to - from) * easeInOutCubic(t) // the only moving number
      if (t < 1) {
        moveRaf = requestAnimationFrame(frame)
      } else {
        p = to
        animating = false
        loopTimer = setTimeout(startMove, PAUSE_MS)
      }
    }
    moveRaf = requestAnimationFrame(frame)
  }

  // Stable pseudo-random hue per slot (golden angle) — slots stay
  // distinguishable across re-renders without any randomness state.
  function slotColor(k: number): string {
    const hue = ((k * 137.508) % 360 + 360) % 360
    return `hsl(${hue.toFixed(1)} 62% 42%)`
  }

  let stageEl = $state<HTMLDivElement | null>(null)

  // Pin the CLIP box to the VISUAL viewport: fixed positioning escapes main's
  // container max-width + overflow-x clip (wings must clip at the PAGE edge).
  // Top/height fill the space below the navbar. The clip box owns
  // overflow:hidden + perspective; the inner stage owns preserve-3d
  // (overflow on a preserve-3d element would force flattening).
  function pinStage(): void {
    if (stageEl === null) return
    const clip = stageEl.parentElement
    if (clip === null) return
    const main = document.querySelector('main')
    const mr = main?.getBoundingClientRect()
    const top = Math.round((mr?.top ?? 73) + 16)
    clip.style.top = `${top}px`
    clip.style.height = `${Math.max(320, window.innerHeight - top - 16)}px`
    clip.style.width = `${document.documentElement.clientWidth}px`
  }

  onMount(() => {
    pinStage()
    const onResize = (): void => pinStage()
    window.addEventListener('resize', onResize)
    // start the move loop after the initial rest
    loopTimer = setTimeout(startMove, PAUSE_MS)
    return () => {
      cancelled = true
      window.removeEventListener('resize', onResize)
      clearTimeout(loopTimer)
      if (moveRaf !== null) cancelAnimationFrame(moveRaf)
    }
  })
</script>

<div
  class="flex min-h-0 flex-1 flex-col justify-center"
  data-testid="coverflow"
>
  <div class="cf-clip fixed left-0">
    <div
      class="cf-stage"
      bind:this={stageEl}
      bind:clientHeight={stageH}
      bind:clientWidth={stageW}
      data-testid="cover-stage"
    >
    {#each indices as k (k)}
      {@const d = k - p}
      {@const t = f(d, S)}
      <div
        class="cf-slot rounded-lg border border-base-100/40"
        data-cf-slot
        data-d={d}
        data-k={k}
        data-theta={t.theta}
        style:background={slotColor(k)}
        style:width="{S}px"
        style:height="{S}px"
        style:margin-left="{-S / 2}px"
        style:margin-top="{-S / 2}px"
        style:transform="translate3d({t.tx}px, 0, {t.tz}px) rotateY({t.theta}deg)"
      >
        <!-- three registration points: center, left-edge mid, right-edge mid -->
        <span class="cf-point bg-info" style:left="50%" style:top="50%"></span>
        <span class="cf-point bg-accent" style:left="0" style:top="50%"></span>
        <span class="cf-point bg-accent" style:left="100%" style:top="50%"></span>
        <span
          class="pointer-events-none absolute inset-0 flex flex-col items-center justify-center text-center select-none"
        >
          <span class="text-sm font-semibold leading-4 text-white/95">d {d}</span>
          <span class="text-[10px] leading-3 text-white/70">k {k}</span>
        </span>
      </div>
    {/each}
    </div>
  </div>
</div>