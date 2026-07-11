# Md.Agile

Plain-text, version-controlled task management for developers.
Your tasks live forever in a simple text file, version-controlled directly alongside your code (ideally in the same repository) – not a web app!

> This project is >> under construction <<  - more features to come. (c.f. [README.vision.md](README.vision.md))

**tasks.md:**

```md
- [ ] a task - this is the task-title
  Some more info on this task - this is the task-body
  Both inside and outside of tasks, you can just use normal markdown syntax
  - [ ] a subtask @markus
    more details for this subtask go here.
  - [x] this subtask is done
```

Tasks follow a specific syntax. You will receive immediate feedback in your text editor if you make a mistake (via Language Server). Use the language server's auto-fix feature for an ergonomic experience. Use the CLI tool to add strict checks as pre-commit hooks or in your pipeline – your task list is always consistent. Everything is designed with a "command line first" approach: text files and CLI tools are the primary interface. Graphical client for convenient "board view" (Currently only a viewer, no edits possible).

Simple animated board

<img width="400" height="auto" alt="animated_board" src="https://github.com/user-attachments/assets/476af559-b4fb-4558-8f09-258a97f1e176" />

Formatting rules enforced via language server with quick fixes!

<img width="400" height="auto" alt="quickfix" src="https://github.com/user-attachments/assets/aaecaa6e-a735-4ae4-98b3-90da54981000" />

## Installation

See [INSTALL.md](INSTALL.md) for building/installing the CLI, language server, and editor setup (currently source-only via `cargo install`).

## Basic Syntax

Tasks must follow the Markdown standard, and subtasks must be correctly indented. A task that is "done" is marked with a lowercase `x`.

Correct:
```md
- [ ] a task
  - [ ] a subtask
  - [x] this subtask is done

- [ ] another task
```

Incorrect:
```md
- [ ] a task
  - [ ] a subtask

  - [ ] is this a subtask or a task? Nobody knows.
```

Other content is ignored. You can therefore freely complement your task lists with other notes. A newline separates tasks from other tasks or content.

```md
- [x] a task
  - [x] a subtask

# A Markdown Heading
And some further notes

- [ ] more tasks
```

mdagile uses the following symbols for syntax: `# @ \`
If necessary, escape these with `\`

```md
- [ ] a task with a hashtag \#not_a_property
- [ ] a task with a mail address: markus\@company.org
```

## Prioritization

Prioritization is fully reflected by the order of tasks in your tasks file(s). The most important task is at the top, the least important at the bottom.

**No Swim Lanes**

Some task management tools define "swim-lanes", where each swim-lane constitutes an independent priority list. mdagile does not have a swim-lanes feature, but you can:
- strictly assign tasks to teams with assignment markers (`@...`).
- loosely assign tasks to teams with property markers (`#...`).

There are no swim-lanes, because this does not work well with milestones—you still have to ultimately decide for each individual task whether it is part of a milestone or not.

If you have multiple truly independent teams, each doing their own prioritization, you can use multiple subdirectories (and multiple `mdagile.toml` and `tasks.md` files). Note that these teams will not work towards the same milestones (see "Milestones").

**No "High Priority" Markers**

There are also no priority categories for tasks ( ~~!prio:high~~ ). There is only a global absolute priority ordering. Ultimately, if I see two tasks in front of me, even if both are "high prio", I still have to pick one of them to do first. There is no way around an absolute priority order. Priority "categories" are misleading.


## CLI Tool: `agile`

### Default: Open Next Task
```bash
agile
```
Opens the highest-priority incomplete task in your `$VISUAL` or `$EDITOR`. Jumps to the correct line for vim, nvim, nano, emacs, and VS Code.

### List Tasks
```bash
agile list              # All active tasks (default)
agile list --all       # All tasks including done and cancelled
agile list -n 5        # First 5 active tasks
agile list --last 3    # Last 3 active tasks
```

### List Files
```bash
agile list files       # Show all task files in priority order
```

### Get Next Task
```bash
agile task next        # Print the next incomplete task (same as `agile` with no editor)
```

### Validate Files
```bash
agile check
```
Parses all `*.agile.md` files and reports validation issues. Exits with status 1 if any issues found, 0 if clean. See [doc/checks.md](doc/checks.md) for the full list of checks, and [doc/config.md](doc/config.md) for the `mdagile.toml` reference.

## Language Server: `agilels`

A minimal LSP server that offers real-time diagnostics as you edit, and offers quickfix code actions for fixable issues. Runs the same checks as `agile check` — see [doc/checks.md](doc/checks.md).

## GUI

After installation launch in the directory with the md.agile.tasks and toml file: 
```
mdagile-gui
```
Then connect to localhost 8080 in a browser (in browser search bar type `http://127.0.0.1:8080/`).

--- 

## Syntax & Features

A task that is "done" is marked with a lowercase `x`; `-` marks a task as cancelled. Subtasks must be indented two spaces per level, directly under their parent (no blank line in between) — a blank line always starts a new top-level task:

```md
- [ ] a task
  - [ ] a subtask
  - [x] this subtask is done
  - [-] this subtask is cancelled

- [ ] another top-level task
```

Other content (headings, prose, etc.) is ignored, so you can freely mix notes into the same file. See [doc/checks.md](doc/checks.md) for the full list of formatting/marker rules `agile check` enforces.

### Multiple Files

Task files must be named `<something>.agile.md` and can live anywhere under the project root; every match is picked up automatically (`agile list files` shows them in priority order). Files are ordered alphabetically by path relative to the project root (directories first, then filename) — this determines cross-file task priority, so e.g. tasks in `tasks/50_current/001.agile.md` outrank tasks in `tasks/60_backlog/001.agile.md`.
Other markdown files (e.g. your `README.md`s) are ignored, even if they contain syntactically valid tasks.

### Optional and Mandatory Subtasks

By default, all subtasks are mandatory. A parent task may only be marked complete when all subtasks are done.
```md
- [ ] a task
  - [ ] some mandatory subtask
```
**Optional subtasks** are not required for completion of the parent task. They are prefixed with the special marker `#OPT` (all caps) after the checkbox:
```md
- [ ] a task
  - [ ] #OPT some optional subtask
```

### Properties (Basics)

You specify available properties globally for your project in the `mdagile.toml` (or `.mdagile.toml`) file. Property names cannot contain spaces, but you can use `-` and `_`. Properties can then be added to tasks with `#<property_name>`.

E.g. to declare a property `#feature`:

**mdagile.toml:**
```toml
[Properties.feature]
```

To use the property, place it anywhere in the task:

**tasks.md:**
```md
- [ ] #feature: add item to basket
```
You are not allowed to use a property that is not defined in the `mdagile.toml`. This is to keep things orderly—no proliferation of random meaningless hashtags, and no duplication (`#Feature #feature #feat`). The tool will issue an error if you use undefined properties. Otherwise, an "empty" property doesn't do much. It just marks a task as part of some group—but you can do much more with them ...!

Properties are the essential building blocks of your team's task management strategy - you can keep things simple or get really sophisticated - it is up to you!

### Subtasks

You can define mandatory subtasks via properties.
```toml
[Properties.feature]
subtasks = ["PO review", "dev implementation", "dev documentation", "test"]
```

Properties are added to tasks with a `#` followed by the property name. This makes the existence of the respective subtasks mandatory:
```md
- [ ] #feature: add item to basket
  - [ ] "PO review"
  - [ ] "dev implementation"
  - [ ] "dev documentation"
  - [ ] "test"
  - [ ] another custom task that is not part of the '#feature' property
```
As you type out a property marker, the language server will give you a hint - use the autofix feature of your text editor to quickly add the required subtasks.

Subtasks that are required by a property are quoted `""`.

Properties can also be added to subtasks! Note how `'#feature'` is quoted in the example above - the ticks prevent the tag from being interpreted as a property of the subtask. Otherwise you get the following:
```md
- [ ] #feature: add item to basket
  - [ ] "PO review"
  - [ ] "dev implementation"
  - [ ] "dev documentation"
  - [ ] "test"
  - [ ] #feature view number of items in basket
    - [ ] "PO review"
    - [ ] "dev implementation"
    - [ ] "dev documentation"
    - [ ] "test"
```

### Cancelled Tasks

You can mark a task as cancelled with a `-`

```md
- [-] this task is cancelled
```
Sometimes your team may plan something and later decide, that it's not necessary after all. Marking tasks as cancelled makes things transparent ("This was part of the original plan, but discarded").

By default you can not cancel subtasks that are required by properties, but this can be adjusted in the configuration.

```toml
[Properties.feature]
subtasks = ["PO review", "dev implementation", "dev documentation", "test"]
subtasks_allow_cancel = [true, true, true, false]
# testing can not be cancelled
```
Why make subtasks "mandatory" by default, but also allow to cancel them? Properties allow you to define general default workflows. But sometimes a step just doesn't make sense in a specific case. You would then be tempted to just mark the task "done", even though that is a lie. You may add a note, but such notes are non standard and easily misinterpreted. Cancelling a subtask provides an idiomatic way to communicate that a step was skipped. It is honest, transparent and conventional.


### Multiple Properties

Tasks can have multiple properties. The placement of the property in the task is not relevant. The order of the subtasks is not relevant.

```toml
[Properties.UI]
subtasks = ["UI / UX concept", "UI review"]
```

```md
- [ ] We want to develop a #feature (add item to basket) that will have a very nice #UI!
  - [ ] "UI / UX concept"
  - [ ] "PO review"
  - [ ] "dev implementation"
  - [ ] "dev documentation"
  - [ ] "test"
  - [ ] "UI review"
```

### Nested Properties
Properties can be nested. This allows you to define subtasks that are shared across multiple properties in a single place. In the example below, the `#review` property is shared between `#feature` and `#UI`:

```toml
[Properties.review]
subtasks = ["independent review by a second person"]
[Properties.feature]
subtasks = ["dev implementation", "dev documentation", "test", "developer #review"]
[Properties.UI]
subtasks = ["UI / UX concept", "UI / UX #review"]
```

A valid task then looks like this:

```md
- [ ] We want to develop a #feature (add item to basket) that will have a very nice #UI!
  - [ ] "dev implementation"
  - [ ] custom task (not from property)
    don't forget that you can still add those anywhere in the structure.
  - [ ] "dev documentation"
  - [ ] "test"
  - [ ] "developer #review"
    - [ ] "independent review by a second person"
  - [ ] "UI / UX concept"
  - [ ] "UI / UX #review"
    - [ ] "independent review by a second person"
```

### Users And Roles

You can assign tasks to specific people or groups with the assignment marker: `@someone`

```
- [ ] implement feature X @markus
  - [ ] Review the feature @QA
```
The implementation can only be marked complete by Markus. The review may be checked by any QA person. mdagile identifies the current user via info available in `git config`, and checks this ID against the assigned users and groups. You can alternatively identify yourself explicitly via command line argument `--as <user>`. Use this in pipelines, where the user's `.gitconfig` is not available.

Assignments on parent tasks do not affect child tasks (but child tasks can be assigned as well).

This feature requires that groups and users are first identified in the configuration.

Consider this feature only "automation", not "access control". This is not secure in any way -- The mechanism can easily be sidestepped! Your git history however will reveal any inconsistencies c.f. [MANIFESTO.md](MANIFESTO.md) "Trust through Transparency".

```toml
[Users.markus]
git_names = ["Markus Myman"]
git_emails = ["markusmyman@company.org"]

[Users.svenja]
git_names = ["Svenja Super"]
git_emails = ["svenjasuper@company.org"]

[Groups.DEV]
members = ["markus"]

[Groups.SENIORDEV]
members = ["markus"]

[Groups.QA]
members = ["svenja"]

[Groups.TEAM]
members = [
  "markus",
  "svenja",
]
```
You can tag multiple people or groups on the same task. In this case any person or any member from any group can mark the task as complete.
```
- [ ] implement feature X @markus or @josh
```
If you want an AND connection instead, create subtasks for each person!


## Project Philosophy

See [MANIFESTO.md](MANIFESTO.md). Term definitions (Marker, Property, Assignment, Special Marker, etc.) are in [GLOSSARY.md](GLOSSARY.md).
