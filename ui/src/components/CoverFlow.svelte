<script lang="ts">
  // CoverFlow — Step 1: static model verification.
  // Everything visible is a pure function of d = k - p (see spec):
  //   theta(d), tx(d) (registration-point docking), tz(d), scale(d).
  // No animation, no photos, no queue fetch — those come in later steps.
  // Render layer: CSS 3D (perspective + transform3d), one div per virtual slot.

  const THETA = 60 // deg, saturated rotation at |d| >= 1
  const M = 3 // slots each side -> 2M+1 = 7 virtual slots
  const P = 0 // playhead: static settled state (integer). Animation comes later.

  const SLOT_MIN = 240
  const SLOT_MAX = 560 // center maximize = adapt S to the stage, NOT scale(d):
                       // scaling the center square would break edge docking.

  let stageH = $state(0)
  const S = $derived(Math.max(SLOT_MIN, Math.min(SLOT_MAX, stageH - 40)))

  interface SlotXf {
    theta: number
    tx: number
    tz: number
  }

  function f(d: number, side: number): SlotXf {
    const rad = (deg: number): number => (deg * Math.PI) / 180
    const ad = Math.abs(d)
    const ae = Math.min(ad, 1)
    const theta = Math.sign(d) * ae * THETA
    const cosMax = Math.cos(rad(THETA))
    const cosCur = Math.cos(rad(ae * THETA))

    let tx: number
    if (ad <= 1) {
      // Dock: the inner registration point (edge mid) sits on the fold line
      // at x = ±S/2 of the center square, in the center plane (z = 0).
      tx = Math.sign(d) * (side / 2) * (1 + cosCur)
    } else {
      // Beyond the fold: continue the angled row edge-to-edge
      // (pitch = S * cos(THETA)) — tight, clipped by the stage (no terminus).
      tx = Math.sign(d) * ((side / 2) * (1 + cosMax) + (ad - 1) * side * cosMax)
    }
    const tz = -(side / 2) * Math.sin(rad(ae * THETA))
    return { theta, tx, tz }
  }

  const indices = $derived(
    Array.from({ length: 2 * M + 1 }, (_, i) => i - M),
  )
</script>

<div
  class="flex min-h-0 flex-1 flex-col items-center justify-center"
  data-testid="coverflow"
>
  <div
    class="cf-stage relative h-[min(600px,calc(100dvh-14rem))] w-full"
    bind:clientHeight={stageH}
    data-testid="cover-stage"
  >
    {#each indices as k (k)}
      {@const d = k - P}
      {@const t = f(d, S)}
      <div
        class="cf-slot flex items-center justify-center rounded-lg bg-base-300/50 ring-1 ring-base-100/30"
        data-cf-slot
        data-d={d}
        data-k={k}
        style:width="{S}px"
        style:height="{S}px"
        style:transform="translate(-50%, -50%) translate3d({t.tx}px, 0, {t.tz}px) rotateY({t.theta}deg)"
      >
        <span class="select-none text-center text-xs leading-4 opacity-70">
          k {k}<br />d {d}
        </span>
      </div>
    {/each}
  </div>
</div>