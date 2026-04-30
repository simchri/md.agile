use std::path::PathBuf;

// ── Status ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Todo,
    Done,
    Cancelled,
}

// ── Markers ───────────────────────────────────────────────────────────────────

// A single enum covers all marker kinds (#word and @word) so the checker can
// walk task.markers in one pass regardless of which kind it's looking for.
#[derive(Debug, Clone, PartialEq)]
pub enum Marker {
    Property(PropertyRef),
    Assignment(String), // the @name token; validated against mdagile.toml at check time
    Special(SpecialMarker),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PropertyRef {
    pub name: String,
    pub form: PropertyForm,
}

// PropertyForm carries the variant state so the checker can enforce rules
// directly: e.g. BranchPending && status == Done is always an error.
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyForm {
    Full,
    Short,                    // #feat_  — brainstorm mode; task cannot be marked Done
    BranchPending,            // #review...  — outcome not yet chosen
    BranchResolved(String),   // #review:passed  — branch name in the String
}

#[derive(Debug, Clone, PartialEq)]
pub enum SpecialMarker {
    Opt,       // #OPT — subtask does not block parent completion
    Milestone, // #MILESTONE — file-level divider; see FileItem
    MdAgile,   // #MDAGILE — file-level directive
}

// ── Ordering ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Order {
    None,
    Ranked(u32), // the "1." prefix; enforces execution sequence among siblings
}

// ── Subtask ───────────────────────────────────────────────────────────────────

// Quoted subtasks ("PO review") come from property definitions; unquoted ones
// are user-added. The checker needs this distinction to verify property
// conformance without re-scanning title text for quote characters.
#[derive(Debug, Clone, PartialEq)]
pub enum SubtaskKind {
    Custom,           // user-written, unquoted
    PropertyRequired, // quoted "", mandated by a Property declaration
}

// Subtask is recursive: both Task and Subtask use `children: Vec<Subtask>`
// for consistency. Task and Subtask are kept as separate types so the compiler
// prevents putting Order/SubtaskKind on a top-level Task where they have no
// meaning.
#[derive(Debug, Clone, PartialEq)]
pub struct Subtask {
    pub status:   Status,
    pub order:    Order,
    pub kind:     SubtaskKind,
    pub title:    String,
    pub body:     Vec<String>, // lines preserve structure for LSP range calculation
    pub markers:  Vec<Marker>,
    pub children: Vec<Subtask>,
}

// ── Task ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct Task {
    pub status:   Status,
    pub title:    String,
    pub body:     Vec<String>,
    pub markers:  Vec<Marker>,
    pub children: Vec<Subtask>,
}

// ── File-level items ──────────────────────────────────────────────────────────

// Milestones sit positionally *between* tasks in the file, so a flat
// Vec<FileItem> is the natural representation — no separate index needed.
#[derive(Debug, Clone, PartialEq)]
pub struct Milestone {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileItem {
    Task(Task),
    Milestone(Milestone),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskFile {
    pub path:  PathBuf,
    pub items: Vec<FileItem>,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Constructs the canonical vision-doc example:
    //   - [ ] #feature: add item to basket
    //     - [ ] "PO review"          ← property-required, mandatory
    //     - [ ] #OPT extra polish    ← optional subtask
    //     - [ ] 1. first step        ← ordered
    //     - [ ] implement @markus    ← assigned
    #[test]
    fn can_construct_task_with_all_node_kinds() {
        let task = Task {
            status: Status::Todo,
            title: "add item to basket".to_string(),
            body: vec![],
            markers: vec![Marker::Property(PropertyRef {
                name: "feature".to_string(),
                form: PropertyForm::Full,
            })],
            children: vec![
                Subtask {
                    status: Status::Todo,
                    order: Order::None,
                    kind: SubtaskKind::PropertyRequired,
                    title: "PO review".to_string(),
                    body: vec![],
                    markers: vec![],
                    children: vec![],
                },
                Subtask {
                    status: Status::Todo,
                    order: Order::None,
                    kind: SubtaskKind::Custom,
                    title: "extra polish".to_string(),
                    body: vec![],
                    markers: vec![Marker::Special(SpecialMarker::Opt)],
                    children: vec![],
                },
                Subtask {
                    status: Status::Todo,
                    order: Order::Ranked(1),
                    kind: SubtaskKind::Custom,
                    title: "first step".to_string(),
                    body: vec![],
                    markers: vec![],
                    children: vec![],
                },
                Subtask {
                    status: Status::Todo,
                    order: Order::None,
                    kind: SubtaskKind::Custom,
                    title: "implement".to_string(),
                    body: vec![],
                    markers: vec![Marker::Assignment("markus".to_string())],
                    children: vec![],
                },
            ],
        };
        assert_eq!(task.status, Status::Todo);
        assert_eq!(task.children.len(), 4);
    }

    #[test]
    fn file_items_interleave_tasks_and_milestones() {
        let items = vec![
            FileItem::Task(Task {
                status: Status::Done,
                title: "ship MVP".to_string(),
                body: vec![],
                markers: vec![],
                children: vec![],
            }),
            FileItem::Milestone(Milestone {
                name: "Release of MVP".to_string(),
            }),
            FileItem::Task(Task {
                status: Status::Todo,
                title: "gather feedback".to_string(),
                body: vec![],
                markers: vec![],
                children: vec![],
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
}
