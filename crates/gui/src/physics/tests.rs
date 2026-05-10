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

const VP: Viewport = REFERENCE_VIEWPORT;

#[test]
fn empty_pool_returns_empty_state() {
    let out = step(&[], VP);
    assert_eq!(out.len(), 0);
}

#[test]
fn inactive_slots_are_zeroed() {
    // Slot has nonzero offset/velocity but is no longer in-progress.
    let slots = vec![slot(None, 50.0, 5.0)];
    let out = step(&slots, VP);
    assert_eq!(out, vec![SlotPhysics::default()]);
}

#[test]
fn single_active_card_at_rest_relaxes_toward_zero() {
    // A single in-progress card with no neighbours should drift toward
    // offset=0 from the centering spring + damping.
    let slots = vec![slot(Some(0.5), 100.0, 0.0)];
    let out = step(&slots, VP);
    let s = out[0];
    // Velocity becomes -K_RESTORE * 100.0 = -6.0, then *DAMPING=0.8 → -4.8.
    assert!((s.perp_velocity - (-4.8)).abs() < 1e-9);
    // Offset moves from 100 → 100 + (-4.8) = 95.2.
    assert!((s.perp_offset - 95.2).abs() < 1e-9);
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
    // Both at rest with no repulsion → no velocity change.
    assert_eq!(out[0].perp_velocity, 0.0);
    assert_eq!(out[1].perp_velocity, 0.0);
}

#[test]
fn boundary_spring_pushes_inward_near_left_edge() {
    // Card at progress=0 sits at the left edge. A negative perp_offset
    // pushes it further left (since left = … + offset * 0.707), into the
    // boundary zone, which should add a positive velocity impulse.
    let slots = vec![slot(Some(0.0), -100.0, 0.0)];
    let out = step(&slots, VP);
    // Centering spring alone would give v = -K_RESTORE * -100 = +6, *0.8 = 4.8.
    // Boundary impulse adds more on top, so velocity should exceed 4.8.
    assert!(
        out[0].perp_velocity > 4.8,
        "boundary impulse must add to centering: got {}",
        out[0].perp_velocity
    );
}

#[test]
fn offset_is_clamped_to_max() {
    // Start at +MAX_OFFSET_PX with zero velocity. Centering tries to pull
    // back, so this won't trigger the clamp going forward — instead test
    // by giving a huge outward velocity and confirming offset stops at the
    // cap.
    let slots = vec![slot(Some(0.5), 295.0, 100.0)];
    let out = step(&slots, VP);
    assert!(out[0].perp_offset <= MAX_OFFSET_PX);
    assert!(out[0].perp_offset >= -MAX_OFFSET_PX);
}

#[test]
fn mixed_pool_only_active_cards_get_physics() {
    // Slot 0 inactive (None), slot 1 active, slot 2 inactive.
    let slots = vec![
        slot(None, 50.0, 5.0),
        slot(Some(0.5), 10.0, 0.0),
        slot(None, -30.0, -2.0),
    ];
    let out = step(&slots, VP);
    assert_eq!(out[0], SlotPhysics::default());
    assert_eq!(out[2], SlotPhysics::default());
    // Active card got its centering update.
    assert!(out[1].perp_velocity != 0.0);
}
