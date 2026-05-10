use super::*;
use crate::card_positioning::{Viewport, REFERENCE_VIEWPORT};

fn slot(progress: Option<f64>, perp_offset: f64, perp_velocity: f64) -> Slot {
    Slot {
        progress,
        physics: SlotPhysics {
            perp_offset,
            perp_velocity,
        },
    }
}

// Use reference viewport for realistic physics calculations.
// Physics are normalized, so same constants work on any viewport.
const VP: Viewport = REFERENCE_VIEWPORT;

#[test]
fn empty_pool_returns_empty_state() {
    let out = step(&[], VP);
    assert_eq!(out.len(), 0);
}

#[test]
fn inactive_slots_are_zeroed() {
    // Slot has nonzero offset/velocity but is no longer in-progress.
    // Normalized offsets: 50px / 900px ≈ 0.056, velocity 5px / 900px ≈ 0.0056.
    let slots = vec![slot(None, 0.056, 0.0056)];
    let out = step(&slots, VP);
    assert_eq!(out, vec![SlotPhysics::default()]);
}

#[test]
fn single_active_card_at_rest_relaxes_toward_zero() {
    // A single in-progress card with no neighbours should drift toward
    // offset=0 from the centering spring + damping.
    // Normalized offset: 100px / 900px ≈ 0.111.
    let slots = vec![slot(Some(0.5), 0.111, 0.0)];
    let out = step(&slots, VP);
    let s = out[0];
    // Velocity becomes -K_RESTORE * 0.111 = -0.00666, then *DAMPING=0.8 → -0.00533.
    assert!((s.perp_velocity - (-0.00533)).abs() < 1e-4);
    // Offset moves from 0.111 → 0.111 + (-0.00533) ≈ 0.106.
    assert!((s.perp_offset - (0.111 - 0.00533)).abs() < 1e-4);
}

#[test]
fn two_close_active_cards_repel() {
    // Two cards at the same offset (0) with progress within COLLISION_THRESHOLD.
    // The lower-offset one (oa <= ob → ia goes negative) should get a negative
    // velocity, the other a positive one.
    let slots = vec![
        slot(Some(0.40), 0.0, 0.0),
        slot(Some(0.45), 0.0, 0.0), // |Δp| = 0.05, well under 0.30 threshold
    ];
    let out = step(&slots, VP);
    assert!(out[0].perp_velocity < 0.0, "first card pushed negative");
    assert!(out[1].perp_velocity > 0.0, "second card pushed positive");
    // Forces are equal and opposite.
    assert!((out[0].perp_velocity + out[1].perp_velocity).abs() < 1e-9);
}

#[test]
fn far_apart_active_cards_dont_repel() {
    // Two cards far enough apart that COLLISION_THRESHOLD doesn't trigger.
    let slots = vec![
        slot(Some(0.10), 0.0, 0.0),
        slot(Some(0.90), 0.0, 0.0), // |Δp| = 0.80 > 0.30
    ];
    let out = step(&slots, VP);
    // Both at rest with no repulsion → centering spring is zero (offset is zero) → no velocity change.
    assert_eq!(out[0].perp_velocity, 0.0);
    assert_eq!(out[1].perp_velocity, 0.0);
}

#[test]
fn boundary_spring_pushes_inward_near_left_edge() {
    // Card at progress=0 sits at the left edge. A negative perp_offset
    // pushes it further left into the boundary zone, which should add a
    // positive velocity impulse.
    // Normalized offset: -100px / 900px ≈ -0.111.
    let slots = vec![slot(Some(0.0), -0.111, 0.0)];
    let out = step(&slots, VP);
    // Centering spring alone would give v = -K_RESTORE * -0.111 = +0.00666, *0.8 = +0.00533.
    // Boundary impulse adds more on top, so velocity should exceed that.
    let centering_only = -K_RESTORE * -0.111 * DAMPING;
    assert!(
        out[0].perp_velocity > centering_only,
        "boundary impulse must add to centering: got {}, centering-only would be {}",
        out[0].perp_velocity,
        centering_only
    );
}

#[test]
fn offset_is_clamped_to_max() {
    // Start with normalized offset near MAX_OFFSET_FRAC and huge outward velocity.
    // Normalized offset ≈ 295px / 900px ≈ 0.327, velocity ≈ 100px / 900px ≈ 0.111.
    let slots = vec![slot(Some(0.5), 0.327, 0.111)];
    let out = step(&slots, VP);
    assert!(out[0].perp_offset <= MAX_OFFSET_FRAC);
    assert!(out[0].perp_offset >= -MAX_OFFSET_FRAC);
}

#[test]
fn mixed_pool_only_active_cards_get_physics() {
    // Slot 0 inactive (None), slot 1 active, slot 2 inactive.
    // Normalized offsets/velocities: 50px/900, 10px/900, etc.
    let slots = vec![
        slot(None, 0.056, 0.0056),
        slot(Some(0.5), 0.011, 0.0),
        slot(None, -0.033, -0.0022),
    ];
    let out = step(&slots, VP);
    assert_eq!(out[0], SlotPhysics::default());
    assert_eq!(out[2], SlotPhysics::default());
    // Active card got its centering update.
    assert!(out[1].perp_velocity != 0.0);
}
