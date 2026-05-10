//! Physics module for calculating card positions.
//!
//! Cards are positioned along the diagonal from top-left to bottom-right,
//! based on their progress percentage (0.0–1.0). A spring-damper system
//! smoothly animates each card toward its target position.

/// Spring stiffness constant (higher = snappier).
const SPRING_K: f64 = 8.0;
/// Damping coefficient (higher = less oscillation). Critical damping ≈ 2*sqrt(k).
const DAMPING_C: f64 = 5.0;

/// A card's full state: target (progress) plus physics state (position + velocity).
///
/// The caller holds a `Vec<Card>` across frames and passes `&mut [Card]` to `step()`.
/// Between frames, callers update only `progress`; the physics state (`x`, `y`,
/// `vx`, `vy`) is preserved and evolved by `step()`.
#[derive(Debug, Clone, Copy)]
pub struct Card {
    /// `Some(p)` if this card is in-progress with `0.0 < p < 1.0`;
    /// `None` if the card is inactive (backlog/done).
    pub progress: Option<f64>,
    /// Current x position in normalized coordinates (0.0 = left, 1.0 = right).
    pub x: f64,
    /// Current y position in normalized coordinates (0.0 = top, 1.0 = bottom).
    pub y: f64,
    /// Current x velocity (normalized units per second).
    pub vx: f64,
    /// Current y velocity (normalized units per second).
    pub vy: f64,
}

impl Card {
    /// Construct a card at rest at a given position with no progress.
    pub fn new(x: f64, y: f64) -> Self {
        Card {
            progress: None,
            x,
            y,
            vx: 0.0,
            vy: 0.0,
        }
    }
}

/// Card output: normalized (x, y) coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CardPosition {
    /// Normalized x coordinate (0.0 = left edge, 1.0 = right edge).
    pub x: f64,
    /// Normalized y coordinate (0.0 = top edge, 1.0 = bottom edge).
    pub y: f64,
}

/// Advance the physics simulation by one time step `dt` (seconds).
///
/// For each card with `progress = Some(p)`, applies a spring-damper that pulls
/// the card toward its target position `(p, p)` on the diagonal.
///
/// Cards with `progress = None` are not moved; their position is unchanged.
///
/// Returns a `Vec<CardPosition>` with the updated position of each card.
pub fn step(cards: &mut [Card], dt: f64) -> Vec<CardPosition> {
    for card in cards.iter_mut() {
        if let Some(p) = card.progress {
            let target = p.clamp(0.0, 1.0);
            // Spring force toward target, plus damping.
            let ax = -SPRING_K * (card.x - target) - DAMPING_C * card.vx;
            let ay = -SPRING_K * (card.y - target) - DAMPING_C * card.vy;
            card.vx += ax * dt;
            card.vy += ay * dt;
            card.x += card.vx * dt;
            card.y += card.vy * dt;
        }
    }

    cards
        .iter()
        .map(|c| CardPosition { x: c.x, y: c.y })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card_at(x: f64, y: f64, progress: f64) -> Card {
        Card {
            progress: Some(progress),
            x,
            y,
            vx: 0.0,
            vy: 0.0,
        }
    }

    fn card_inactive() -> Card {
        Card::new(0.5, 0.5)
    }

    #[test]
    fn inactive_card_does_not_move() {
        let mut cards = [card_inactive()];
        let pos = step(&mut cards, 0.05);
        assert_eq!(pos[0], CardPosition { x: 0.5, y: 0.5 });
        assert_eq!(cards[0].vx, 0.0);
        assert_eq!(cards[0].vy, 0.0);
    }

    #[test]
    fn card_at_target_does_not_move() {
        // Card already at its target (progress = 0.5 → target (0.5, 0.5)).
        let mut cards = [card_at(0.5, 0.5, 0.5)];
        let pos_before = (cards[0].x, cards[0].y);
        let _ = step(&mut cards, 0.05);
        assert_eq!(cards[0].x, pos_before.0);
        assert_eq!(cards[0].y, pos_before.1);
    }

    #[test]
    fn spring_pulls_card_toward_target() {
        // Card at (0.0, 0.0) with progress = 1.0 → target is (1.0, 1.0).
        // After one step the card should move toward the target.
        let mut cards = [card_at(0.0, 0.0, 1.0)];
        let _ = step(&mut cards, 0.05);
        assert!(cards[0].x > 0.0, "card should move right toward target");
        assert!(cards[0].y > 0.0, "card should move down toward target");
    }

    #[test]
    fn spring_pulls_in_correct_direction() {
        // Card at (1.0, 1.0) with progress = 0.0 → target is (0.0, 0.0).
        let mut cards = [card_at(1.0, 1.0, 0.0)];
        let _ = step(&mut cards, 0.05);
        assert!(cards[0].x < 1.0, "card should move left toward target");
        assert!(cards[0].y < 1.0, "card should move up toward target");
    }

    #[test]
    fn damping_reduces_velocity_over_time() {
        // Card with initial velocity away from target; damping should slow it.
        let card = Card {
            progress: Some(0.5),
            x: 0.5,
            y: 0.5,
            vx: 1.0,
            vy: 1.0,
        };
        let mut cards = [card];
        let _ = step(&mut cards, 0.05);
        assert!(cards[0].vx < 1.0, "damping should reduce x velocity");
        assert!(cards[0].vy < 1.0, "damping should reduce y velocity");
    }

    #[test]
    fn card_settles_near_target_after_many_steps() {
        // Run many steps; card should converge to its target.
        let mut cards = [card_at(0.0, 0.0, 0.8)];
        for _ in 0..200 {
            let _ = step(&mut cards, 0.05);
        }
        let err_x = (cards[0].x - 0.8).abs();
        let err_y = (cards[0].y - 0.8).abs();
        assert!(
            err_x < 0.01,
            "card x should settle near 0.8, got {}",
            cards[0].x
        );
        assert!(
            err_y < 0.01,
            "card y should settle near 0.8, got {}",
            cards[0].y
        );
    }

    #[test]
    fn multiple_cards_are_independent() {
        let mut cards = [card_at(0.0, 0.0, 1.0), card_at(1.0, 1.0, 0.0)];
        let _ = step(&mut cards, 0.05);
        assert!(cards[0].x > 0.0, "first card should move right");
        assert!(cards[1].x < 1.0, "second card should move left");
    }

    #[test]
    fn velocity_persists_across_steps() {
        let mut cards = [card_at(0.0, 0.0, 1.0)];
        let _ = step(&mut cards, 0.05);
        let vx_after_first = cards[0].vx;
        assert!(
            vx_after_first > 0.0,
            "velocity should be non-zero after first step"
        );
    }

    #[test]
    fn returned_positions_match_card_state() {
        let mut cards = [card_at(0.2, 0.3, 0.8)];
        let positions = step(&mut cards, 0.05);
        assert_eq!(positions[0].x, cards[0].x);
        assert_eq!(positions[0].y, cards[0].y);
    }
}
