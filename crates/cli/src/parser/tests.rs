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
        markers: vec![Marker::Property(PropertyRef {
            name: "feature".to_string(),
            form: PropertyForm::Full,
        })],
        children: vec![
            Subtask {
                location: loc(2),
                indent: 2,
                status: Status::Todo,
                order: Order::None,
                kind: SubtaskKind::PropertyRequired,
                title: "PO review".to_string(),
                body: vec![],
                markers: vec![],
                children: vec![],
                has_space_after_box: true,
                box_valid: true,
            },
            Subtask {
                location: loc(3),
                indent: 2,
                status: Status::Todo,
                order: Order::None,
                kind: SubtaskKind::Custom,
                title: "extra polish".to_string(),
                body: vec![],
                markers: vec![Marker::Special(SpecialMarker::Opt)],
                children: vec![],
                has_space_after_box: true,
                box_valid: true,
            },
            Subtask {
                location: loc(4),
                indent: 2,
                status: Status::Todo,
                order: Order::Ranked(1),
                kind: SubtaskKind::Custom,
                title: "first step".to_string(),
                body: vec![],
                markers: vec![],
                children: vec![],
                has_space_after_box: true,
                box_valid: true,
            },
            Subtask {
                location: loc(5),
                indent: 2,
                status: Status::Todo,
                order: Order::None,
                kind: SubtaskKind::Custom,
                title: "implement".to_string(),
                body: vec![],
                markers: vec![Marker::Assignment("markus".to_string())],
                children: vec![],
                has_space_after_box: true,
                box_valid: true,
            },
        ],
        has_space_after_box: true,
        box_valid: true,
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
            markers: vec![],
            children: vec![],
            has_space_after_box: true,
            box_valid: true,
        }),
        FileItem::Milestone(Milestone {
            name: "Release of MVP".to_string(),
        }),
        FileItem::Task(Task {
            location: loc(5),
            indent: 0,
            preceded_by_blank: true,
            status: Status::Todo,
            title: "gather feedback".to_string(),
            body: vec![],
            markers: vec![],
            children: vec![],
            has_space_after_box: true,
            box_valid: true,
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
    assert_eq!(t.title, "add basket");
    assert_eq!(
        t.markers,
        vec![Marker::Property(PropertyRef {
            name: "feature".to_string(),
            form: PropertyForm::Full,
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
    assert_eq!(t.title, "implement");
    assert_eq!(t.markers, vec![Marker::Assignment("markus".to_string())]);
}

#[test]
fn parse_opt_subtask() {
    let input = "\
- [ ] parent
  - [ ] #OPT optional thing
";
    let items = p(input);
    let sub = &task(&items, 0).children[0];
    assert_eq!(sub.title, "optional thing");
    assert_eq!(sub.markers, vec![Marker::Special(SpecialMarker::Opt)]);
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
    assert_eq!(children[0].order, Order::Ranked(1));
    assert_eq!(children[0].title, "first step");
    assert_eq!(children[1].order, Order::Ranked(2));
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
- [ ] perform #review...
";
    let items = p(input);
    let markers = &task(&items, 0).markers;
    assert_eq!(
        markers,
        &vec![Marker::Property(PropertyRef {
            name: "review".to_string(),
            form: PropertyForm::BranchPending,
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
