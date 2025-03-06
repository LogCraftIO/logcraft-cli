# Policies

LogCraft supports the creation of custom policies for Detection Governance.

## At a glance

**Security teams often struggle with maintaining consistency in detection rules**, as each team member has their own approach.

For example, Bob always includes the `disabled` field when creating a new detection, while Alice omits it, assuming it can be left out since it has a default value. Meanwhile, the new recruit, Charlie, is unaware that each detection rule must follow a strict naming pattern to align with internal conventions.

To address these inconsistencies, **policies formalize and enforce these rules, ensuring uniformity regardless of who creates detection rules**.

## Policies

A policy is a small file that defines specific constraints for detection rules, ensuring consistency and standardization. There is no limit to the number of policies that can be created.

Policies are applied per plugin and stored in the corresponding folder within `.logcraft`.

For example:

- Splunk policies are in `.logcraft/splunk` and apply to Splunk detections.
- Tanium policies are in `.logcraft/tanium` and apply to Tanium detections.

This structure ensures that each detection platform has its own dedicated set of policies, keeping rule enforcement organized and scalable.

Policies are verified against detection rules using the [validate](../commands/validate.md) command.

::: tip
Policy filenames can be named freely, but it’s recommended that your team follow a shared naming convention for consistency. Common formats include camelCase (`titleFormat.yaml`) or kebab-case (`title-format.yaml`).
:::

## File format

The following raw YAML details what a policy is - simple yet highly effective. :muscle:

```yaml
# global, required
field: <json-path>
check: existence | absence | pattern | constraint
severity: warning | error

# global, optional
message: <str> # with ${fieldName} for field replacement.
ignorecase: true | false = false # applies to pattern and constraint

# specific fields (depends on 'check' type)
## check: pattern
regex: <pcre>
## check: constraint
validations:
  minLength: int
  maxLength: int
  values:
    - <list of values to match against>
```

::: details Common error on heck constraints
A common error is to define validation constraints as a list instead as unique keys.

**Correct**:
```yaml
...
validations:
  minLength: int
  maxLength: int
```

**Incorrect**:
```yaml
...
validations:
  - minLength: int
  - maxLength: int
```
:::

### Option field

The `field` attribute specifies the target field for the policy and is defined using a JSON path. For example, to reference the `disabled` key located under `parameters` in a detection rule, use `/parameters/disabled`.

### Option check

This table summarizes the available values for the `check` parameter and the kind of test performed.

| Check      | Purpose                                       |
| ---------- | --------------------------------------------- |
| existence  | Ensure the field is present                   |
| absence    | Ensure the field is not present               |
| constraint | Ensure the field respects a constraint        |
| pattern    | Ensure the field matches a regular expression |

### Option severity

The `severity` field indicates the importance of the policy and defines the consequences if the policy is not respected.

| Severity | Behavior                                                           |
| -------- | ------------------------------------------------------------------ |
| warning  | Prints a warning message and continue to the next policy/detection |
| error    | Prints an error message and stop the execution flow                |

<!-- vale Vale.Spelling = NO -->

### Option ignorecase

<!-- vale Vale.Spelling = YES -->

This field is applicable to 'pattern' and 'constraint' checks. For example, the following constraint matches 'foo', 'FOO', or any other mixed case such as 'FoO'.

```yaml
ignorecase: true
validations:
  values:
    - foo
```

### Option message

The `message` field is optional and allows overriding the default message. The table below outlines the default messages based on check type and severity.

| check      | severity | default message                                  |
| ---------- | -------- | ------------------------------------------------ |
| existence  | warning  | field '${fieldName}' should be present           |
| existence  | error    | field '${fieldName}' must be present             |
| absence    | warning  | field '${fieldName}' shouldn't be present        |
| absence    | error    | field '${fieldName}' must not be present         |
| constraint | warning  | field '${fieldName}' doesn't respect constraints |
| constraint | error    | field '${fieldName}' doesn't respect constraints |
| pattern    | warning  | field '${fieldName}' doesn't match pattern       |
| pattern    | error    | field '${fieldName}' doesn't match pattern       |

Overall, the output format is as follow

```bash
% lgc validate
[<SEVERITY>] <message> (policy: <policy-file.yml>, detection: <detection-file.yaml>)
%
```

The following example overrides the default message to provide a more specific error.

```yaml
field: /parameters/counttype
severity: error
check: absence
message: "Splunk REST-API forbid setting the '${fieldName}' parameter"
```

Note that `${fieldName}` is a placeholder that is replaced with the specified field name in the policy, such as `/parameters/counttype` in the preceding example.

## Examples

### Field should be present

The example below ensures that the `disabled` parameter is specified in the detection.

```yaml
field: /parameters/disabled
severity: warning
check: existence
```

```bash
% lgc validate
WARN field '/parameters/disabled' should be present (policy: disabled.yaml, detection: detection-file.yaml)
%
```

### Field must respect a regular expression

The example below ensures that detections start with 'MAL-' or 'INT-', followed by a digit. This could represent an internal naming convention, for example.

```yaml
field: /title
severity: error
check: pattern
regex: '^[MAL|INT]-\d+\s'
```

```bash
% lgc validate
ERROR field '/title' doesn't match pattern (policy: titleFormat.yaml, detection:
detection-file.yaml)
%
```

### Field must respect a length constraint

The example below ensures that detection titles are at least 10 characters long and no more than 256 characters.

```yaml
field: /title
severity: error
check: constraint
validations:
  maxLength: 256
  minLength: 10
```

### Field must be 'one of'

The example below ensures that the parameter 'foo' is one of the specified values.

```yaml
field: /parameters/foo
severity: error
check: constraint
validations:
  values:
    - bar
    - baz
    - foobar
```

This check is case sensitive, see `ignorecase` to change this behavior.
