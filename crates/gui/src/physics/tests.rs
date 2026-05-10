use super::*;

fn card(progress: Option<f64>, perp_offset: f64, perp_velocity: f64) -> Card {
    Card {
        progress,
        state: CardPhysicsState {
            perp_offset,
            perp_velocity,
        },
    }
}

#[test]
fn empty_pool_returns_empty_state() {
    let out = step(&[]);
    assert_eq!(out.len(), 0);
}

#[test]
fn inactive_cards_are_zeroed() {
    // Card has nonzero offset/velocity but is no longer in-progress.
    let cards = vec![card(None, 0.056, 0.0056)];
    let out = step(&cards);
    assert_eq!(out[0].x, 0.0);
    assert_eq!(out[0].y, 0.0);
    assert_eq!(out[0].state, CardPhysicsState::default());
}

#[test]
fn single_active_card_at_rest_relaxes_toward_zero() {
    // A single in-progress card with no neighbours should drift toward
    // offset=0 from the centering spring + damping.
    // Normalized offset: 100px / 900px ≈ 0.111.
    let cards = vec![card(Some(0.5), 0.111, 0.0)];
    let out = step(&cards);
    let pos = &out[0];
    // Velocity becomes -K_RESTORE * 0.111 = -0.00666, then *DAMPING=0.8 → -0.00533.
    assert!((pos.state.perp_velocity - (-0.00533)).abs() < 1e-4);
    // Offset moves from 0.111 → 0.111 + (-0.00533) ≈ 0.106.
    assert!((pos.state.perp_offset - (0.111 - 0.00533)).abs() < 1e-4);
    // Position should be somewhere in normalized space, not at edges.
    assert!(pos.x > 0.0 && pos.x < 1.0);
    assert!(pos.y > 0.0 && pos.y < 1.0);
}

#[test]
fn two_close_active_cards_repel() {
    // Two cards at the same offset (0) with progress within COLLISION_THRESHOLD.
    // The lower-offset one (oa <= ob → ia goes negative) should get a negative
    // velocity, the other a positive one.
    let cards = vec![
        card(Some(0.40), 0.0, 0.0),
        card(Some(0.45), 0.0, 0.0), // |Δp| = 0.05, well under 0.30 threshold
    ];
    let out = step(&cards);
    assert!(
        out[0].state.perp_velocity < 0.0,
        "first card pushed negative"
    );
    assert!(
        out[1].state.perp_velocity > 0.0,
        "second card pushed positive"
    );
    // Forces are equal and opposite.
    assert!((out[0].state.perp_velocity + out[1].state.perp_velocity).abs() < 1e-9);
}

#[test]
fn far_apart_active_cards_dont_repel() {
    // Two cards far enough apart that COLLISION_THRESHOLD doesn't trigger.
    let cards = vec![
        card(Some(0.10), 0.0, 0.0),
        card(Some(0.90), 0.0, 0.0), // |Δp| = 0.80 > 0.30
    ];
    let out = step(&cards);
    // Both at rest with no repulsion → centering spring is zero (offset is zero) → velocity is zero.
    // Note: boundary springs might still activate due to edge proximity, but forces should be tiny.
    assert!((out[0].state.perp_velocity).abs() < 0.02);
    assert!((out[1].state.perp_velocity).abs() < 0.02);
}

#[test]
fn boundary_spring_pushes_inward_near_left_edge() {
    // Card at progress=0 sits at the left edge. A negative perp_offset
    // pushes it further left into the boundary zone, which should add a
    // positive velocity impulse.
    // Normalized offset: -100px / 900px ≈ -0.111.
    let cards = vec![card(Some(0.0), -0.111, 0.0)];
    let out = step(&cards);
    // Centering spring alone would give v = -K_RESTORE * -0.111 = +0.00666, *0.8 = +0.00533.
    // Boundary impulse adds more on top, so velocity should exceed that.
    let centering_only = -K_RESTORE * -0.111 * DAMPING;
    assert!(
        out[0].state.perp_velocity > centering_only,
        "boundary impulse must add to centering: got {}, centering-only would be {}",
        out[0].state.perp_velocity,
        centering_only
    );
}

#[test]
fn offset_is_clamped_to_max() {
    // Start with normalized offset near MAX_OFFSET_FRAC and huge outward velocity.
    // Normalized offset ≈ 295px / 900px ≈ 0.327, velocity ≈ 100px / 900px ≈ 0.111.
    let cards = vec![card(Some(0.5), 0.327, 0.111)];
    let out = step(&cards);
    assert!(out[0].state.perp_offset <= MAX_OFFSET_FRAC);
    assert!(out[0].state.perp_offset >= -MAX_OFFSET_FRAC);
}

#[test]
fn mixed_pool_only_active_cards_get_physics() {
    // Card 0 inactive (None), card 1 active, card 2 inactive.
    let cards = vec![
        card(None, 0.056, 0.0056),
        card(Some(0.5), 0.011, 0.0),
        card(None, -0.033, -0.0022),
    ];
    let out = step(&cards);
    assert_eq!(out[0].state, CardPhysicsState::default());
    assert_eq!(out[0].x, 0.0);
    assert_eq!(out[0].y, 0.0);
    assert_eq!(out[2].state, CardPhysicsState::default());
    assert_eq!(out[2].x, 0.0);
    assert_eq!(out[2].y, 0.0);
    // Active card got its centering update and position.
    assert!(out[1].state.perp_velocity != 0.0);
    assert!(out[1].x > 0.0);
    assert!(out[1].y > 0.0);
}

#[test]
fn output_contains_normalized_positions() {
    // Verify that output CardPosition contains valid normalized coordinates
    // and that progress affects position along the diagonal.
    let cards = vec![
        card(Some(0.0), 0.0, 0.0), // Top-left
        card(Some(0.5), 0.0, 0.0), // Middle
        card(Some(1.0), 0.0, 0.0), // Bottom-right
    ];
    let out = step(&cards);

    for pos in &out {
        // All coordinates should be in normalized range.
        assert!(pos.x >= 0.0 && pos.x <= 1.0, "x out of bounds: {}", pos.x);
        assert!(pos.y >= 0.0 && pos.y <= 1.0, "y out of bounds: {}", pos.y);
    }

    // Progress moves along diagonal: as progress increases, x and y both increase
    // (in CSS coordinates, y increases downward).
    assert!(out[0].x < out[1].x, "higher progress should move right");
    assert!(out[1].x < out[2].x, "higher progress should move right");
    assert!(
        out[0].y < out[1].y,
        "higher progress should move down (CSS coords)"
    );
    assert!(
        out[1].y < out[2].y,
        "higher progress should move down (CSS coords)"
    );
}
