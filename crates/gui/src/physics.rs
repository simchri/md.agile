//! Physics module for calculating card positions.
//!
//! Cards are positioned along the diagonal from top-left to bottom-right,
//! based on their progress percentage (0.0–1.0).

/// Card input: progress percentage (0.0–1.0).
#[derive(Debug, Clone, Copy)]
pub struct Card {
    /// `Some(p)` if this card is in-progress with `0.0 < p < 1.0`;
    /// `None` if the card is inactive (backlog/done).
    pub progress: Option<f64>,
}

/// Card output: normalized (x, y) coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CardPosition {
    /// Normalized x coordinate (0.0 = left edge, 1.0 = right edge).
    pub x: f64,
    /// Normalized y coordinate (0.0 = top edge, 1.0 = bottom edge).
    pub y: f64,
}

/// Computes positions for every card.
///
/// Input:
/// - `cards`: slice of Card with progress value
///
/// Output:
/// - `Vec<CardPosition>`: normalized (x, y) coordinates for each card
///
/// Logic:
/// - Cards with `progress = Some(p)` are positioned at (p, p) on the diagonal
///   (0.0, 0.0) = top-left, (1.0, 1.0) = bottom-right
/// - Cards with `progress = None` (backlog/done) default to (0.5, 0.5) but are
///   typically not rendered in the in-progress area
pub fn step(cards: &[Card]) -> Vec<CardPosition> {
    cards
        .iter()
        .map(|card| match card.progress {
            Some(p) => {
                let p = p.clamp(0.0, 1.0);
                CardPosition { x: p, y: p }
            }
            None => CardPosition { x: 0.5, y: 0.5 },
        })
        .collect()
}
