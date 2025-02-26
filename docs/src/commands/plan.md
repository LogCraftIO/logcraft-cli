# lgc plan

<!-- vale Google.Will = NO -->

The plan command lets previewing the changes that lgc will make to your security services. It evaluates all security detections from the workspace and then compare that desired state to the real detections on remote services.

<!-- vale Google.Will = YES -->

The plan command uses state data and checks the current state of each detection using the relevant API to determine detections that should be created, updated, or removed from remotes services.

This command does not perform any actual changes to remote services and it is usually run to ensure pending changes are as expected, before [applying the changes](apply).

```sh
% lgc plan [<IDENTIFIER>] [options]
```

Example:

```sh
% lgc plan
INFO [+] 'High Domain Entropy (DGA)' will be created on 'splunk-prod'
INFO [~] 'Crazy High Domain Entropy' will be updated on 'splunk-prod'
INFO [-] 'Test Rule 123' will be removed from 'splunk-prod'
...
%
```

The plan command presents changes with the following formalism:

1. **Creation**: a new detection has been created in the workspace and needs to be deployed (symbol: `+`).
2. **Edition**: a detection has been modified in the workspace and changes need to be propagated to the remote service (symbol: `~`).
3. **Deletion**: a detection has been removed from the workspace so it needs to be removed from the remote service (symbol: `-`).

## Options

<!-- vale Google.Headings = NO -->

### --state-only/-s

<!-- vale Google.Headings = YES -->

By default, the plan command uses [state data](/developers/state) and connects to remote services to determine the changes.

This flag changes this behavior by only using the local state to determine the changes.

```sh
% lgc plan --state-only
INFO using local state only, plan might be incorrect or incomplete
INFO [+] 'High Domain Entropy (DGA)' will be created on 'splunk-prod'
%
```

This flag makes the planning operation faster by reducing the number of remote API requests. However, this causes lgc to ignore external changes that occurred outside of normal workflows. This could potentially result in an incomplete or incorrect plan.

::: tip
This flag is only interesting in development environments where having incorrect or incomplete plans may be acceptable.
:::

<!-- vale Google.Headings = NO -->

### --verbose/-v

<!-- vale Google.Headings = YES -->

By default, the plan command only displays a summary of the changes. With the verbose flag set, the details of the changes are also displayed:

```sh
% lgc plan prod --verbose
[+] rule 'High Domain Entropy (DGA)' will be created on 'splunk-prod'
[~] rule 'Crazy High Domain Entropy' will be updated on 'splunk-prod'
| {
|   "app": "DemoApp",
|   "savedsearch": {
|      "cron_schedule": "*/15 0 0 0 0",
| -    "disabled": "true",
| +    "disabled": "false"
|    }
| }
[-] rule 'Test Rule 123' will be removed from 'splunk-prod'
%
```
