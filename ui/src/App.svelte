<script lang="ts">
  import NavBar from './components/NavBar.svelte'
  import Router from 'svelte-spa-router'
  import { routes } from './routes'

  interface TwinkleStar {
    left: number
    top: number
    size: number
    dur: number
    delay: number
    min: number
  }
  interface Meteor {
    left: number
    top: number
    dur: number
    delay: number
    dx: number
    dy: number
    rot: number
    tail: number
  }

  let starsA = $state('')
  let starsB = $state('')
  let twinkles = $state<TwinkleStar[]>([])
  let meteors = $state<Meteor[]>([])

  function makeStars(count: number, size: number, r: () => number, oMin: number, oMax: number): string {
    const shadows: string[] = []
    for (let i = 0; i < count; i++) {
      const x = Math.round(r() * 2200)
      const y = Math.round(r() * 1400)
      const o = oMin + r() * (oMax - oMin)
      shadows.push(`${x}px ${y}px 0 ${size}px rgba(255,255,255,${o.toFixed(2)})`)
    }
    return shadows.join(',')
  }

  function makeTwinkles(r: () => number): TwinkleStar[] {
    const list: TwinkleStar[] = []
    for (let i = 0; i < 22; i++) {
      list.push({
        left: Math.round(r() * 94) + 1,
        top: Math.round(r() * 90) + 1,
        size: Math.round((1.6 + r() * 1.6) * 10) / 10,
        dur: Math.round((1.8 + r() * 1.8) * 10) / 10,
        delay: Math.round(r() * 400) / 100,
        min: Math.round((0.05 + r() * 0.2) * 100) / 100,
      })
    }
    return list
  }

  $effect(() => {
    starsA = makeStars(140, 0, Math.random, 0.2, 0.9)
    starsB = makeStars(90, 1, Math.random, 0.3, 1)
    twinkles = makeTwinkles(Math.random)
    meteors = [
      { left: 78, top: 8, dur: 6.5, delay: 6, dx: -520, dy: 540, rot: -46, tail: 200 },
      { left: 92, top: 22, dur: 8, delay: 21, dx: -700, dy: 380, rot: -28, tail: 260 },
      { left: 22, top: 4, dur: 7.5, delay: 36, dx: 640, dy: 430, rot: -146, tail: 230 },
      { left: 60, top: 30, dur: 7, delay: 52, dx: -360, dy: 680, rot: -62, tail: 170 },
    ]
  })
</script>

<div class="fixed inset-0 -z-10 overflow-hidden">
  <div class="bg-cosmos"></div>
  {#if starsA}
    <div class="star-layer star-a" style:box-shadow={starsA}></div>
  {/if}
  {#if starsB}
    <div class="star-layer star-b" style:box-shadow={starsB}></div>
  {/if}
  {#each twinkles as s}
    <div
      class="twinkle-star"
      style:left="{s.left}%"
      style:top="{s.top}%"
      style:width="{s.size}px"
      style:height="{s.size}px"
      style:--tw-dur="{s.dur}s"
      style:--tw-delay="{s.delay}s"
      style:--tw-min="{s.min}"
    ></div>
  {/each}
  {#each meteors as m}
    <div
      class="meteor"
      style:left="{m.left}%"
      style:top="{m.top}%"
      style:--me-dur="{m.dur}s"
      style:--me-delay="{m.delay}s"
      style:--me-dx="{m.dx}px"
      style:--me-dy="{m.dy}px"
      style:--me-rot="{m.rot}deg"
      style:--me-tail="{m.tail}px"
    ></div>
  {/each}
</div>

<div class="flex min-h-screen flex-col">
  <NavBar />
  <main class="container relative z-10 mx-auto flex-1 p-4">
    <Router {routes} />
  </main>
</div>