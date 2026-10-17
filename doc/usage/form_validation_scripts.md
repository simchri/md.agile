# Form validation scripts

A form (`[Form.X]`) may declare a `validation` script in
[`mdagile.toml`](config.md): a project-relative path to an executable file
(with a shebang) that validates the form's fields as a whole. See "Forms &
Fields" in [README.vision.md](../../README.vision.md).

## Trust

A script runs only after it was approved with `agile trust` (see README).
Approval is per user and keyed by the script's project-relative path and
content hash, so editing a script revokes it.

## Checks

`agile check` runs the script once per task carrying the form and reports
E030 (form invalid) or E031 (script could not run, e.g. untrusted). In CI run
`agile trust --yes` first. The language server runs scripts only on save; results disappear on the next
edit. See [checks.md](checks.md).

## Execution

- One process per task carrying the form (no batching).
- Executed directly: no shell, no arguments.
- Environment cleared except a minimal `PATH`; working directory is the project root.
- Subject to a timeout (default 10 s) and a stdout size cap (default 1 MiB);
  a hung or runaway script is killed and reported as a failure to run.
- This is hardening, not an OS-level sandbox: the script can still access
  files and the network as the current user.

## Input (stdin)

Only the form's own fields are sent, never other properties or fields of the task:

```json
{
  "form": "personal_info",
  "task": { "title": "new colleague: John #onboarding", "line": 12 },
  "fields": { "First Name": "John", "Email": "john@example.com" }
}
```

`fields` maps each field label to its value; `line` is the 1-based source line of the task.

## Output (stdout)

A script that ran properly **always exits 0**, whether or not the form is
valid, and prints:

```json
{ "valid": false, "errors": [ { "field": "Email", "message": "not an email address" } ] }
```

- `valid` — required.
- `errors` — optional list; `field` (a field label) is optional and omitted for
  errors about the form as a whole.

A non-zero exit, a timeout, oversized output, or output that isn't this JSON
means the script *failed to run properly*, which is distinct from "validation failed".
