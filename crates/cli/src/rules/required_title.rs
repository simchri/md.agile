use crate::parser::{Marker, PropertyForm, parse_markers};

/// Matches a configured title, allowing only its pending property markers to
/// resolve. All text outside those markers remains byte-for-byte identical.
pub(super) fn matches_required_title(required: &str, actual: &str) -> bool {
    if required == actual {
        return true;
    }

    let (required_markers, _) = parse_markers(required);
    let (actual_markers, _) = parse_markers(actual);
    let mut required_offset = 0;
    let mut actual_offset = 0;

    for marker in required_markers {
        let Marker::Property(property) = marker else {
            continue;
        };
        if property.form != PropertyForm::BranchPending {
            continue;
        }

        let start = property.column - 1;
        let prefix = &required[required_offset..start];
        if !actual[actual_offset..].starts_with(prefix) {
            return false;
        }
        actual_offset += prefix.len();
        let Some(actual_property) = actual_markers.iter().find_map(|marker| match marker {
            Marker::Property(candidate)
                if candidate.column == actual_offset + 1 && candidate.name == property.name =>
            {
                Some(candidate)
            }
            _ => None,
        }) else {
            return false;
        };
        let outcome_len = match &actual_property.form {
            PropertyForm::BranchPending => "...".len(),
            PropertyForm::BranchResolved(outcome) => outcome.len(),
            _ => return false,
        };
        actual_offset += 1 + property.name.len() + 1 + outcome_len;
        required_offset = start + 1 + property.name.len() + ":...".len();
    }

    required[required_offset..] == actual[actual_offset..]
}
