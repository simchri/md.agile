//! Spring-damper repulsion that spreads in-progress cards perpendicular to
//! the diagonal.
//!
//! Pure, signal-free integrator. The caller snapshots its slot signals
//! into a `Vec<Slot>`, runs [`step`] once per tick, and writes the
//! returned [`SlotPhysics`] back to the signals (typically with a
//! "skip-if-unchanged" filter so unrelated cards don't re-render).

// --- Tunables ---

/// Two in-progress cards collide when their progress values are within this
/// threshold. At 0.30 progress units ≈ a card-width's worth of overlap on a
/// typical 1440-wide viewport.
const COLLISION_THRESHOLD: f64 = 0.30;
/// Velocity impulse (px/tick) per unit of progress overlap.
const K_REPEL: f64 = 16.0;
/// Centering spring constant: pulls each card's perpendicular offset back
/// toward 0.
const K_RESTORE: f64 = 0.06;
/// Velocity retention per tick (lower = snappier settle, higher = more drift).
const DAMPING: f64 = 0.80;
/// Boundary springs activate when a card edge is within this many px of the
/// screen edge.
const BOUNDARY_ZONE_PX: f64 = 80.0;
/// Velocity impulse per pixel of penetration into the boundary zone.
const K_BOUNDARY: f64 = 0.08;
/// Hard clamp on perpendicular offset to prevent runaway in pathological
/// configurations.
const MAX_OFFSET_PX: f64 = 300.0;

// --- Card geometry constants ---
//
// These mirror the layout used by `card_positioning::diagonal_style`. If
// they're ever changed in one place, change them in the other too — see
// refactor_notes.tmp.md item #4.

/// Card edge length on the canvas (cards are square).
const CARD_PX: f64 = 220.0;
/// Total px the card occupies including its 5px diagonal-end margins.
const CARD_PLUS_MARGIN_PX: f64 = CARD_PX + 10.0;
/// 5px gap from the left edge of the canvas at progress = 0.
const LEFT_MARGIN_PX: f64 = 5.0;
/// Vertical fraction of the viewport at which the diagonal starts (just
/// below the top separator).
const DIAG_TOP_FRAC: f64 = 0.15;
/// Vertical fraction of the viewport spanned by the diagonal.
const DIAG_HEIGHT_FRAC: f64 = 0.70;
/// 45° approximation of the perpendicular to the diagonal: positive
/// `perp_offset` moves the card upper-right.
const PERP_AXIS: f64 = 0.707;

// --- Public types ---

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SlotPhysics {
    pub perp_offset: f64,
    pub perp_velocity: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct Slot {
    /// `Some(p)` if this slot holds an in-progress card with `0.0 < p < 1.0`;
    /// `None` if the slot is empty or holds a backlog/done card.
    pub progress: Option<f64>,
    pub physics: SlotPhysics,
}

#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    pub width_px: f64,
    pub height_px: f64,
}

/// Reference viewport used by the GUI when no real measurement is available.
pub const REFERENCE_VIEWPORT: Viewport = Viewport {
    width_px: 1440.0,
    height_px: 900.0,
};

// --- Integrator ---

/// Computes the next-tick physics state for every slot.
///
/// In-progress slots get pairwise repulsion + boundary springs + a
/// centering spring + damping applied. Empty / non-in-progress slots
/// always return [`SlotPhysics::default`] (offset = 0, velocity = 0).
pub fn step(slots: &[Slot], viewport: Viewport) -> Vec<SlotPhysics> {
    let mut out: Vec<SlotPhysics> = vec![SlotPhysics::default(); slots.len()];

    let active: Vec<(usize, f64, f64)> = slots
        .iter()
        .enumerate()
        .filter_map(|(i, s)| s.progress.map(|p| (i, p, s.physics.perp_offset)))
        .collect();

    // Pairwise repulsion: cards within COLLISION_THRESHOLD progress units
    // push each other apart in the perpendicular direction.
    let mut dv = vec![0.0_f64; slots.len()];
    for a in 0..active.len() {
        for b in (a + 1)..active.len() {
            let (ia, pa, oa) = active[a];
            let (ib, pb, ob) = active[b];
            let dp = (pa - pb).abs();
            if dp < COLLISION_THRESHOLD {
                let force = K_REPEL * (COLLISION_THRESHOLD - dp);
                if oa <= ob {
                    dv[ia] -= force;
                    dv[ib] += force;
                } else {
                    dv[ia] += force;
                    dv[ib] -= force;
                }
            }
        }
    }

    // Per-card integration: repulsion + boundary springs + centering + damping.
    for &(i, p, offset) in &active {
        let mut v = slots[i].physics.perp_velocity + dv[i];

        let (left, top) = card_top_left_px(p, offset, viewport);
        let right = left + CARD_PX;
        let bottom = top + CARD_PX;

        if left < BOUNDARY_ZONE_PX {
            v += K_BOUNDARY * (BOUNDARY_ZONE_PX - left);
        }
        if right > viewport.width_px - BOUNDARY_ZONE_PX {
            v -= K_BOUNDARY * (right - (viewport.width_px - BOUNDARY_ZONE_PX));
        }
        if top < BOUNDARY_ZONE_PX {
            v -= K_BOUNDARY * (BOUNDARY_ZONE_PX - top);
        }
        if bottom > viewport.height_px - BOUNDARY_ZONE_PX {
            v += K_BOUNDARY * (bottom - (viewport.height_px - BOUNDARY_ZONE_PX));
        }

        v -= K_RESTORE * offset;
        v *= DAMPING;
        let new_offset = (offset + v).clamp(-MAX_OFFSET_PX, MAX_OFFSET_PX);

        out[i] = SlotPhysics {
            perp_offset: new_offset,
            perp_velocity: v,
        };
    }

    out
}

/// Top-left pixel coordinates of a card at `progress` with `perp_offset` px
/// perpendicular displacement. Mirrors the formula in
/// `card_positioning::diagonal_style`.
fn card_top_left_px(progress: f64, perp_offset: f64, viewport: Viewport) -> (f64, f64) {
    let left = LEFT_MARGIN_PX
        + progress * (viewport.width_px - CARD_PLUS_MARGIN_PX)
        + perp_offset * PERP_AXIS;
    let top = DIAG_TOP_FRAC * viewport.height_px
        + LEFT_MARGIN_PX
        + progress * (DIAG_HEIGHT_FRAC * viewport.height_px - CARD_PLUS_MARGIN_PX)
        - perp_offset * PERP_AXIS;
    (left, top)
}

#[cfg(test)]
mod tests;
