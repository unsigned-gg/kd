# .claude/ - Claude Code settings and hooks

The rules for this repo live in [AGENTS.md](../AGENTS.md). This directory holds only the Claude Code wiring.

`settings.json` is shared and committed. `settings.local.json` is per user and gitignored.

## Hooks


## Tests

One test per hook is in `hooks/tests/`. Each test is plain bash. It sends JSON on stdin, as Claude Code does.

```bash
for t in .claude/hooks/tests/*.sh; do bash "$t" || exit 1; done
```

## Permissions

Allowed without a prompt: `cargo`, `make`, `just`, `git fetch`, `git rebase`. Denied: reads of `docs/archive/**`.
