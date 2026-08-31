use super::*;

#[test]
fn plain_title_with_no_order_or_markers_is_a_single_plain_token() {
    let tokens = tokenize_title("first step", None, &[]);
    assert_eq!(tokens, vec![TitleToken::Plain("first step".to_string())]);
}

#[test]
fn empty_title_produces_no_tokens() {
    let tokens = tokenize_title("", None, &[]);
    assert_eq!(tokens, vec![]);
}

#[test]
fn leading_order_prefix_is_split_into_its_own_token() {
    let tokens = tokenize_title("1. first step", Some(1), &[]);
    assert_eq!(
        tokens,
        vec![
            TitleToken::Order("1.".to_string()),
            TitleToken::Plain(" first step".to_string()),
        ]
    );
}

#[test]
fn order_is_ignored_when_title_does_not_actually_start_with_the_expected_prefix() {
    // Defensive: if `order` and the title text somehow disagree (shouldn't
    // happen given the parser's own guarantees, but this must not panic or
    // corrupt the title), the whole title is kept as plain text untouched.
    let tokens = tokenize_title("first step", Some(1), &[]);
    assert_eq!(tokens, vec![TitleToken::Plain("first step".to_string())]);
}

#[test]
fn markers_anywhere_in_the_title_are_split_out_in_order() {
    let tokens = tokenize_title(
        "fix login #bug @alice",
        None,
        &["#bug".to_string(), "@alice".to_string()],
    );
    assert_eq!(
        tokens,
        vec![
            TitleToken::Plain("fix login ".to_string()),
            TitleToken::Marker("#bug".to_string()),
            TitleToken::Plain(" ".to_string()),
            TitleToken::Marker("@alice".to_string()),
        ]
    );
}

#[test]
fn order_and_markers_combine_in_a_single_pass() {
    let tokens = tokenize_title("1. first step @bob", Some(1), &["@bob".to_string()]);
    assert_eq!(
        tokens,
        vec![
            TitleToken::Order("1.".to_string()),
            TitleToken::Plain(" first step ".to_string()),
            TitleToken::Marker("@bob".to_string()),
        ]
    );
}

#[test]
fn marker_at_the_very_start_of_the_title_produces_no_leading_empty_plain_token() {
    let tokens = tokenize_title("#bug needs fixing", None, &["#bug".to_string()]);
    assert_eq!(
        tokens,
        vec![
            TitleToken::Marker("#bug".to_string()),
            TitleToken::Plain(" needs fixing".to_string()),
        ]
    );
}

#[test]
fn marker_at_the_very_end_of_the_title_produces_no_trailing_empty_plain_token() {
    let tokens = tokenize_title("fix login #bug", None, &["#bug".to_string()]);
    assert_eq!(
        tokens,
        vec![
            TitleToken::Plain("fix login ".to_string()),
            TitleToken::Marker("#bug".to_string()),
        ]
    );
}

#[test]
fn a_marker_not_found_in_the_title_text_is_silently_skipped_and_kept_as_plain_text() {
    // Defensive: a mismatch between the formatted marker string and the
    // title's literal text (shouldn't normally happen) must not lose or
    // corrupt any text — it's just left unstyled.
    let tokens = tokenize_title("fix login issue", None, &["#nonexistent".to_string()]);
    assert_eq!(
        tokens,
        vec![TitleToken::Plain("fix login issue".to_string())]
    );
}

#[test]
fn duplicate_marker_text_appearing_twice_is_matched_progressively_left_to_right() {
    let tokens = tokenize_title(
        "ping @bob then ping @bob again",
        None,
        &["@bob".to_string(), "@bob".to_string()],
    );
    assert_eq!(
        tokens,
        vec![
            TitleToken::Plain("ping ".to_string()),
            TitleToken::Marker("@bob".to_string()),
            TitleToken::Plain(" then ping ".to_string()),
            TitleToken::Marker("@bob".to_string()),
            TitleToken::Plain(" again".to_string()),
        ]
    );
}

#[test]
fn empty_marker_string_is_skipped() {
    let tokens = tokenize_title("first step", None, &["".to_string()]);
    assert_eq!(tokens, vec![TitleToken::Plain("first step".to_string())]);
}

#[test]
fn tokenize_text_plain_text_with_no_markers_is_a_single_plain_token() {
    let tokens = tokenize_text("just a body line, nothing special");
    assert_eq!(
        tokens,
        vec![TitleToken::Plain(
            "just a body line, nothing special".to_string()
        )]
    );
}

#[test]
fn tokenize_text_empty_text_produces_no_tokens() {
    assert_eq!(tokenize_text(""), vec![]);
}

#[test]
fn tokenize_text_finds_hash_and_at_markers_anywhere() {
    let tokens = tokenize_text("see #bug and ping @alice about it");
    assert_eq!(
        tokens,
        vec![
            TitleToken::Plain("see ".to_string()),
            TitleToken::Marker("#bug".to_string()),
            TitleToken::Plain(" and ping ".to_string()),
            TitleToken::Marker("@alice".to_string()),
            TitleToken::Plain(" about it".to_string()),
        ]
    );
}

#[test]
fn tokenize_text_marker_at_start_and_end_produce_no_stray_empty_plain_tokens() {
    let tokens = tokenize_text("#bug needs @alice");
    assert_eq!(
        tokens,
        vec![
            TitleToken::Marker("#bug".to_string()),
            TitleToken::Plain(" needs ".to_string()),
            TitleToken::Marker("@alice".to_string()),
        ]
    );
}

#[test]
fn tokenize_text_strips_trailing_punctuation_from_marker_name() {
    let tokens = tokenize_text("ping @alice, then #bug.");
    assert_eq!(
        tokens,
        vec![
            TitleToken::Plain("ping ".to_string()),
            TitleToken::Marker("@alice".to_string()),
            TitleToken::Plain(", then ".to_string()),
            TitleToken::Marker("#bug".to_string()),
            TitleToken::Plain(".".to_string()),
        ]
    );
}

#[test]
fn tokenize_text_bare_sigil_with_no_name_is_left_as_plain_text() {
    let tokens = tokenize_text("cost is # 5 dollars @ noon");
    assert_eq!(
        tokens,
        vec![TitleToken::Plain("cost is # 5 dollars @ noon".to_string())]
    );
}

#[test]
fn tokenize_text_sigil_as_final_character_is_left_as_plain_text() {
    let tokens = tokenize_text("trailing hash #");
    assert_eq!(
        tokens,
        vec![TitleToken::Plain("trailing hash #".to_string())]
    );
}

#[test]
fn tokenize_text_marker_bounded_by_brackets_is_recognized() {
    let tokens = tokenize_text("blocked by (#bug) right now");
    assert_eq!(
        tokens,
        vec![
            TitleToken::Plain("blocked by (".to_string()),
            TitleToken::Marker("#bug".to_string()),
            TitleToken::Plain(") right now".to_string()),
        ]
    );
}
