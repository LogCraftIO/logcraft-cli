# lgc services

This command lets you manage the services your organisation rely on for its defenses, for example a SIEM instance, an EDR appliance and any other security tools.

A service must be associated to a specific plugin, a specific technology, and most often services also belong to environments (*production*, *development*, *staging*, etc).

::: tip 1 service = 1 plugin
lgc relies on a plugin system to integrate with various technologies. A service is an instantiation of a plugin with specific contextual parameters.
:::

## lgc services create

### Usage

This command creates a new service.

The following command creates a new service in interactive mode:

```sh
% lgc services create
Select the plugin to use: splunk
Service identifier: splunk-prod
Environment name (optional): prod
Do you want to configure the plugin now? (y/n) n
INFO service `splunk-prod` successfully created.
%
```

These parameters can also be set from the command line as illustrated below:

```sh
% lgc services create -i splunk-prod -p splunk -e prod
INFO service `splunk-prod` successfully created.
%
```

### Options

| Parameter                       | Required? | Purpose                                              |
| ------------------------------- | --------- | ---------------------------------------------------- |
| `--identifier\|-i <IDENTIFIER>` | Required  | Set the new service identifier                       |
| `--plugin\|-p <PLUGIN_NAME>`    | Required  | Plugin to use                                        |
| `--env\|-e <IDENTIFIER>`        | Optional  | Environment name this service belongs to             |
| `--configure\|-c`               | Optional  | If set, launch the interactive service configuration |

## lgc services configure

This command configures a service previously created using [`lgc services create`](#lgc-services-create).

A service is associated to a plugin so each service has different parameters and this command lets you tune these parameters to match your environment.

```sh
% lgc services configure <IDENTIFIER>
// interactive prompt with plugin specific parameters
%
```

::: tip Configure a service at creation
The plugin configuration can occur right after the service creation using the `--configure` parameter. See [`lgc services create`](#lgc-services-create) for details.
:::

## lgc services list

This command lists existing services defined in the configuration file.

```sh
% lgc services list
---
service    : mspr-stnl
environment: staging
plugin     : sentinel
---
service    : shc-prd-01
environment: production
plugin     : splunk
%
```

## lgc services remove

This command deletes an existing service.

```sh
% lgc services remove <IDENTIFIER>
```

Note that the service definition is removed from the configuration file but detections aren't impacted. To clear a remote system, see [`lgc destroy`](destroy.md)
