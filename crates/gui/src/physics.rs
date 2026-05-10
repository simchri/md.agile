//! Physics module for calculating card positions.
//!
//! Currently: calculates normalized (x, y) position of each card.
//! All cards are placed at the center of the screen.

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
/// Currently: all cards are positioned at the center (0.5, 0.5) in normalized space.
pub fn step(cards: &[Card]) -> Vec<CardPosition> {
    cards
        .iter()
        .map(|_card| CardPosition { x: 0.5, y: 0.5 })
        .collect()
}
