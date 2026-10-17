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
  "fields": { "first_name": "John", "email": "john@example.com" }
}
```

`fields` maps each field's `[Field.X]` key from `mdagile.toml` (not its display label, which may change) to its value; `line` is the 1-based source line of the task.

## Output (stdout)

A script that ran properly **always exits 0**, whether or not the form is
valid, and prints:

```json
{ "valid": false, "errors": [ { "field": "email", "message": "not an email address" } ] }
```

- `valid` — required.
- `errors` — optional list; `field` (a field key, as in `fields`) is optional and omitted for
  errors about the form as a whole.

A non-zero exit, a timeout, oversized output, or output that isn't this JSON
means the script *failed to run properly*, which is distinct from "validation failed".

## Example: basic email check

A readable, regex-free syntax check. It does not prove an address exists, it
only catches obvious typos.

`mdagile.toml`:

```toml
[Field.email]
label = "Email"

[Form.personal_info]
fields = ["email"]
validation = "scripts/validate_email.py"

[Properties.onboarding]
form = "personal_info"
```

`scripts/validate_email.py` (make it executable: `chmod +x`, then `agile trust`):

```python
#!/usr/bin/env python3
# Validates the "email" field of a form. See form_validation_scripts.md.
import json
import sys


def email_problem(email):
    """Returns a message describing what is wrong, or None if it looks fine."""
    if email == "":
        return "email address is missing"
    if " " in email:
        return "email address must not contain spaces"

    # Exactly one "@" separates the local part from the domain.
    if email.count("@") != 1:
        return "email address must contain exactly one '@'"
    local, domain = email.split("@")

    if local == "":
        return "nothing before the '@'"
    if domain == "":
        return "nothing after the '@'"

    # The domain needs at least one dot ("example.com"), and no empty
    # labels ("example..com", ".example.com", "example.com.").
    labels = domain.split(".")
    if len(labels) < 2:
        return "domain must contain a '.' (e.g. example.com)"
    if "" in labels:
        return "domain must not have empty parts around a '.'"
    return None


# The script receives the form's data as JSON on stdin.
data = json.load(sys.stdin)
email = data["fields"].get("email", "").strip()

problem = email_problem(email)
if problem is None:
    result = {"valid": True}
else:
    # "field" ties the error to the Email line in the task file.
    result = {"valid": False, "errors": [{"field": "email", "message": problem}]}

# A script that ran properly always prints the verdict and exits 0,
# even when the form is invalid.
print(json.dumps(result))
