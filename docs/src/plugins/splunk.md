---
title: Splunk
category: SIEM
stage: stable
---

# Splunk

<PluginStatus :category="$frontmatter.category" :stage="$frontmatter.stage" />

## File format

Splunk detections are normalized as follow:

```yaml
title: <title of the detection (stanza name)>

search: |-
  <search query>

parameters: <all the parameters you want to set>
```

This is a pretty simple and straightforward format (example below).

## File names

Each detection must be stored in its own YAML file under the plugin directory inside the workspace root.

Example:

- `rules/splunk/detect-foo.yaml`
- `rules/splunk/high-entropy-domain-name.yaml`

This ensure each detection is tracked individually.

## Example

```yaml
title: High Entropy Domain Names - Sample Rule
search: |-
  sourcetype=proxy
  | eval list = "mozilla"
  | `ut_parse(url, list)`
  | stats count by url_domain
  | `ut_shannon(url_domain)`
  | where shannon_entropy > 4.0
  | sort -count
  | rename count as total, url_domain as domain, shannon_entropy as entropy
parameters:
  cron_schedule: 0 * * * *
  disabled: false
  is_visible: true
  description: |-
    This rule explodes URLs found in proxy logs to isolate the domain part in order
    to compute the shannon entropy of the domain names. If the entropy is greater
    than 4.0, the domain is likely a DGA (or CDN!).
```

## Plugin configuration

To configure a [service](../commands/services), the plugin has the following parameters.

| Parameter   | Required | Default | Purpose                                               |
| ----------- |:--------:| ------- | ----------------------------------------------------- |
| `url`       | Yes      | n/a     | Splunk server URL (https\://your-server:8089)       |
| `auth_type` | Yes      | n/a     | Authorization mechanism: `Bearer` or `Basic`          |
| `token`     | Yes      | n/a     | JWT Token (Bearer) or a Base64 encoded string (Basic) |
| `timeout`   | No       | `60`    | Optional timeout, in seconds                          |
| `app`       | No       | `search`| Target Splunk App to work with                        |
| `user`      | No       | n/a     | Optional Splunk user to work with (see note below)    |


::: details `user` option
The `user` option is not related to authentication and should be left to its default value `nobody` in most situations. If `user` is set, for example `user = "admin"`, then the savedsearches are stored under `/opt/splunk/etc/users/<user>` instead of the commonly expected `/opt/splunk/etc/apps/<app>`.
:::


Example:

```toml
[services.splunk-server]
environment = "prod"
plugin = "splunk"

[services.splunk-server.settings]
url = "https://192.168.64.22:8089"
auth_type = "Bearer"
token = "eyJraWQiOiJzcGx1bmsuc2VjcmV0IiwiYW.....z4IaBtAHPFg"
```

::: info https
By default, Splunk management port is accessible over https on tcp/8089. Make sure to set `url = "https://...` otherwise the `connection reset by peer` error could be raised if you try to access it over regular http.
:::

## Authorization

#### Authentication tokens (recommended)

::: tip
Authentication tokens are the recommended mechanism to authenticate to Splunk.
:::

Log in to Splunk with administrator privileges, then go to **Settings > Tokens** and create a new token as follow:

- User: &lt;your user&gt;
- Audience: LogCraft
- Expiration: as it fits your needs

Then click **Create** and save it in the service definition.

```toml
[services.<identifier>.settings]
auth_type = "Bearer"
token = "eyJraWQiOiJzcGx1bm.....z4IaBtAHPFg"
```

#### Basic

::: danger
**Avoid using Basic authentication**, prefer using user (jwt) tokens.
::::

Convert your credentials `username:password` in based64:

```bash
~$ echo -n "bwayne:batman" | base64
YndheW5lOmJhdG1hbg==
~$
```

And save it in the service definition:

```toml
[services.<identifier>.settings]
auth_type = "Basic"
token = "YndheW5lOmJhdG1hbg=="
```

## Linting

Each plugin has some linting capabilities (see [lgc validate](../commands/validate)) and this section details the linting capabilities of the Splunk plugin.

### bool

Accepted values:

- true, false
- True, False

Erroneous detection file (snippet):

```yaml
parameters:
  disabled: hello
```

Output

```bash
% lgc validate
ERROR validation failed on `rules/splunk/my-detection.yaml`, field: `parameters.disabled', error: invalid type: string "hello", expected a boolean
%
```

### string

Accepted values:

- any string expressed with `"..."` or `|-`

Erroneous detection file (snippet):

```yaml
search: true
```

Output

```bash
% lgc validate
ERROR validation failed on `rules/splunk/my-detection.yaml`, field: `search`, error: invalid type: boolean `true`, expected a string
%
```

### enum

Accepted values:

- a defined set of values (parameter dependent)

Erroneous detection file (snippet):

```yaml
parameters:
  dispatchAs: bob
```

Output

```bash
% lgc validate
ERROR validation failed on `rules/splunk/high-entropy-domain.yaml`, field: `parameters.dispatchAs`, error: unknown variant `bob`, expected `owner` or `user`
%
```

::: info Incorrect schema validation?
Open a ticket [here](https://github.com/LogCraftIO/logcraft-cli/issues) with the parameter and its value and explain if this is a valid or an invalid key-value pair. You can also report this [by email](mailto:hello@logcraft.io)
:::
