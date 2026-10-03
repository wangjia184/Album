import type { Action } from 'svelte/action'

export function useVisibleSlice<T>(items: () => T[], step = 30) {
  let count = $state(step)

  $effect(() => {
    items()
    count = step
  })

  function more(): void {
    const list = items()
    if (count < list.length) {
      count = Math.min(count + step, list.length)
    }
  }

  const sentinel: Action<HTMLElement> = (element) => {
    const observer = new IntersectionObserver((entries) => {
      if (entries.some((entry) => entry.isIntersecting)) more()
    })
    observer.observe(element)
    return {
      destroy() {
        observer.disconnect()
      },
    }
  }

  return {
    get count() {
      return count
    },
    get visible() {
      return items().slice(0, count)
    },
    get hasMore() {
      return count < items().length
    },
    sentinel,
  }
}