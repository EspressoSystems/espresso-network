# Operator notes

One file per change that requires operator action when upgrading, for example a renamed or newly required setting, a
manual migration step, a restart order or a deadline. Changes that work with existing setups need none. The release
process collects the files into the release body.

## Format

File: `doc/operator-notes/<short-topic>.md`, added in the PR that needs it. Backports carry it.

```markdown
### <title>

Urgency: High | Medium | Low

What operators must do, and by when (view number and UTC time) if there is a deadline.
```

## Rules

- No `<!--` in a fragment, and no subdirectories or non-`.md` files here: the release commands fail naming the file.
- A release lists the fragments added or edited since the previous release, so an edited fragment is shown again.
