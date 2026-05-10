//! Physics module for calculating card positions.
//!
//! Cards are positioned along the diagonal from top-left to bottom-right,
//! based on their progress percentage (0.0–1.0). A spring-damper system
//! smoothly animates each card toward its target position.

/// Spring stiffness constant (higher = snappier).
const SPRING_K: f64 = 8.0;
/// Damping coefficient (higher = less oscillation). Critical damping ≈ 2*sqrt(k).
const DAMPING_C: f64 = 5.0;

/// Normalized (x, y) position on the canvas (0.0 = left/top, 1.0 = right/bottom).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CardPosition {
    pub x: f64,
    pub y: f64,
}

/// A card's full state: target (progress) plus physics state (position + velocity).
///
/// The caller holds a `Vec<Card>` across frames and passes `&mut [Card]` to `step()`.
/// Between frames, callers update only `progress`; `position` and velocity are
/// preserved and evolved by `step()`.
#[derive(Debug, Clone, Copy)]
pub struct Card {
    /// `Some(p)` if this card is in-progress with `0.0 < p < 1.0`;
    /// `None` if the card is inactive (backlog/done).
    pub progress: Option<f64>,
    /// Current position in normalized coordinates.
    pub position: CardPosition,
    /// Current x velocity (normalized units per second).
    pub vx: f64,
    /// Current y velocity (normalized units per second).
    pub vy: f64,
}

impl Card {
    /// Construct a card at rest at a given position with no progress.
    pub fn new(position: CardPosition) -> Self {
        Card {
            progress: None,
            position,
            vx: 0.0,
            vy: 0.0,
        }
    }
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
            let ax = -SPRING_K * (card.position.x - target) - DAMPING_C * card.vx;
            let ay = -SPRING_K * (card.position.y - target) - DAMPING_C * card.vy;
            card.vx += ax * dt;
            card.vy += ay * dt;
            card.position.x += card.vx * dt;
            card.position.y += card.vy * dt;
        }
    }

    cards.iter().map(|c| c.position).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card_at(x: f64, y: f64, progress: f64) -> Card {
        Card {
            progress: Some(progress),
            position: CardPosition { x, y },
            vx: 0.0,
            vy: 0.0,
        }
    }

    fn card_inactive() -> Card {
        Card::new(CardPosition { x: 0.5, y: 0.5 })
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
        let pos_before = cards[0].position;
        let _ = step(&mut cards, 0.05);
        assert_eq!(cards[0].position, pos_before);
    }

    #[test]
    fn spring_pulls_card_toward_target() {
        // Card at (0.0, 0.0) with progress = 1.0 → target is (1.0, 1.0).
        let mut cards = [card_at(0.0, 0.0, 1.0)];
        let _ = step(&mut cards, 0.05);
        assert!(
            cards[0].position.x > 0.0,
            "card should move right toward target"
        );
        assert!(
            cards[0].position.y > 0.0,
            "card should move down toward target"
        );
    }

    #[test]
    fn spring_pulls_in_correct_direction() {
        // Card at (1.0, 1.0) with progress = 0.0 → target is (0.0, 0.0).
        let mut cards = [card_at(1.0, 1.0, 0.0)];
        let _ = step(&mut cards, 0.05);
        assert!(
            cards[0].position.x < 1.0,
            "card should move left toward target"
        );
        assert!(
            cards[0].position.y < 1.0,
            "card should move up toward target"
        );
    }

    #[test]
    fn damping_reduces_velocity_over_time() {
        // Card with initial velocity away from target; damping should slow it.
        let mut cards = [Card {
            progress: Some(0.5),
            position: CardPosition { x: 0.5, y: 0.5 },
            vx: 1.0,
            vy: 1.0,
        }];
        let _ = step(&mut cards, 0.05);
        assert!(cards[0].vx < 1.0, "damping should reduce x velocity");
        assert!(cards[0].vy < 1.0, "damping should reduce y velocity");
    }

    #[test]
    fn card_settles_near_target_after_many_steps() {
        let mut cards = [card_at(0.0, 0.0, 0.8)];
        for _ in 0..200 {
            let _ = step(&mut cards, 0.05);
        }
        let err_x = (cards[0].position.x - 0.8).abs();
        let err_y = (cards[0].position.y - 0.8).abs();
        assert!(
            err_x < 0.01,
            "card x should settle near 0.8, got {}",
            cards[0].position.x
        );
        assert!(
            err_y < 0.01,
            "card y should settle near 0.8, got {}",
            cards[0].position.y
        );
    }

    #[test]
    fn multiple_cards_are_independent() {
        let mut cards = [card_at(0.0, 0.0, 1.0), card_at(1.0, 1.0, 0.0)];
        let _ = step(&mut cards, 0.05);
        assert!(cards[0].position.x > 0.0, "first card should move right");
        assert!(cards[1].position.x < 1.0, "second card should move left");
    }

    #[test]
    fn velocity_persists_across_steps() {
        let mut cards = [card_at(0.0, 0.0, 1.0)];
        let _ = step(&mut cards, 0.05);
        assert!(
            cards[0].vx > 0.0,
            "velocity should be non-zero after first step"
        );
    }

    #[test]
    fn returned_positions_match_card_state() {
        let mut cards = [card_at(0.2, 0.3, 0.8)];
        let positions = step(&mut cards, 0.05);
        assert_eq!(positions[0], cards[0].position);
    }
}
