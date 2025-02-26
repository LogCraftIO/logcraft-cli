# lgc destroy

This command is a convenient way to clean up remote services, especially ephemeral environments often encountered for development purposes. This command removes all detections from the target services.

```sh
% lgc destroy <IDENTIFIER>
```

## Options

<!-- vale Google.Headings = NO -->

### --auto-approve

<!-- vale Google.Headings = YES -->

The `--auto-approve` flag skips the prompt and immediately remove the detections from the remote services. This is especially handy in CI/CD workflows.

Normal (interactive) run:

```bash
% lgc destroy dev
... list of suppression ...
Apply changes? (y/n)
// changes are applied if the user confirm 'yes'
%
```

Non-interactive run:

```bash
% lgc destroy --auto-approve
... list of changes ...
// changes are applied automatically
%
```
