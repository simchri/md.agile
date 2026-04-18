# md.Agile

> This file states the project's vision from a user POV – nothing here is currently implemented

Simple, collaborative task management using Markdown (plain text) and Git.

Your tasks live forever in a simple text file, version-controlled directly alongside your code (ideally in the same repository) – not a web app!

**tasks.md:**

```md
- [ ] a task - this is the task-title
Some more info on this task - this is the task-body
Both inside and outside of tasks, you can just use normal markdown syntax
  - [ ] a subtask @markus
  more details for this subtask go here.
  - [x] this subtask is done

- [ ] #bug: another task
  - [ ] "1. reproduce in test"
  - [ ] "2. implement fix"
  - [ ] "3. regression test"
```

Tasks follow a specific syntax. You will receive immediate feedback in your text editor if you make a mistake (via Language Server). Add mandatory subtasks using "hash-tag" markers – fully configurable and recursive. Use the language server's auto-fix feature for an ergonomic experience. Use the CLI tool to add strict checks as pre-commit hooks or in your pipeline – your task list is always consistent. Everything is designed with a "command line first" approach: text files and CLI tools are the primary interface. Optional graphical clients aim to make adoption easy for non-technical colleagues.

```
agile
```
Show me my next tasks in an interactive viewer

```
agile check
```
Check if all rules are satisfied. Returns non-zero and prints errors otherwise.

```
agile task new
```
Drop into an interactive mask (TUI) to create a new task. You can decide whether the task goes to the bottom or top of your backlog.

```
agile when
```
Get an estimated time until the next milestone


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

## Multiple files

As your project grows, you may want to split your task list over multiple files. If you want to use more than one file, files must follow the naming convention `<some name>.agile.md`
- by default, any file in any subdirectory to the root is picked up by the tool
- other markdown files (e.g. your `README.md`s) are ignored, even if they contain syntactically valid tasks.
- all found files are then brought into a global order, alphabetically (TODO: spec details), using only the file name, not its location (path).
- The order of tasks in this aggregated file determines the priority order.

Recommendation: Keep all files near the top level in a common folder. Anything else just makes understanding priorities confusing.

## More Features

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

Subtasks that are required by a property are quoted `""`. If a property tag is followed by a single punctuation symbol (`:;,.` etc.), that symbol is ignored (it is not considered part of the property name, nor the task name).

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

### Multiple Properties

Tasks can have multiple properties. The placement of the property in the task is not relevant. The order of the subtasks is not relevant.

```toml
[Properties.UI]
subtasks = ["UI / UX concept", "UI review"]
```

```
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
  - [ ] "dev documentation"
  - [ ] "test"
  - [ ] "developer #review"
    - [ ] "independent review by a second person"
  - [ ] "UI / UX concept"
  - [ ] "UI / UX #review"
    - [ ] "independent review by a second person"
```

### Required Properties

You can define properties as mandatory for each task in a file. This is useful if you want to apply some properties by default for a certain part of the project.

**tasks.md:**
```md
#MDAGILE.file.mandatory_property=feature
```
If this is set, tasks must have the `#feature` property, otherwise errors are issued. New tasks created via the cli tool are given the property automatically. Use the `agile fix` subcommand or autofix in your text editor to add missing properties to existing tasks.

### Ordered Tasks

You can define an order in which tasks have to be done:
```md
- [ ] make app more responsive
  - [ ] 1. add performance UI test
  - [ ] 2. refactor signals
  - [ ] 4 document learnings
  - [ ] 3 run UI tests
  - [ ] discuss further steps
```
Ordering numbers have to follow the checkbox after a single space character, can optionally be followed by a `.` and must be separated from the next word by at least one space. The subtasks do not have to be arranged in order (note how 3 and 4 are mixed up).

When an order is defined, the following rules apply:

- Unordered tasks ("discuss further steps") can be marked complete at any point.
- Ordered tasks can be marked complete only when all previous tasks are complete.

Meaning the following is not allowed:
```md
- [ ] make app more responsive
  - [ ] 1. add performance UI test
  - [x] 2. refactor signals
  - [ ] 4 document learnings
  - [ ] 3 run UI tests
  - [ ] discuss further steps
```

### Ordered Tasks via Properties

Subtasks required by #properties can also be ordered:

```toml
[Properties.feature]
subtasks = ["1. dev implementation", "2. dev documentation", "3. test", "4. developer #review"]
```

### Property Short Forms - Brainstorming

You can earmark future tasks with properties, but skip writing out all subtasks for now. For this, define a short form marker for a property:
```toml
[Properties.feature]
subtasks = ["1. dev implementation", "2. dev documentation", "3. test", "4. developer #review"]
short = "feat"
```
If a property is applied in short form, the subtasks are not mandatory, but the task can not be marked complete:

```md
- [ ] #feat: add item to basket
OK!
```
```md
- [x] #feat: add item to basket
Not Ok!
```

If you want to be able to easily distinguish short form properties from full properties, use a naming convention, e.g. a postfix:
```toml
short = "feat_"
```

### Milestones and ETA to Milestone

Mdagile supports agile planning and time estimation via milestones:

A milestone is simply a marker between tasks, identified by the special tag `#MILESTONE` . When all tasks above (before) the milestone are complete, the milestone is reached.

```md
- [x] implement all MVP features
- [x] perform first release

#MILESTONE: Release of MVP :)

- [ ] gather first user feedback

```
Punctuation directly behind the tag is ignored (`#MILESTONE` is equivalent to `#MILESTONE:`, `#MILESTONE!` etc.). A milestone name must be provided, and milestones must be unique across the project.

You can then get

- count of remaining tasks (and subtasks) to milestones
- estimate average time per task for past tasks
- ETAs (Estimated Time to Arrival) for each milestone

with the `agile when` command:
```bash
$ agile when
Milestone: Release of MVP :)
ETA: 2024-07-01
Done: 8
Remaining: 5

Milestone: Release of v2.0
ETA: 2024-08-15
Done: 4
Remaining: 12
```
(c.f. also `agile when --help`)

### ETA - Task Weights

For the purpose of ETA estimation only, the tool assigns different weights to tasks and subtasks. The total weight of a task is the sum of the weights of its subtasks, plus 1 (the task itself). The weight of a subtask is 1/"subtask level". E.g.:

```md
- [ ] A simple task: Weight = **1**

- [ ] A task with two subtasks: Total weight = 1 + .5 + .5 = **2**
  - [ ] subtask 1: Weight = 1/2
  - [ ] subtask 2: Weight = 1/2

- [ ] Total weight = 1 + .5 + .33 = **1.83**
  - [ ] Weight = 1/2
    - [ ] Weight = 1/3
```

Whenever the tool needs to "count" tasks, for the purpose of time estimation, task weights are used instead of the raw count.

Subtasks that are required by a property are counted in exactly the same way as custom tasks. Subtasks required by a property used in **short form** are also counted, even if not (yet) explicitly written out! (c.f. "Property Short Forms")


### Users And Roles

You can assign tasks to specific people or groups with the assignment marker: `@someone`

```
- [ ] implement feature X @markus
  - [ ] Review the feature @QA
```
The implementation can only be marked complete by Markus. The review may be checked by any QA person. mdagile checks this by comparing the current user's email, as provided in the git config, against the assigned users and groups. You can alternatively inject an identity explicitly via arguments or environment variables. Use this in pipelines, where the user's `.gitconfig` is not available.

This feature requires that groups and users are first identified in the configuration.

Consider this feature only "automation", not "access control". This is not secure in any way! The mechanism can easily be sidestepped! We assume that our colleagues use this responsibly and do not impersonate others. (However, you can always check in your git history if someone cheated). (c.f. MANIFESTO.md "Trust but Control")

```toml
[Users]
markus = {full_name="Markus Myman", email = "markusmyman@company.org"}
svenja = {full_name="Svenja Super", email = "Svenjasuper@company.org"}

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

## Neighbor Tasks

Neighbor Tasks are tasks that must be present on the same level as the task with the property. Neighbor Tasks are mainly useful in combination with Branch Properties.

You define Neighbor Tasks with the `neighbortasks` key:

**mdagile.toml:**
```toml
[Properties.frontend-implementation]
neighbortasks = ["do the back end implementation also!"]
```

**tasks.agile.md:**
```md
- [ ] let's build a feature and
  - [ ] ..first do the #frontend-implementation
  - [ ] do the back end implementation also!
```

Neighbor tasks can themselves have their own properties and subtasks. This mechanism helps ensure that important follow-up steps are not forgotten and are tracked explicitly in your workflow. You can also use this to couple certain properties together.

**Properties with Neighbor Tasks can not be set at the top level (only on subtask level 1 and lower).** Neighbor tasks at the top level don't make sense, because the feature would only be usable exactly once for the entire project.

## Branch Properties

Branch Properties allow you to implement branching workflows depending on the outcomes of tasks. The following config snippet defines a property `#review...` with two branches `#review:passed` and `#review:failed`:

```toml
[Properties.review]
subtasks = ["document review findings"]
[Properties.review.passed]
neighbortasks = ["publish feature"]
[Properties.review.failed]
neighbortasks = ["create follow up task for fixes"]
```

A Branching Workflow property is written in its incomplete form (e.g., `#review...`) while the task is still in progress. When the task is marked as done, this property must be updated to one of its defined outcome states (e.g., `#result:passed`, `#result:failed`). Each outcome can have its own constraints, such as mandatory subtasks or neighbor tasks.

While task in progress:
```md
- [ ] build something
  - [ ] perform #review...
    - [ ] "document review findings"
```
Review passed:
```md
- [ ] build something
  - [x] perform #review:passed
    - [x] "document review findings"
  - [ ] "publish feature"
```
Review failed:
```md
- [ ] build something
  - [x] perform #review:failed
    - [x] "document review findings"
  - [ ] "create follow up task for fixes"
```

It is not allowed to mark the task as complete without updating the property to one of its defined outcomes. The following will be marked with an error:
```md
- [x] perform #review...
  - [x] "document review findings"

```
