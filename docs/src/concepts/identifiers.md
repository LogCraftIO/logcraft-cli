# Identifiers

An identifier is a unique ID that references a service or an environment and must be unique across all configuration. An identifier can be anything you like as long as it is expressed in [kebab-case (lowercase)](https://developer.mozilla.org/en-US/docs/Glossary/Kebab_case).

::: tip Valid identifiers

- `prod`
- `siem-prod`
- `service-2-0`
  :::

::: warning Invalid identifiers

- `PROD` _(identifiers must be lowercase)_
- `siem_prod` _(underscore isn't accepted as a separator)_
- `service-2.0` _(the dot isn't accepted either)_
- `foo-` _(ending hyphen)_
- `-foo` _(leading hyphen)_
- `foo----bar` _(multiple hyphens)_
  :::

To create a service named `my-service`, either use the [services command](../commands/services.md) (recommended approach) or manually edit the configuration file to add the following block.

```toml
[services]
[services.my-service]
... service definition ...
```

The service can then be referenced in the command line, for example:

```sh
% lgc ping my-service
```

::: info Uniqueness
Environments are loosely defined, so make sure they don't overlap with services names. For example, declaring a `prod` environment with a `siem-prod` service belonging to that environment is possible (and recommended!). However, if for some - weird - reason you wanted to have a service `foo` and an environment of the same name, that is not possible as lgc won't be able to distinguish the service from the environment.
:::
