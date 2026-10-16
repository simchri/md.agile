use super::*;

// Tests that don't care about source location share this dummy path.
fn loc(line: usize) -> Location {
    Location {
        path: PathBuf::from("test.agile.md"),
        line,
    }
}

// Wrapper around `parse` so tests don't have to repeat the dummy path.
fn p(input: &str) -> Vec<FileItem> {
    parse(input, PathBuf::from("test.agile.md"))
}

// Constructs the canonical vision-doc example:
//   - [ ] #feature: add item to basket
//     - [ ] "PO review"          <- property-required, mandatory
//     - [ ] #OPT extra polish    <- optional subtask
//     - [ ] 1. first step        <- ordered
//     - [ ] implement @markus    <- assigned
#[test]
fn can_construct_task_with_all_node_kinds() {
    let task = Task {
        location: loc(1),
        indent: 0,
        preceded_by_blank: true,
        status: Status::Todo,
        title: "add item to basket".to_string(),
        body: vec![],
        fields: vec![],
        malformed_fields: vec![],
        markers: vec![Marker::Property(PropertyRef {
            name: "feature".to_string(),
            form: PropertyForm::Full,
            column: 1,
        })],
        children: vec![
            Subtask {
                location: loc(2),
                indent: 2,
                status: Status::Todo,
                order: Order::None,
                kind: SubtaskKind::PropertyRequired,
                raw_title: Some("PO review".to_string()),
                title: "PO review".to_string(),
                body: vec![],
                fields: vec![],
                malformed_fields: vec![],
                markers: vec![],
                children: vec![],
                parsing_issues: vec![],
            },
            Subtask {
                location: loc(3),
                indent: 2,
                status: Status::Todo,
                order: Order::None,
                kind: SubtaskKind::Custom,
                raw_title: None,
                title: "extra polish".to_string(),
                body: vec![],
                fields: vec![],
                malformed_fields: vec![],
                markers: vec![Marker::Special(SpecialMarker {
                    column: 1,
                    kind: SpecialMarkerKind::Opt,
                })],
                children: vec![],
                parsing_issues: vec![],
            },
            Subtask {
                location: loc(4),
                indent: 2,
                status: Status::Todo,
                order: Order::Ordered(1),
                kind: SubtaskKind::Custom,
                raw_title: None,
                title: "first step".to_string(),
                body: vec![],
                fields: vec![],
                malformed_fields: vec![],
                markers: vec![],
                children: vec![],
                parsing_issues: vec![],
            },
            Subtask {
                location: loc(5),
                indent: 2,
                status: Status::Todo,
                order: Order::None,
                kind: SubtaskKind::Custom,
                raw_title: None,
                title: "implement".to_string(),
                body: vec![],
                fields: vec![],
                malformed_fields: vec![],
                markers: vec![Marker::Assignment(AssignmentRef {
                    name: "markus".to_string(),
                    column: 11,
                })],
                children: vec![],
                parsing_issues: vec![],
            },
        ],
        parsing_issues: vec![],
    };
    assert_eq!(task.status, Status::Todo);
    assert_eq!(task.children.len(), 4);
}

#[test]
fn file_items_interleave_tasks_and_milestones() {
    let items = vec![
        FileItem::Task(Task {
            location: loc(1),
            indent: 0,
            preceded_by_blank: true,
            status: Status::Done,
            title: "ship MVP".to_string(),
            body: vec![],
            fields: vec![],
            malformed_fields: vec![],
            markers: vec![],
            children: vec![],
            parsing_issues: vec![],
        }),
        FileItem::Milestone(Milestone {
            name: "Release of MVP".to_string(),
            location: loc(3),
        }),
        FileItem::Task(Task {
            location: loc(5),
            indent: 0,
            preceded_by_blank: true,
            status: Status::Todo,
            title: "gather feedback".to_string(),
            body: vec![],
            fields: vec![],
            malformed_fields: vec![],
            markers: vec![],
            children: vec![],
            parsing_issues: vec![],
        }),
    ];
    assert_eq!(items.len(), 3);
    assert!(matches!(items[1], FileItem::Milestone(_)));
}

#[test]
fn branch_property_form_carries_resolved_branch_name() {
    let marker = Marker::Property(PropertyRef {
        name: "review".to_string(),
        form: PropertyForm::BranchResolved("passed".to_string()),
        column: 1,
    });
    if let Marker::Property(r) = marker {
        assert!(matches!(r.form, PropertyForm::BranchResolved(_)));
    }
}

// ── parse() tests ─────────────────────────────────────────────────────────

fn task(items: &[FileItem], i: usize) -> &Task {
    if let FileItem::Task(t) = &items[i] {
        t
    } else {
        panic!("item {i} is not a Task")
    }
}

#[test]
fn parse_todo_task() {
    let input = "\
- [ ] do the thing
";
    let items = p(input);
    assert_eq!(items.len(), 1);
    assert_eq!(task(&items, 0).status, Status::Todo);
    assert_eq!(task(&items, 0).title, "do the thing");
}

#[test]
fn parse_todo_task_missing_space() {
    let input = "\
- [] do the thing
";
    let items = p(input);
    assert_eq!(items.len(), 1);
    assert_eq!(task(&items, 0).status, Status::Todo);
    assert_eq!(task(&items, 0).title, "do the thing");
}

#[test]
fn parse_done_task() {
    let input = "\
- [x] finished
";
    let items = p(input);
    assert_eq!(task(&items, 0).status, Status::Done);
}

#[test]
fn parse_cancelled_task() {
    let input = "\
- [-] won't do
";
    let items = p(input);
    assert_eq!(task(&items, 0).status, Status::Cancelled);
    assert_eq!(task(&items, 0).title, "won't do");
}

#[test]
fn parse_task_with_subtask() {
    let input = "\
- [ ] parent
  - [x] child
";
    let items = p(input);
    assert_eq!(items.len(), 1);
    let t = task(&items, 0);
    assert_eq!(t.children.len(), 1);
    assert_eq!(t.children[0].status, Status::Done);
    assert_eq!(t.children[0].title, "child");
}

#[test]
fn parse_deeply_nested_subtasks() {
    let input = "\
- [ ] root
  - [ ] level1
    - [ ] level2
";
    let items = p(input);
    let root = task(&items, 0);
    assert_eq!(root.children.len(), 1);
    assert_eq!(root.children[0].title, "level1");
    assert_eq!(root.children[0].children.len(), 1);
    assert_eq!(root.children[0].children[0].title, "level2");
}

#[test]
fn parse_multiple_consecutive_tasks() {
    let input = "\
- [ ] task a
- [ ] task b
- [x] task c
";
    let items = p(input);
    assert_eq!(items.len(), 3);
    assert_eq!(task(&items, 0).title, "task a");
    assert_eq!(task(&items, 1).title, "task b");
    assert_eq!(task(&items, 2).status, Status::Done);
}

#[test]
fn parse_property_marker_in_title() {
    let input = "\
- [ ] #feature: add basket
";
    let items = p(input);
    let t = task(&items, 0);
    assert_eq!(t.title, "#feature: add basket");
    assert_eq!(
        t.markers,
        vec![Marker::Property(PropertyRef {
            name: "feature".to_string(),
            form: PropertyForm::Full,
            column: 1,
        })]
    );
}

#[test]
fn parse_assignment_marker_in_title() {
    let input = "\
- [ ] implement @markus
";
    let items = p(input);
    let t = task(&items, 0);
    assert_eq!(t.title, "implement @markus");
    assert_eq!(
        t.markers,
        vec![Marker::Assignment(AssignmentRef {
            name: "markus".to_string(),
            column: 11,
        })]
    );
}

#[test]
fn parse_opt_subtask() {
    let input = "\
- [ ] parent
  - [ ] #OPT optional thing
";
    let items = p(input);
    let sub = &task(&items, 0).children[0];
    assert_eq!(sub.title, "#OPT optional thing");
    assert_eq!(
        sub.markers,
        vec![Marker::Special(SpecialMarker {
            column: 1,
            kind: SpecialMarkerKind::Opt
        })]
    );
}

#[test]
fn parse_ordered_subtask() {
    let input = "\
- [ ] parent
  - [ ] 1. first step
  - [ ] 2. second step
";
    let items = p(input);
    let children = &task(&items, 0).children;
    assert_eq!(children[0].order, Order::Ordered(1));
    // The "N. " prefix stays inline in the displayed title — like `#`/`@`
    // markers, it's detected into `Order` but not stripped out — so it's
    // still visible wherever the title is rendered, e.g. `agile task next`.
    assert_eq!(children[0].title, "1. first step");
    assert_eq!(children[1].order, Order::Ordered(2));
}

#[test]
fn parse_property_required_subtask() {
    let input = "\
- [ ] parent
  - [ ] \"PO review\"
";
    let items = p(input);
    let sub = &task(&items, 0).children[0];
    assert_eq!(sub.kind, SubtaskKind::PropertyRequired);
    assert_eq!(sub.title, "PO review");
    assert_eq!(sub.raw_title, Some("PO review".to_string()));
}

#[test]
fn parse_property_required_subtask_with_embedded_property_stores_raw_title() {
    let input = "\
- [ ] parent
  - [ ] \"developer #review\"
";
    let items = p(input);
    let sub = &task(&items, 0).children[0];
    assert_eq!(sub.kind, SubtaskKind::PropertyRequired);
    // raw_title preserves the full inner text before marker extraction
    assert_eq!(sub.raw_title, Some("developer #review".to_string()));
    // title keeps the marker text inline for display purposes
    assert_eq!(sub.title, "developer #review");
}

#[test]
fn parse_property_required_subtask_with_trailing_assignment_marker() {
    // Dynamic per-instance assignment (Option 1 from
    // doc/development/dynamic_assignment_mandatory_subtasks.md): a marker placed *after*
    // the closing quote is an ordinary marker on the subtask, and does not
    // affect the byte-exact raw_title used for config matching. It's still
    // appended to the display `title`, though, so renderers show it inline
    // (like `Custom` subtasks already do for their markers).
    let input = "\
- [ ] parent
  - [ ] \"PO review\" @alice
";
    let items = p(input);
    let sub = &task(&items, 0).children[0];
    assert_eq!(sub.kind, SubtaskKind::PropertyRequired);
    assert_eq!(sub.raw_title, Some("PO review".to_string()));
    assert_eq!(sub.title, "PO review @alice");
    assert_eq!(
        sub.markers,
        vec![Marker::Assignment(AssignmentRef {
            name: "alice".to_string(),
            column: 13,
        })]
    );
}

#[test]
fn parse_property_required_subtask_with_multiple_trailing_markers() {
    let input = "\
- [ ] parent
  - [ ] \"PO review\" @alice #OPT
";
    let items = p(input);
    let sub = &task(&items, 0).children[0];
    assert_eq!(sub.kind, SubtaskKind::PropertyRequired);
    assert_eq!(sub.raw_title, Some("PO review".to_string()));
    assert!(
        sub.markers
            .iter()
            .any(|m| matches!(m, Marker::Assignment(a) if a.name == "alice"))
    );
    assert!(
        sub.markers
            .iter()
            .any(|m| matches!(m, Marker::Special(s) if s.kind == SpecialMarkerKind::Opt))
    );
}

#[test]
fn custom_subtask_has_no_raw_title() {
    let input = "\
- [ ] parent
  - [ ] just a task
";
    let items = p(input);
    let sub = &task(&items, 0).children[0];
    assert_eq!(sub.kind, SubtaskKind::Custom);
    assert_eq!(sub.raw_title, None);
}

#[test]
fn parse_milestone() {
    let input = "\
#MILESTONE: Release of MVP
";
    let items = p(input);
    assert_eq!(items.len(), 1);
    assert!(matches!(&items[0], FileItem::Milestone(m) if m.name == "Release of MVP"));
}

#[test]
fn milestone_tag_glued_to_alphanumeric_suffix_is_not_recognized() {
    // "#MILESTONEfoo" has no punctuation/whitespace boundary after the tag,
    // so it must NOT be misread as a milestone named "foo" -- it should be
    // treated as ordinary (silently ignored) prose, per the parser's
    // documented handling of non-task content.
    let input = "\
#MILESTONEfoo
";
    let items = p(input);
    assert_eq!(
        items.len(),
        0,
        "expected no milestone/task parsed, got: {items:?}"
    );
}

#[test]
fn milestone_tag_with_punctuation_boundary_is_still_recognized() {
    // Sanity check alongside the glued-suffix test above: punctuation
    // directly after the tag (no space) must still work per the spec.
    let input = "\
#MILESTONE!Release
";
    let items = p(input);
    assert_eq!(items.len(), 1);
    assert!(matches!(&items[0], FileItem::Milestone(m) if m.name == "Release"));
}

#[test]
fn bare_milestone_tag_with_no_name_is_still_recognized_with_an_empty_name() {
    // "A milestone name must be provided" (README.md) is enforced as a lint
    // (E018, `rules::missing_milestone_name`), not silently dropped by the
    // parser -- so a nameless `#MILESTONE`/`#MILESTONE:` must still produce a
    // `FileItem::Milestone` (with an empty name) for the rule to flag,
    // rather than falling through as ordinary ignored prose.
    let input = "\
#MILESTONE
";
    let items = p(input);
    assert_eq!(items.len(), 1);
    assert!(matches!(&items[0], FileItem::Milestone(m) if m.name.is_empty()));
}

#[test]
fn bare_milestone_tag_with_colon_and_no_name_is_still_recognized_with_an_empty_name() {
    let input = "\
#MILESTONE:
";
    let items = p(input);
    assert_eq!(items.len(), 1);
    assert!(matches!(&items[0], FileItem::Milestone(m) if m.name.is_empty()));
}

#[test]
fn parse_tasks_with_milestone_between() {
    let input = "\
- [x] ship MVP

#MILESTONE: Release of MVP

- [ ] gather feedback
";
    let items = p(input);
    assert_eq!(items.len(), 3);
    assert!(matches!(&items[0], FileItem::Task(_)));
    assert!(matches!(&items[1], FileItem::Milestone(_)));
    assert!(matches!(&items[2], FileItem::Task(_)));
}

#[test]
fn parse_branch_pending_marker() {
    let input = "\
- [ ] perform #review:...
";
    let items = p(input);
    let markers = &task(&items, 0).markers;
    assert_eq!(
        markers,
        &vec![Marker::Property(PropertyRef {
            name: "review".to_string(),
            form: PropertyForm::BranchPending,
            column: 9,
        })]
    );
}

#[test]
fn parse_bare_dots_without_a_colon_is_a_plain_property_not_a_pending_branch() {
    // The old (pre-colon) pending syntax `#review...` no longer means
    // "pending" -- the trailing dots are just trimmed as trailing
    // punctuation (see `MARKER_TRAILING_PUNCT`), leaving a plain property.
    // (Whether a property that declares branches is then used validly is a
    // checker concern -- see E023 -- not the parser's.)
    let input = "\
- [ ] perform #review...
";
    let items = p(input);
    let markers = &task(&items, 0).markers;
    assert_eq!(
        markers,
        &vec![Marker::Property(PropertyRef {
            name: "review".to_string(),
            form: PropertyForm::Full,
            column: 9,
        })]
    );
}

#[test]
fn parse_branch_resolved_marker() {
    let input = "\
- [x] perform #review:passed
";
    let items = p(input);
    let markers = &task(&items, 0).markers;
    assert_eq!(
        markers,
        &vec![Marker::Property(PropertyRef {
            name: "review".to_string(),
            form: PropertyForm::BranchResolved("passed".to_string()),
            column: 9,
        })]
    );
}

#[test]
fn parse_task_body_lines() {
    let input = "\
- [ ] do the thing
some details here
more info

- [ ] next task
";
    let items = p(input);
    assert_eq!(items.len(), 2);
    assert_eq!(task(&items, 0).body, vec!["some details here", "more info"]);
    assert!(task(&items, 1).body.is_empty());
}

#[test]
fn parse_single_line_field() {
    let input = "\
- [ ] new colleague: John #onboarding
  __First Name:__ John
  __Email:__ john.doe@gmail.com
";
    let items = p(input);
    let t = task(&items, 0);
    assert_eq!(
        t.fields,
        vec![
            FieldLine {
                label: "First Name".to_string(),
                value: "John".to_string(),
                location: loc(2),
            },
            FieldLine {
                label: "Email".to_string(),
                value: "john.doe@gmail.com".to_string(),
                location: loc(3),
            },
        ]
    );
}

#[test]
fn parse_single_line_field_with_unfilled_placeholder_value() {
    let input = "\
- [ ] new colleague: John #onboarding
  __Gender:__ ___
";
    let items = p(input);
    let t = task(&items, 0);
    assert_eq!(
        t.fields,
        vec![FieldLine {
            label: "Gender".to_string(),
            value: "___".to_string(),
            location: loc(2),
        }]
    );
}

#[test]
fn bare_label_with_no_trailing_value_opens_a_multiline_field() {
    let input = "\
- [ ] new colleague: John #onboarding
  __Bio:__
  John doe is an outstanding character.
";
    let items = p(input);
    let t = task(&items, 0);
    assert_eq!(
        t.fields,
        vec![FieldLine {
            label: "Bio".to_string(),
            value: "  John doe is an outstanding character.".to_string(),
            location: loc(2),
        }]
    );
    // The raw body lines are kept unchanged alongside the parsed field.
    assert_eq!(
        t.body,
        vec!["  __Bio:__", "  John doe is an outstanding character."]
    );
}

#[test]
fn multiline_field_stops_at_closing_ellipsis_marker() {
    let input = "\
- [ ] new colleague: John #onboarding
  __Bio:__
  John doe is an outstanding character.
  He is well known for his many appearances in sample forms.
  ...
  More notes on the task can go here. This sentence is not part of any field.
";
    let items = p(input);
    let t = task(&items, 0);
    assert_eq!(
        t.fields,
        vec![FieldLine {
            label: "Bio".to_string(),
            value: "  John doe is an outstanding character.\n  He is well known for his many appearances in sample forms.".to_string(),
            location: loc(2),
        }]
    );
}

#[test]
fn multiline_field_keeps_a_literal_ellipsis_line_via_escape() {
    let input = "\
- [ ] new colleague: John #onboarding
  __Bio:__
  Before the escaped line.
  \\...
  After the escaped line.
  ...
  Not part of the field.
";
    let items = p(input);
    let t = task(&items, 0);
    assert_eq!(
        t.fields,
        vec![FieldLine {
            label: "Bio".to_string(),
            value: "  Before the escaped line.\n  ...\n  After the escaped line.".to_string(),
            location: loc(2),
        }]
    );
}

#[test]
fn multiline_field_stops_at_the_next_field_line_without_a_closing_ellipsis() {
    let input = "\
- [ ] new colleague: John #onboarding
  __Bio:__
  John doe is an outstanding character.
  __Email:__ john.doe@gmail.com
";
    let items = p(input);
    let t = task(&items, 0);
    assert_eq!(
        t.fields,
        vec![
            FieldLine {
                label: "Bio".to_string(),
                value: "  John doe is an outstanding character.".to_string(),
                location: loc(2),
            },
            FieldLine {
                label: "Email".to_string(),
                value: "john.doe@gmail.com".to_string(),
                location: loc(4),
            },
        ]
    );
}

#[test]
fn multiline_field_stops_at_the_start_of_another_multiline_field() {
    let input = "\
- [ ] new colleague: John #onboarding
  __Bio:__
  First field's content.
  __Notes:__
  Second field's content.
";
    let items = p(input);
    let t = task(&items, 0);
    assert_eq!(
        t.fields,
        vec![
            FieldLine {
                label: "Bio".to_string(),
                value: "  First field's content.".to_string(),
                location: loc(2),
            },
            FieldLine {
                label: "Notes".to_string(),
                value: "  Second field's content.".to_string(),
                location: loc(4),
            },
        ]
    );
}

#[test]
fn multiline_field_with_no_closing_marker_captures_to_the_end_of_the_body() {
    let input = "\
- [ ] new colleague: John #onboarding
  __Bio:__
  Line one.
  Line two.
";
    let items = p(input);
    let t = task(&items, 0);
    assert_eq!(
        t.fields,
        vec![FieldLine {
            label: "Bio".to_string(),
            value: "  Line one.\n  Line two.".to_string(),
            location: loc(2),
        }]
    );
}

#[test]
fn multiline_field_with_nothing_after_it_has_an_empty_value() {
    let input = "\
- [ ] new colleague: John #onboarding
  __Bio:__
";
    let items = p(input);
    let t = task(&items, 0);
    assert_eq!(
        t.fields,
        vec![FieldLine {
            label: "Bio".to_string(),
            value: String::new(),
            location: loc(2),
        }]
    );
}

#[test]
fn field_line_on_subtask_is_parsed_too() {
    let input = "\
- [ ] parent
  - [ ] sub #onboarding
    __Email:__ a@b.com
";
    let items = p(input);
    let t = task(&items, 0);
    assert_eq!(
        t.children[0].fields,
        vec![FieldLine {
            label: "Email".to_string(),
            value: "a@b.com".to_string(),
            location: loc(3),
        }]
    );
}

#[test]
fn non_field_body_text_is_unaffected() {
    let input = "\
- [ ] do the thing
some details here
more info
";
    let items = p(input);
    assert!(task(&items, 0).fields.is_empty());
}

// ── field parsing edge cases ─────────────────────────────────────────────────

#[test]
fn label_marker_without_a_following_space_is_recognized_as_a_malformed_field() {
    // Missing the single required space after `:__` -- neither the
    // single-line nor the multiline grammar matches (both require either a
    // space or nothing at all after the marker), but it's still recognized
    // as a *malformed* field (not an ordinary `fields` entry, not silently
    // treated as plain body text either).
    let input = "\
- [ ] new colleague: John #onboarding
  __Email:__john.doe@gmail.com
";
    let items = p(input);
    let t = task(&items, 0);
    assert!(t.fields.is_empty());
    assert_eq!(
        t.malformed_fields,
        vec![MalformedFieldLine {
            label: "Email".to_string(),
            location: loc(2),
        }]
    );
    // The raw body line is kept unchanged alongside the parsed malformed field.
    assert_eq!(t.body, vec!["  __Email:__john.doe@gmail.com"]);
}

#[test]
fn malformed_field_with_empty_label_is_not_recognized() {
    let input = "\
- [ ] new colleague: John #onboarding
  __:__value
";
    let items = p(input);
    let t = task(&items, 0);
    assert!(t.fields.is_empty());
    assert!(t.malformed_fields.is_empty());
}

#[test]
fn malformed_field_on_subtask_is_parsed_too() {
    let input = "\
- [ ] parent
  - [ ] sub #onboarding
    __Email:__a@b.com
";
    let items = p(input);
    let t = task(&items, 0);
    assert_eq!(
        t.children[0].malformed_fields,
        vec![MalformedFieldLine {
            label: "Email".to_string(),
            location: loc(3),
        }]
    );
}

#[test]
fn malformed_field_stops_an_in_progress_multiline_field_capture() {
    // A malformed line also ends a multiline field's capture (without being
    // consumed by it), same as a valid single-line/multiline field line
    // would -- it's re-processed by the outer loop as its own (malformed)
    // field on the next iteration.
    let input = "\
- [ ] new colleague: John #onboarding
  __Bio:__
  First field's content.
  __Email:__a@b.com
";
    let items = p(input);
    let t = task(&items, 0);
    assert_eq!(
        t.fields,
        vec![FieldLine {
            label: "Bio".to_string(),
            value: "  First field's content.".to_string(),
            location: loc(2),
        }]
    );
    assert_eq!(
        t.malformed_fields,
        vec![MalformedFieldLine {
            label: "Email".to_string(),
            location: loc(4),
        }]
    );
}

#[test]
fn label_marker_with_only_a_trailing_space_is_a_single_line_field_with_empty_value() {
    // Exactly one trailing space and nothing else still matches the
    // single-line grammar (not the bare-label multiline grammar), yielding
    // an empty value rather than opening a multiline capture.
    let input = "\
- [ ] new colleague: John #onboarding
  __Gender:__ 
  __Email:__ a@b.com
";
    let items = p(input);
    let t = task(&items, 0);
    assert_eq!(
        t.fields,
        vec![
            FieldLine {
                label: "Gender".to_string(),
                value: String::new(),
                location: loc(2),
            },
            FieldLine {
                label: "Email".to_string(),
                value: "a@b.com".to_string(),
                location: loc(3),
            },
        ]
    );
}

#[test]
fn empty_label_is_not_recognized_as_a_single_line_field() {
    let input = "\
- [ ] new colleague: John #onboarding
  __:__ value
";
    let items = p(input);
    let t = task(&items, 0);
    assert!(t.fields.is_empty());
}

#[test]
fn empty_label_is_not_recognized_as_a_multiline_field() {
    let input = "\
- [ ] new colleague: John #onboarding
  __:__
  some content
";
    let items = p(input);
    let t = task(&items, 0);
    assert!(t.fields.is_empty());
}

#[test]
fn label_without_a_closing_marker_is_not_recognized_as_a_field() {
    // `__Bio` has no closing `:__` at all -- plain body text.
    let input = "\
- [ ] new colleague: John #onboarding
  __Bio
  some content
";
    let items = p(input);
    let t = task(&items, 0);
    assert!(t.fields.is_empty());
}

#[test]
fn duplicate_field_labels_are_both_captured_independently() {
    // The parser does not deduplicate repeated labels within a node's body
    // -- both `FieldLine`s are kept, in source order. (It's up to a
    // not-yet-implemented check to flag this as a config/authoring mistake,
    // if ever desired; the parser itself stays purely structural.)
    let input = "\
- [ ] new colleague: John #onboarding
  __Email:__ first@example.com
  __Email:__ second@example.com
";
    let items = p(input);
    let t = task(&items, 0);
    assert_eq!(
        t.fields,
        vec![
            FieldLine {
                label: "Email".to_string(),
                value: "first@example.com".to_string(),
                location: loc(2),
            },
            FieldLine {
                label: "Email".to_string(),
                value: "second@example.com".to_string(),
                location: loc(3),
            },
        ]
    );
}

#[test]
fn multiline_field_may_start_with_an_escaped_ellipsis_line() {
    // The very first captured line is itself an escaped `\...` -- it should
    // still be unescaped into a literal "..." without prematurely closing
    // the field.
    let input = "\
- [ ] new colleague: John #onboarding
  __Bio:__
  \\...
  real content after the literal ellipsis.
  ...
";
    let items = p(input);
    let t = task(&items, 0);
    assert_eq!(
        t.fields,
        vec![FieldLine {
            label: "Bio".to_string(),
            value: "  ...\n  real content after the literal ellipsis.".to_string(),
            location: loc(2),
        }]
    );
}

#[test]
fn field_line_with_deeper_indentation_than_the_task_is_still_recognized() {
    // Recognition is purely content-based (via trim), independent of how
    // deeply the line happens to be indented.
    let input = "\
- [ ] new colleague: John #onboarding
      __Email:__ a@b.com
";
    let items = p(input);
    let t = task(&items, 0);
    assert_eq!(
        t.fields,
        vec![FieldLine {
            label: "Email".to_string(),
            value: "a@b.com".to_string(),
            location: loc(2),
        }]
    );
}

#[test]
fn parse_records_task_locations() {
    let input = "\
# heading

- [x] done
- [ ] active
  - [ ] sub
";
    let path = PathBuf::from("/abs/file.agile.md");
    let items = parse(input, path.clone());
    let t0 = task(&items, 0);
    let t1 = task(&items, 1);
    assert_eq!(
        t0.location,
        Location {
            path: path.clone(),
            line: 3
        }
    );
    assert_eq!(
        t1.location,
        Location {
            path: path.clone(),
            line: 4
        }
    );
    assert_eq!(t1.children[0].location, Location { path, line: 5 });
}

#[test]
fn parse_records_source_indent() {
    let input = "\
- [ ] top
  - [ ] sub
    - [ ] deeper
";
    let items = p(input);
    let t = task(&items, 0);
    assert_eq!(t.indent, 0);
    assert_eq!(t.children[0].indent, 2);
    assert_eq!(t.children[0].children[0].indent, 4);
}

#[test]
fn parse_keeps_indent_for_orphaned_indented_task() {
    // The `- [ ] orphan` line is indented like a subtask, but the
    // preceding blank line breaks the parent-child chain, so the
    // parser produces it as a top-level Task with indent > 0 — that
    // is exactly the "wrongly indented" case the checker will flag.
    let input = "\
- [ ] real top level

  - [ ] orphan indented
";
    let items = p(input);
    assert_eq!(items.len(), 2);
    let orphan = task(&items, 1);
    assert_eq!(orphan.indent, 2);
    assert_eq!(orphan.title, "orphan indented");
}

#[test]
fn parse_empty_box_style() {
    let input = "\
- [] empty box 
";
    let items = p(input);
    let task = task(&items, 0);
    assert_eq!(task.title, "empty box");
}

#[test]
fn parse_invalid_box_style() {
    let input = "\
- [R] empty box
";
    let items = p(input);
    let task = task(&items, 0);
    assert_eq!(task.title, "empty box");
}

#[test]
fn parse_uppercase_x_sets_flag() {
    let input = "\
- [X] task title
";
    let items = p(input);
    let t = task(&items, 0);
    assert!(t.parsing_issues.contains(&ParsingIssue::UppercaseX));
    assert!(!t.parsing_issues.contains(&ParsingIssue::InvalidBox));
    assert_eq!(t.title, "task title");
}

#[test]
fn markers_in_parens_are_detected() {
    // (@bob) and (#myprop) should be parsed as assignment / property markers
    let input = "\
- [ ] some task (@bob) (#myprop)
";
    let items = p(input);
    let t = task(&items, 0);
    assert!(
        t.markers
            .iter()
            .any(|m| matches!(m, Marker::Assignment(a) if a.name == "bob")),
        "expected @bob assignment, got {:?}",
        t.markers
    );
    assert!(
        t.markers
            .iter()
            .any(|m| matches!(m, Marker::Property(p) if p.name == "myprop")),
        "expected #myprop property, got {:?}",
        t.markers
    );
}

#[test]
fn marker_attached_to_preceding_word_is_detected() {
    // asdf#marker — the '#' is not space-separated from the preceding word
    let input = "\
- [ ] asdf#myprop
";
    let items = p(input);
    let t = task(&items, 0);
    assert!(
        t.markers
            .iter()
            .any(|m| matches!(m, Marker::Property(p) if p.name == "myprop")),
        "expected #myprop property, got {:?}",
        t.markers
    );
}

#[test]
fn all_three_inline_markers_in_user_example() {
    // The sample from the bug report: three markers not all space-separated
    let input = "\
- [ ] (@bob) (#someundefproperty) asdf#anotherundefprop
";
    let items = p(input);
    let t = task(&items, 0);
    assert!(
        t.markers
            .iter()
            .any(|m| matches!(m, Marker::Assignment(a) if a.name == "bob")),
        "expected @bob, got {:?}",
        t.markers
    );
    assert!(
        t.markers
            .iter()
            .any(|m| matches!(m, Marker::Property(p) if p.name == "someundefproperty")),
        "expected #someundefproperty, got {:?}",
        t.markers
    );
    assert!(
        t.markers
            .iter()
            .any(|m| matches!(m, Marker::Property(p) if p.name == "anotherundefprop")),
        "expected #anotherundefprop, got {:?}",
        t.markers
    );
}

#[test]
fn marker_wrapped_in_single_ticks_is_not_detected() {
    // '#feat' and '@alice' — fully wrapped in single ticks, so both are
    // treated as prose, not markers.
    let input = "\
- [ ] refer to '#feat' and '@alice' in prose
";
    let items = p(input);
    let t = task(&items, 0);
    assert!(
        t.markers.is_empty(),
        "expected no markers, got {:?}",
        t.markers
    );
}

#[test]
fn marker_wrapped_in_double_quotes_is_still_detected() {
    // Double quotes have no escaping effect at all — "@alice" and "#feat"
    // are real markers, unlike single-tick wrapping.
    let input = "\
- [ ] refer to \"#feat\" and \"@alice\" in prose
";
    let items = p(input);
    let t = task(&items, 0);
    assert!(
        t.markers
            .iter()
            .any(|m| matches!(m, Marker::Property(p) if p.name == "feat")),
        "expected #feat, got {:?}",
        t.markers
    );
    assert!(
        t.markers
            .iter()
            .any(|m| matches!(m, Marker::Assignment(a) if a.name == "alice")),
        "expected @alice, got {:?}",
        t.markers
    );
}

#[test]
fn marker_preceded_by_lone_unmatched_tick_is_still_detected() {
    // A single tick with no matching closing tick right after the marker
    // name must NOT suppress recognition — only a full `'...'` wrap does.
    let input = "\
- [ ] weird'#feat and weird'@alice without closing ticks
";
    let items = p(input);
    let t = task(&items, 0);
    assert!(
        t.markers
            .iter()
            .any(|m| matches!(m, Marker::Property(p) if p.name == "feat")),
        "expected #feat, got {:?}",
        t.markers
    );
    assert!(
        t.markers
            .iter()
            .any(|m| matches!(m, Marker::Assignment(a) if a.name == "alice")),
        "expected @alice, got {:?}",
        t.markers
    );
}

#[test]
fn marker_inside_backtick_code_span_is_not_detected() {
    let input = "\
- [ ] explain `#feat @alice` in prose
";
    let items = p(input);
    let t = task(&items, 0);
    assert!(
        t.markers.is_empty(),
        "expected no markers inside code span, got {:?}",
        t.markers
    );
}

#[test]
fn backslash_escaped_hash_is_not_a_marker() {
    let input = "\
- [ ] this is \\#not_a_property in prose
";
    let items = p(input);
    let t = task(&items, 0);
    assert!(
        t.markers.is_empty(),
        "expected no markers, got {:?}",
        t.markers
    );
}

#[test]
fn backslash_escaped_at_is_not_a_marker() {
    let input = "\
- [ ] this is \\@not_an_assignment in prose
";
    let items = p(input);
    let t = task(&items, 0);
    assert!(
        t.markers.is_empty(),
        "expected no markers, got {:?}",
        t.markers
    );
}

#[test]
fn backslash_escaped_marker_strips_the_backslash_from_the_title() {
    let input = "\
- [ ] this is \\#not_a_property in prose
";
    let items = p(input);
    let t = task(&items, 0);
    assert_eq!(t.title, "this is #not_a_property in prose");
}

#[test]
fn unescaped_marker_after_escaped_marker_is_still_parsed() {
    let input = "\
- [ ] \\#not_a_property but #feature is real
";
    let items = p(input);
    let t = task(&items, 0);
    assert_eq!(
        t.markers,
        vec![Marker::Property(PropertyRef {
            name: "feature".to_string(),
            form: PropertyForm::Full,
            column: 22,
        })]
    );
    assert_eq!(t.title, "#not_a_property but #feature is real");
}

#[test]
fn property_required_subtask_with_order_prefix_is_detected_as_ranked() {
    // Vision doc "Ordered Tasks via Properties": a property's `subtasks` config
    // entry can bake an order prefix into the literal quoted string, e.g.
    // `subtasks = ["1. dev implementation", "2. dev documentation"]`. The
    // `Order` must still be detected so E014/E015 can validate it, even
    // though the whole thing is wrapped in quotes.
    let input = "\
- [ ] parent
  - [ ] \"1. dev implementation\"
";
    let items = p(input);
    let sub = &task(&items, 0).children[0];
    assert_eq!(sub.kind, SubtaskKind::PropertyRequired);
    assert_eq!(sub.order, Order::Ordered(1));
}

#[test]
fn property_required_subtask_with_order_prefix_keeps_full_raw_title_for_config_matching() {
    // raw_title must stay byte-identical to the config's declared subtask
    // string (order prefix included), since E010/E012 match on it literally.
    let input = "\
- [ ] parent
  - [ ] \"1. dev implementation\"
";
    let items = p(input);
    let sub = &task(&items, 0).children[0];
    assert_eq!(sub.raw_title, Some("1. dev implementation".to_string()));
}

#[test]
fn custom_subtask_order_prefix_is_kept_inline_like_markers() {
    // The "N. " prefix on a Custom (unquoted) ranked subtask is detected into
    // `Order` but no longer stripped out of the displayed title — mirroring
    // how `#`/`@` markers are kept inline instead of stripped, so the
    // ordering is still visible wherever the title is rendered (e.g. `agile
    // task next`).
    let input = "\
- [ ] parent
  - [ ] 1. add performance UI test
";
    let items = p(input);
    let sub = &task(&items, 0).children[0];
    assert_eq!(sub.kind, SubtaskKind::Custom);
    assert_eq!(sub.order, Order::Ordered(1));
    assert_eq!(sub.title, "1. add performance UI test");
}
