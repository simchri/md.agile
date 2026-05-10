//! Spring-damper repulsion that spreads in-progress cards perpendicular to
//! the diagonal.
//!
//! Pure, signal-free integrator. The caller snapshots its slot signals
//! into a `Vec<Slot>`, runs [`step`] once per tick, and writes the
//! returned [`SlotPhysics`] back to the signals (typically with a
//! "skip-if-unchanged" filter so unrelated cards don't re-render).
//!
//! All coordinates are **normalized** (0.0–1.0 fractions of viewport dimensions):
//! - Progress: 0.0 = top-left, 1.0 = bottom-right (unchanged from before)
//! - Perpendicular offset: fraction of viewport height
//! - Card size: `ASSUMED_CARD_SIZE_FRAC` as fraction of viewport height
//! - Boundary zone: `BOUNDARY_ZONE_FRAC` as fraction of viewport width/height
//!
//! This makes the physics engine **viewport-size-agnostic**: the same behavior
//! applies whether the screen is 1440×900 or 1920×1080.
//!
//! Boundary checks and CSS positioning happen in [`crate::card_positioning`],
//! which converts these normalized coordinates to pixels for rendering.

use crate::card_positioning::{card_position_normalized, Viewport, CARD_PX};

// --- Tunables (all normalized) ---

/// Card edge length as a fraction of viewport height.
/// Reference: 220px on 900px = 0.244.
pub const ASSUMED_CARD_SIZE_FRAC: f64 = 0.244;

/// Two in-progress cards collide when their progress values are within this
/// threshold. Unchanged from pixel-based system.
pub const COLLISION_THRESHOLD: f64 = 0.30;

/// Velocity impulse (fraction of height per tick) per unit of progress overlap.
/// Tuned for normalized space: original K_REPEL=16.0 @ 220px card.
pub const K_REPEL: f64 = 0.018;

/// Centering spring constant: pulls each card's perpendicular offset back
/// toward 0. Original K_RESTORE=0.06 (pixel-based).
pub const K_RESTORE: f64 = 0.060;

/// Velocity retention per tick (lower = snappier settle, higher = more drift).
pub const DAMPING: f64 = 0.80;

/// Boundary springs activate when a card edge is within this fraction of the
/// screen width or height. Original: 80px @ 1440w, 900h → ~0.0556 (width) or ~0.0889 (height).
/// Use average: 0.0722.
pub const BOUNDARY_ZONE_FRAC: f64 = 0.0722;

/// Velocity impulse per unit of penetration into the boundary zone.
/// Tuned for normalized space: original K_BOUNDARY=0.08 (pixel-based).
pub const K_BOUNDARY: f64 = 0.088;

/// Hard clamp on perpendicular offset to prevent runaway.
/// Original: 300px / 900px = 0.333.
pub const MAX_OFFSET_FRAC: f64 = 0.333;

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

// --- Integrator ---

/// Computes the next-tick physics state for every slot.
///
/// In-progress slots get pairwise repulsion + boundary springs + a
/// centering spring + damping applied. Empty / non-in-progress slots
/// always return [`SlotPhysics::default`] (offset = 0, velocity = 0).
///
/// All calculations use normalized coordinates (fractions of viewport dimensions).
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
    // Boundary checks work by getting normalized card position, converting to pixels,
    // and checking against pixel-based boundary zones.
    const BOUNDARY_ZONE_PX: f64 = 80.0;

    for &(i, p, offset) in &active {
        let mut v = slots[i].physics.perp_velocity + dv[i];

        // Get normalized card position (0.0–1.0 fractions of viewport).
        let (left_norm, top_norm) = card_position_normalized(p, offset);

        // Convert to pixels for boundary checking.
        let left = left_norm * viewport.width_px;
        let top = top_norm * viewport.height_px;
        let right = left + CARD_PX;
        let bottom = top + CARD_PX;

        // Boundary impulses: scale from pixel space to normalized space by dividing by viewport.height_px.
        // This ensures that K_BOUNDARY works the same way regardless of viewport size.
        if left < BOUNDARY_ZONE_PX {
            v += K_BOUNDARY * (BOUNDARY_ZONE_PX - left) / viewport.height_px;
        }
        if right > viewport.width_px - BOUNDARY_ZONE_PX {
            v -= K_BOUNDARY * (right - (viewport.width_px - BOUNDARY_ZONE_PX)) / viewport.height_px;
        }
        if top < BOUNDARY_ZONE_PX {
            v -= K_BOUNDARY * (BOUNDARY_ZONE_PX - top) / viewport.height_px;
        }
        if bottom > viewport.height_px - BOUNDARY_ZONE_PX {
            v += K_BOUNDARY * (bottom - (viewport.height_px - BOUNDARY_ZONE_PX))
                / viewport.height_px;
        }

        // Centering spring: pulls offset back toward 0.
        v -= K_RESTORE * offset;

        // Apply damping and clamp.
        v *= DAMPING;
        let new_offset = (offset + v).clamp(-MAX_OFFSET_FRAC, MAX_OFFSET_FRAC);

        out[i] = SlotPhysics {
            perp_offset: new_offset,
            perp_velocity: v,
        };
    }

    out
}

#[cfg(test)]
mod tests;
