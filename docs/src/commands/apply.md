# lgc apply

The apply command performs a plan just like [lgc plan](./plan.md) does, but then actually carries out the planned changes to each remote services using the relevant plugin's API.

It asks for confirmation from the user before making any changes, unless it was explicitly told to skip approval.

```bash
lgc apply [<IDENTIFIER>] [options]
```

To apply changes to a specific service:

```bash
lgc apply my-service
```

To apply changes to all services belonging to a specific environment:

```bash
lgc apply my-environment
```

In the case of an environment, all services belonging to that environment are updated, if, and only if, changes are pending toward these services.

::: tip
The apply command is the only command that locks the [state](../developers/state.md) because it is the only command modifying the state.
:::

## Options

<!-- vale Google.Headings = NO -->

### --auto-approve/-a

<!-- vale Google.Headings = YES -->

The `--auto-approve` flag skips the prompt and immediately apply the changes without requiring user intervention. This is especially handy in CI/CD workflows.

Normal (interactive) run:

```bash
% lgc apply
... list of changes ...
Apply changes? (y/n)
// changes are applied if the user confirms 'yes'
%
```

Non-interactive run:

```bash
% lgc apply --auto-approve
... list of changes ...
// changes are applied automatically
%
```
