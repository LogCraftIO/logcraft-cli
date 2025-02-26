# Quickstart

The command-line utility `lgc` has been designed to be integrated into a CI/CD pipeline but it can also be used locally, on workstation.

## Initialize a repository

As security detections should be hosted and tracked individually in a git server, the first step is to initialize a repository.

```bash
% mkdir security-repo
% cd security-repo
security-repo % git init
security-repo % git remote add origin git@<GIT_SERVER>:/<REPOSITORY_URL>.git
```

::: details Alternative approach
Alternatively, create a new repository in your git and clone it using `git clone`.
::::

In this repository, you can have any folder or file your want. All lgc needs, it's a dedicated _workspace_. To kickstart this workspace, use the [lgc init](../commands/init.md) command to create the default workspace _rules/_ and a minimal configuration file _lgc.toml_.

```bash
security-repo % lgc init --create
```

The boilerplate is now completed, commit, and move on to configuration.

```bash
security-repo % git add .
security-repo % git commit -m 'chore: first commit'
security-repo % git push --set-upstream origin main
```

## Configure services

Once lgc is installed and initialized, it is time to define [services](../commands/services.md).

A service is a server, an instance, a remote host, of a security system and each service rely on a [plugin](../plugins/index.md) for its configuration and communication. For example, you may have a Splunk server for production and another for testing, that would be 2 services.

For this quickstart, a single server `splunk-prod` is created and attached to an arbitrary environment name `prod` (see [identifiers](../concepts/identifiers.md) for more information on identifiers)

```bash
security-repo % lgc services create
✔ Select the plugin to use: · splunk
✔ Service identifier: · splunk-prod
✔ Environment name: · prod
✔ Do you want to configure the service now? · yes
✔ Application context · "search"
✔ Authorization type · "Bearer"
✔ Timeout (seconds) · 60
✔ Authorization token · "************"
✔ Splunk URL · "https://splunk-server-url:8089"
✔ User context · "nobody"
 INFO service `splunk-prod` successfully created
security-repo %
```

::: info
Always refer to the [plugin documentation](../plugins/index.md) when adding or editing a service.
:::

Finally, once you created and configured a service, ensure it is operational with the [ping](../commands/ping.md) command:

```bash
security-repo % lgc ping splunk-prod
/ splunk-prod ... OK
security-repo %
```

## Adding a detection

It is now time to create a first detection for the configured service, here a Splunk detection for this example.

```bash
security-repo % mkdir rules/splunk
security-repo % cat << EOF > rules/splunk/our-first-detection.yaml
title: Our First Detection
search: |-
  | stats count
parameters:
  disabled: false
  description: |-
    This is a first sample detection!
EOF
security-repo %
```

Before moving any further, check if this detection is valid from a syntax and grammar point of view using the [validate](../commands/validate.md) command.

```bash
security-repo % lgc validate
 INFO all good, no problems identified.
security-repo %
```

Great. Now, edit the detection (YAML) file `rules/splunk/our-first-detection.yaml` and change the value of `disabled` from **false** to **hello**. That's obviously a gross error, but who never made a typo? :sweat_smile:

```bash
security-repo % lgc validate
ERROR validation failed on `rules/splunk/our-first-detection.yaml`, field: `parameters.disabled`, error: invalid type: string "hello", expected a boolean
security-repo %
```

Tada :tada: 

**Your detections are validated before even reaching the remote service**. Refer to the plugin documentation for detailed information on the validation process.

Fix the detection, and see what changes [may occur](../commands/plan.md) if they actually are applied:

```bash
security-repo % lgc plan
[+] `rules/splunk/our-first-detection.yaml` will be created on service `splunk-prod`
security-repo %
```

Obviously, this new rule is going to be created. These changes can be propagated using the [apply command](../commands/apply.md):

```bash
security-repo % lgc apply
[+] `rules/splunk/our-first-detection.yaml` will be created on service `splunk-prod`
Apply these changes? yes
`rules/splunk/our-first-detection.yaml` created on service `splunk-prod`
security-repo %
```

Now, edit the detection rule to change one or multiple parameters, and repeat the plan operation to highlight the changes.

```bash
security-repo % lgc plan --verbose
[~] `rules/splunk/our-first-detection.yaml` will be updated on service `splunk-prod`
---
   parameters.disabled: false => true
---
security-repo %
```

Use the `--verbose` flag to get detailed information about the changes. In this example, the `disabled` option was changed from **false** to **true**. This capability particularly shines in multi-lines searches, as illustrated below:

```bash
security-repo % lgc plan --verbose
[~] `rules/splunk/detect-high-entropy-domains.yaml` will be updated on service `splunk-prod`
---
   search:
        sourcetype=proxy
        | eval list = "mozilla"
        | `ut_parse(url, list)`
        | stats count by url_domain
        | `ut_shannon(url_domain)`
      - | where shannon_entropy > 3.8
      + | where shannon_entropy > 3.5
        | sort -count
        | rename count as total, url_domain as domain, shannon_entropy as entropy
---
security-repo %
```

As illustrated in the preceding example, the [plan command](../commands/plan.md) highlights exactly what has changed in the search (this is even more noticeable in a terminal with color contrast, which this doc lacks). 
