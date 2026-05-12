use tower_lsp::lsp_types::*;

/// E008: Add missing property definition to mdagile.toml
pub fn build(diagnostic: &Diagnostic, _doc_text: &str, uri: &Url) -> Option<CodeAction> {
    let issue_data = super::issue_data(diagnostic)?;
    let property_name = match issue_data {
        crate::rules::IssueData::UndefinedProperty { property_name } => property_name,
        _ => return None,
    };

    // Derive the file path from the URI
    let file_path = uri.to_file_path().ok()?;

    // Find mdagile.toml by walking up the directory tree
    let mut dir = file_path.parent()?;
    let toml_path = loop {
        let plain = dir.join("mdagile.toml");
        let dot = dir.join(".mdagile.toml");
        if plain.exists() {
            break plain;
        }
        if dot.exists() {
            break dot;
        }
        dir = dir.parent()?;
    };

    // Read the current toml file
    let current_content = std::fs::read_to_string(&toml_path).unwrap_or_default();

    // Build the new content: append the new property section
    let new_content = if current_content.is_empty() {
        format!("[Properties.{}]\n", property_name)
    } else {
        let mut content = current_content;
        if !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&format!("[Properties.{}]\n", property_name));
        content
    };

    // Create a URI for mdagile.toml
    let toml_uri = Url::from_file_path(&toml_path).ok()?;

    // Create a TextEdit that replaces the entire file content
    let edit = TextEdit {
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: u32::MAX,
                character: u32::MAX,
            },
        },
        new_text: new_content,
    };

    Some(super::make_quickfix(
        format!("Add '[Properties.{}]' to mdagile.toml", property_name),
        &toml_uri,
        diagnostic,
        edit,
    ))
}
