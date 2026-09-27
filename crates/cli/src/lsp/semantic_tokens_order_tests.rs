use super::*;
use crate::parser::parse;
use std::path::PathBuf;

#[test]
fn ordered_subtasks_highlight_only_digits_and_interleave_with_markers() {
    let file_content = "\
- [ ] parent
  - [ ] 12. work #feature
    - [ ] \"3. review\" @alice
";
    let items = parse(file_content, PathBuf::from("test.agile.md"));
    let tokens = build_tokens(&items, file_content);

    assert_eq!(TOKEN_TYPES[NUMBER as usize], SemanticTokenType::NUMBER);
    assert_eq!(tokens.len(), 4);
    assert_eq!(
        (
            tokens[0].delta_line,
            tokens[0].delta_start,
            tokens[0].length,
            tokens[0].token_type
        ),
        (1, 8, 2, NUMBER)
    );
    assert_eq!(
        (
            tokens[1].delta_line,
            tokens[1].delta_start,
            tokens[1].length,
            tokens[1].token_type
        ),
        (0, 9, 8, PROPERTY)
    );
    assert_eq!(
        (
            tokens[2].delta_line,
            tokens[2].delta_start,
            tokens[2].length,
            tokens[2].token_type
        ),
        (1, 11, 1, NUMBER)
    );
    assert_eq!(
        (
            tokens[3].delta_line,
            tokens[3].delta_start,
            tokens[3].length,
            tokens[3].token_type
        ),
        (0, 11, 6, PARAMETER)
    );
}

#[test]
fn unordered_titles_and_invalid_order_prefixes_do_not_get_number_tokens() {
    let file_content = "\
- [ ] 1. top-level title
  - [ ] work 2. within title
  - [ ] 3.no space
  - [ ] 4294967296. overflow
";
    let items = parse(file_content, PathBuf::from("test.agile.md"));
    assert!(build_tokens(&items, file_content).is_empty());
}
