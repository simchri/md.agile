//! Physics module for calculating card positions.
//!
//! Currently: calculates normalized (x, y) position of each card.
//! All cards are placed at the center of the screen.
//! State parameter is present for future expansion but not currently used.

use crate::card_positioning::card_position_normalized;

// --- Public types ---

/// Internal state for a card's physics (currently unused, reserved for future).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CardPhysicsState {
    /// Reserved for future physics calculations.
    pub _reserved: f64,
}

/// Card input: progress percentage (0.0–1.0) and physics state.
#[derive(Debug, Clone, Copy)]
pub struct Card {
    /// `Some(p)` if this card is in-progress with `0.0 < p < 1.0`;
    /// `None` if the card is inactive (backlog/done).
    pub progress: Option<f64>,
    /// Current physics state (currently unused).
    pub state: CardPhysicsState,
}

/// Card output: normalized (x, y) coordinates and updated state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CardPosition {
    /// Normalized x coordinate (0.0 = left edge, 1.0 = right edge).
    pub x: f64,
    /// Normalized y coordinate (0.0 = top edge, 1.0 = bottom edge).
    pub y: f64,
    /// Updated physics state (currently same as input).
    pub state: CardPhysicsState,
}

// --- Integrator ---

/// Computes positions for every card.
///
/// Input:
/// - `cards`: slice of Card with progress and state
///
/// Output:
/// - `Vec<CardPosition>`: normalized (x, y) coordinates and state for each card
///
/// Currently: all cards are positioned at the center (0.5, 0.5) in normalized space.
/// The state parameter is ignored but preserved for future physics implementation.
pub fn step(cards: &[Card]) -> Vec<CardPosition> {
    cards
        .iter()
        .map(|card| CardPosition {
            x: 0.5,
            y: 0.5,
            state: card.state,
        })
        .collect()
}
