# lgc validate

This command ensures the detections are correctly formatted, typed, and consistent. In short, the validate command is a linter for security detections and this command shines in a CI/CD or locally to validate the detections before even attempting to deploy them.

```sh
% lgc validate
ERROR validation failed on 'rules/splunk/some-detection.yaml': field: 'parameters.disabled', error: invalid type: string "fals", expected a boolean
%
```

If no errors are encountered, lgc exits gracefully:

```sh
% lgc validate
INFO all good, no problems identified
%
```

The validation is specific to each technology (see [plugins](/concepts/plugins)).

::: info Example
Both [Splunk](../plugins/splunk#linting) and [Microsoft Sentinel](../plugins/microsoft-sentinel) plugins implement the validate command, but they perform different validations:

- Splunk have a field `disabled` that has to be set to a boolean value (`true` or `false`). The validation process ensure that if the field `disabled` is specified, it has an appropriate value.
- For Microsoft Sentinel, the same validation exists, except it is performed on the field `enabled` because the field `disabled` simply doesn't exists.
  :::

## Options

<!-- vale Google.Headings = NO -->

### --quiet/-q

<!-- vale Google.Headings = YES -->

The `--quiet` option instructs `validate` to stay quiet, except if errors are encountered.

```sh
% lgc validate --quiet
%
```

When errors occur

```sh
% lgc validate -q
ERROR validation failed on 'some-detection.yml'
%
```
