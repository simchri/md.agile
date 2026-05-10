//! Spring-damper repulsion that spreads in-progress cards perpendicular to
//! the diagonal.
//!
//! Pure, signal-free integrator. The caller maintains a list of card progress
//! values and internal physics state (`CardPhysicsState`). Each tick, call [`step`]
//! with current progress and state to get back normalized (x, y) coordinates for
//! rendering and the new state for the next tick.
//!
//! All coordinates are **normalized** (0.0–1.0 fractions of viewport dimensions):
//! - Progress: 0.0 = top-left, 1.0 = bottom-right (input)
//! - Output coordinates: (x, y) as fractions of viewport width and height
//! - Perpendicular offset: fraction of viewport height (internal state)
//! - Card size: `ASSUMED_CARD_SIZE_FRAC` as fraction of viewport height
//! - Boundary zone: `BOUNDARY_ZONE_FRAC` as fraction of viewport width/height
//!
//! This makes the physics engine **viewport-size-agnostic**: the same behavior
//! applies whether the screen is 1440×900 or 1920×1080.

use crate::card_positioning::card_position_normalized;

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

/// Internal state for a single card's physics.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CardPhysicsState {
    /// Perpendicular offset as fraction of viewport height.
    pub perp_offset: f64,
    /// Perpendicular velocity as fraction of height per tick.
    pub perp_velocity: f64,
}

/// Card input for physics simulation: progress percentage and current state.
#[derive(Debug, Clone, Copy)]
pub struct Card {
    /// `Some(p)` if this card is in-progress with `0.0 < p < 1.0`;
    /// `None` if the card is inactive (backlog/done).
    pub progress: Option<f64>,
    /// Current physics state (offset and velocity).
    pub state: CardPhysicsState,
}

/// Output from physics simulation: normalized (x, y) coordinates and new state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CardPosition {
    /// Normalized x coordinate (0.0 = left edge, 1.0 = right edge).
    pub x: f64,
    /// Normalized y coordinate (0.0 = top edge, 1.0 = bottom edge).
    pub y: f64,
    /// New physics state after this tick (to be fed back into next step call).
    pub state: CardPhysicsState,
}

// --- Integrator ---

/// Computes physics and positions for every card.
///
/// Input:
/// - `cards`: slice of progress values (0.0–1.0) and current physics state
///
/// Output:
/// - `Vec<CardPosition>`: normalized (x, y) coordinates and new state for each card
///
/// In-progress cards (progress = Some) get pairwise repulsion + boundary springs +
/// centering spring + damping applied. Inactive cards (progress = None) return
/// coordinates at (x=0, y=0) with zeroed state.
///
/// All calculations use normalized coordinates independent of viewport size.
pub fn step(cards: &[Card]) -> Vec<CardPosition> {
    let mut out: Vec<CardPosition> = cards
        .iter()
        .map(|_| CardPosition {
            x: 0.0,
            y: 0.0,
            state: CardPhysicsState::default(),
        })
        .collect();

    let active: Vec<(usize, f64, f64)> = cards
        .iter()
        .enumerate()
        .filter_map(|(i, c)| c.progress.map(|p| (i, p, c.state.perp_offset)))
        .collect();

    // Pairwise repulsion: cards within COLLISION_THRESHOLD progress units
    // push each other apart in the perpendicular direction.
    let mut dv = vec![0.0_f64; cards.len()];
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
    // Boundary checks work in normalized space without needing viewport dimensions.
    for &(i, p, offset) in &active {
        let mut v = cards[i].state.perp_velocity + dv[i];

        // Get normalized card position (0.0–1.0 fractions).
        let (x_norm, y_norm) = card_position_normalized(p, offset);

        // Boundary springs: activate when normalized position is within
        // BOUNDARY_ZONE_FRAC of the edges (0.0 or 1.0 in normalized space).
        let card_half_size = ASSUMED_CARD_SIZE_FRAC / 2.0;

        // Left boundary: when card goes below left edge + boundary zone.
        if x_norm < BOUNDARY_ZONE_FRAC + card_half_size {
            let penetration = (BOUNDARY_ZONE_FRAC + card_half_size) - x_norm;
            v += K_BOUNDARY * penetration;
        }

        // Right boundary: when card goes past right edge - boundary zone.
        if x_norm > 1.0 - BOUNDARY_ZONE_FRAC - card_half_size {
            let penetration = x_norm - (1.0 - BOUNDARY_ZONE_FRAC - card_half_size);
            v -= K_BOUNDARY * penetration;
        }

        // Top boundary: when card goes above top edge + boundary zone.
        if y_norm < BOUNDARY_ZONE_FRAC + card_half_size {
            let penetration = (BOUNDARY_ZONE_FRAC + card_half_size) - y_norm;
            v -= K_BOUNDARY * penetration;
        }

        // Bottom boundary: when card goes below bottom edge - boundary zone.
        if y_norm > 1.0 - BOUNDARY_ZONE_FRAC - card_half_size {
            let penetration = y_norm - (1.0 - BOUNDARY_ZONE_FRAC - card_half_size);
            v += K_BOUNDARY * penetration;
        }

        // Centering spring: pulls offset back toward 0.
        v -= K_RESTORE * offset;

        // Apply damping and clamp.
        v *= DAMPING;
        let new_offset = (offset + v).clamp(-MAX_OFFSET_FRAC, MAX_OFFSET_FRAC);

        out[i] = CardPosition {
            x: x_norm,
            y: y_norm,
            state: CardPhysicsState {
                perp_offset: new_offset,
                perp_velocity: v,
            },
        };
    }

    out
}

#[cfg(test)]
mod tests;
