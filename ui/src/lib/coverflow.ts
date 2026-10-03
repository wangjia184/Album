/**
 * CoverFlow slot model — the single source of truth for slot geometry.
 *
 * Everything visible is a pure function of d = k - p (playhead model):
 *   slotTransform(d, S) -> { theta, tx, tz }
 *
 * All lengths are ratios of the square side S — no fixed pixels — so the
 * model stays consistent at any stage size (SLOT_MIN..SLOT_MAX in the view).
 *
 * Invariants (verified by the fine-grid point-in-both-quads oracle,
 * sampling step H/12 and p×80 / H/16 × p×120):
 *  - slotTransform is continuous in d (lane pitch, depth bias, prox tent);
 *  - presets below are collision-free (0 interpenetrations) at S=660 and
 *    scale linearly with S;
 *  - center card bulges toward the camera at d=0 (PROX_RATIO·S ≈ +16%
 *    perspective enlargement), decays with cos²(πd/2), exactly 0 at |d|≥1.
 *
 * Spacing knobs (the only two tunables):
 *  - GAP_C: center-adjacent lane pitch (×S). Smaller → tighter seam between
 *    n and n±1, but demands a larger GAP_W to stay collision-free.
 *  - GAP_W: wing lane pitch (×S). Smaller → stronger wing-on-wing cover.
 *  Trade-off: small seam AND strong screen cover cannot coexist under the
 *    collision constraint — pick a preset.
 */

export interface SlotXf {
  /** deg */
  theta: number
  /** px along the row (derived from S) */
  tx: number
  /** px depth (derived from S) */
  tz: number
}

export const MODEL = {
  POS_DELTA_DEG: 10,
  WING_ANGLE_DEG: 38,
  /** center-adjacent lane pitch, ×S — the seam knob */
  GAP_C: 1.1,
  /** wing lane pitch, ×S — the overlap knob */
  GAP_W: 0.65,
  /** resting-center proximity bulge, ×S (220/660 ≈ 0.333) */
  PROX_RATIO: 220 / 660,
  /** extra depth keeping wing inner edge behind the center plane, ×S (24/660) */
  BACK_SLACK_RATIO: 24 / 660,
  /** depth-curve weight for the z lane (shape only, dimensionless) */
  Z_CURVE: 0.55,
} as const

/** Fine-grid verified presets (H/12 × p80 oracle: 0 collisions @ S=660). */
export const PRESETS = {
  /** parent seam ≈ 0.14·S (screen ≈66px @660), wing overlap ≈ 0.14·S, 0 collisions */
  tightSeam: { GAP_C: 1.1, GAP_W: 0.65 },
  /** seam ≈ 0.31·S (~202px @660), visible screen cover on wings */
  wingCover: { GAP_C: 1.2, GAP_W: 0.5 },
} as const

export type PresetName = keyof typeof PRESETS

/** Apply a preset onto MODEL (the component uses one at init). */
export function applyPreset(name: PresetName): void {
  const p = PRESETS[name]
  ;(MODEL as { GAP_C: number; GAP_W: number }).GAP_C = p.GAP_C
  ;(MODEL as { GAP_C: number; GAP_W: number }).GAP_W = p.GAP_W
}

export function slotTransform(d: number, S: number): SlotXf {
  const rad = (deg: number): number => (deg * Math.PI) / 180
  const {
    POS_DELTA_DEG: pos,
    WING_ANGLE_DEG: wing,
    GAP_C,
    GAP_W,
    PROX_RATIO,
    BACK_SLACK_RATIO,
    Z_CURVE,
  } = MODEL
  const phi = rad(d * pos)
  const ad = Math.abs(d)
  const sgn = Math.sign(d)
  // Piecewise-linear lane: |d|≤1 wide GAP_C pitch (motion safety for turning
  // cards), beyond → tight GAP_W (parallel wings, depth-safe overlap).
  const tx =
    ad <= 1
      ? sgn * S * GAP_C * ad
      : sgn * (S * GAP_C + (ad - 1) * S * GAP_W)
  const R = (S / 2) / Math.tan(rad(pos) / 2) * Z_CURVE
  const theta = sgn * Math.min(ad, 1) * wing
  const backFull = (S / 2) * Math.sin(rad(wing)) + BACK_SLACK_RATIO * S
  const back = backFull * Math.min(ad, 1)
  const prox =
    PROX_RATIO * S * Math.pow(Math.max(0, Math.cos((Math.PI * ad) / 2)), 2)
  return {
    theta,
    tx,
    tz: -R * (1 - Math.cos(phi)) - back + prox,
  }
}
/**
 * Visual prominence as a pure function of d: the resting center is most
 * present (0.8) and cards fade toward the edges (0.2 at |d| ≥ 3).
 * Applied on the inner (2D) frame so the slot's preserve-3d participation
 * is untouched; multiplies with the img's load-fade (0→1).
 */
export const OPACITY_CENTER = 0.8
export const OPACITY_EDGE = 0.2
export const OPACITY_FALLOFF_D = 3 // matches M slots per side

export function slotOpacity(d: number): number {
  const t = Math.min(Math.abs(d), OPACITY_FALLOFF_D) / OPACITY_FALLOFF_D
  return OPACITY_CENTER - (OPACITY_CENTER - OPACITY_EDGE) * t
}
