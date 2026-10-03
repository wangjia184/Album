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
  const DELTA = 28 // deg per slot — 3*DELTA=84 < 90 (no face-flip), pulls
                   // wings into the depth so d2/d3 stay on screen at 1280px
  const STACK = 0.86 // pull wing positions inward so each successive cover
                     // occludes part of the previous one — tuned so every
                     // layer keeps a visible band under perspective
  const THETA_CAP = 60 // deg — beyond ~60° a square's projection FOLDS BACK
                       // (outer edge lands inboard of inner edge) and far
                       // cards collapse onto each other; iPad CF never goes
                       // near edge-on. Position keeps DELTA; facing caps here.
  const M = 3 // slots each side -> 2M+1 = 7 virtual slots
  const P = 0 // playhead: static settled state (integer). Animation comes later.

  const SLOT_MIN = 240
  const SLOT_MAX = 560 // center maximize = adapt S to the stage, NOT scale(d):
                       // scaling the center square would break edge docking.

  let stageH = $state(0)
  const S = $derived(Math.max(SLOT_MIN, Math.min(SLOT_MAX, stageH - 40)))

  interface SlotXf {
    theta: number // deg, = +phi (radial/normal alignment)
    tx: number
    tz: number
    phiDeg: number
  }

  function f(d: number, side: number): SlotXf {
    const rad = (deg: number): number => (deg * Math.PI) / 180
    const phi = rad(d * DELTA)
    const R = side / 2 / Math.tan(rad(DELTA) / 2) * STACK
    const theta = Math.sign(d) * Math.min(Math.abs(d * DELTA), THETA_CAP)
    return {
      theta,
      tx: R * Math.sin(phi),
      tz: -R * (1 - Math.cos(phi)),
      phiDeg: d * DELTA,
    }
  }

  const indices = $derived(
    Array.from({ length: 2 * M + 1 }, (_, i) => i - M),
  )

  let stageEl = $state<HTMLDivElement | null>(null)

  // Pin the stage to the VISUAL viewport: 100vw includes classic scrollbars
  // (15px asymmetry), so set width/offset from measured layout instead.
  function pinStage(): void {
    if (stageEl === null) return
    const parentLeft = stageEl.parentElement?.getBoundingClientRect().left ?? 0
    stageEl.style.marginLeft = `${-parentLeft}px`
    stageEl.style.width = `${document.documentElement.clientWidth}px`
  }

  onMount(() => {
    pinStage()
    const onResize = (): void => pinStage()
    window.addEventListener('resize', onResize)
    // Parent geometry can settle after mount (scrollbar/fonts) — re-pin.
    let po: ResizeObserver | null = null
    if (stageEl !== null && stageEl.parentElement !== null) {
      po = new ResizeObserver(() => pinStage())
      po.observe(stageEl.parentElement)
    }
    return () => {
      window.removeEventListener('resize', onResize)
      po?.disconnect()
    }
  })
</script>

<div
  class="flex min-h-0 flex-1 flex-col justify-center"
  data-testid="coverflow"
>
  <div
    class="cf-stage relative h-[min(600px,calc(100dvh-14rem))]"
    bind:this={stageEl}
    bind:clientHeight={stageH}
    data-testid="cover-stage"
  >
    {#each indices as k (k)}
      {@const d = k - P}
      {@const t = f(d, S)}
      <div
        class="cf-slot rounded-lg {d === 0
          ? 'border-2 border-primary bg-primary'
          : Math.abs(d) <= 1
            ? 'border border-secondary bg-base-300'
            : 'border border-base-100/50 bg-base-300/95'}"
        data-cf-slot
        data-d={d}
        data-k={k}
        data-theta={t.theta}
        style:width="{S}px"
        style:height="{S}px"
        style:margin-left="{-S / 2}px"
        style:margin-top="{-S / 2}px"
        style:z-index={d === 0 ? 10 : Math.abs(d)}
        style:transform="translate3d({t.tx}px, 0, {t.tz}px) rotateY({t.theta}deg)"
      >
        <!-- three registration points: center, left-edge mid, right-edge mid -->
        <span class="cf-point bg-info" style:left="50%" style:top="50%"></span>
        <span class="cf-point bg-accent" style:left="0" style:top="50%"></span>
        <span class="cf-point bg-accent" style:left="100%" style:top="50%"></span>
        <span class="pointer-events-none absolute inset-0 flex flex-col items-center justify-center select-none text-center">
          <span class="text-sm font-semibold leading-4 opacity-90">d {d}</span>
          <span class="text-[10px] leading-3 opacity-50">k {k}</span>
        </span>
      </div>
    {/each}
  </div>
</div>