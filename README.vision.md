# Md.Agile

> This file states the vision for the project, from a user's POV – the features described here are not yet implemented. For what's currently available, c.f. [README.md](README.md).

...
```
agile task new
```
Drop into an interactive mask (TUI) to create a new task. You can decide whether the task goes to the bottom or top of your backlog.

```
agile when
```
Get an estimated time until the next milestone

...

## File Structure for Large Projects & Archiving

When you initialize task management in a directory with `agile init --large`, the tool automatically creates the following file structure:

```
tasks/
  00_archive/
    ...
    2026-04-06_003.agile.md
    2026-04-13_001.agile.md

  50_current/
    001.agile.md
    002.agile.md
    ...

  60_backlog/
    001.agile.md
    002.agile.md
    ...

  80_inbox/
    inbox.agile.md
```

Files in `50_current` and `60_backlog` are numbered sequentially. The numbers establish priority order between files — lower numbers come first. You create new files by incrementing the counter; you never rename existing ones.

You can place new tasks in any of the files, but you are discouraged from touching the archive.

If this file structure is present, the command `agile archive` will move any file in `50_current` or `60_backlog` that contains only completed or cancelled tasks to `00_archive`, prefixing it with today's date (e.g. `001.agile.md` → `2026-04-27_001.agile.md`). The archive is a plain record of when the file was closed out; the timestamp carries no scheduling meaning.

Tasks in `inbox` are never moved automatically.

The following configurations are available

```toml
[Archive]
archive_path = "tasks/00_archive/"
current_path = "tasks/50_current/"
backlog_path = "tasks/60_backlog/"
inbox_path = "tasks/80_inbox/"
```
...

### Required Properties

You can define properties as mandatory for each task in a file. This is useful if you want to apply some properties by default for a certain part of the project.

**tasks.md:**
```md
#MDAGILE.file.mandatory_property=feature
```
If this is set, tasks must have the `#feature` property, otherwise errors are issued. New tasks created via the cli tool are given the property automatically. Use the `agile fix` subcommand or autofix in your text editor to add missing properties to existing tasks.


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
(N.B. The short form markers do not actually have to be shorter than the respective full form, nor do they have to be linguistically similar - they can be any unique identifier, adhering to the naming rules for markers.)

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

You can then get ...

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

...

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
  - [ ] "do the back end implementation also!"
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

A Branching Workflow property is written in its incomplete form (e.g., `#review...`) while the task is still in progress. When the task is marked as done, this property must be updated to one of its defined outcome states (e.g., `#review:passed`, `#review:failed`). Each outcome can have its own constraints, such as mandatory subtasks or neighbor tasks.

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
