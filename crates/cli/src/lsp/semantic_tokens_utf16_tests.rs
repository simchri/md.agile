use super::*;
use crate::parser::parse;
use std::path::PathBuf;

#[test]
fn semantic_tokens_use_utf16_offsets_and_lengths() {
    let file_content = "\
- [ ] 😀 #féat @a😀ce
";
    let items = parse(file_content, PathBuf::from("test.agile.md"));
    let tokens = build_tokens(&items, file_content);

    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].delta_start, 9);
    assert_eq!(tokens[0].length, 5); // #féat
    assert_eq!(tokens[0].token_type, PROPERTY);
    assert_eq!(tokens[1].delta_start, 6);
    assert_eq!(tokens[1].length, 6); // @a😀ce
    assert_eq!(tokens[1].token_type, PARAMETER);
}

#[test]
fn special_marker_after_unicode_has_utf16_start() {
    let file_content = "\
- [ ] 😀 #OPT
";
    let items = parse(file_content, PathBuf::from("test.agile.md"));
    let tokens = build_tokens(&items, file_content);

    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].delta_start, 9);
    assert_eq!(tokens[0].length, 4);
    assert_eq!(tokens[0].token_type, KEYWORD);
}
