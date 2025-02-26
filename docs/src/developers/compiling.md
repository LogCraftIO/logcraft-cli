# Compiling

::: warning Compiling from the sources
This procedure should only be followed by contributors to the project. If you are a regular user or just want to give a try to `lgc`, follow [this procedure](/essentials/quickstart) that uses pre-compiled binaries instead.
:::

## Monorepo

LogCraft follows a monorepo approach with almost everything related to the command-line utility located in the same repository tree, including its documentation and all officially maintained plugins.

The first step in compiling lgc from the sources is to install [moonrepo](https://moonrepo.dev). Please refer to this official documentation.

## Building from the sources

Building `lgc` from the sources is pretty straightforward.

First, install [moonrepo](https://moonrepo.dev) on your system.

Then, run `moon lgc:build` from the root of the repository to build the command-line utility.

As lgc relies on plugins to interact with remote systems, run `moon '#plugin:build'` from the root of the repository to build all plugins. Alternatively, run `moon <PLUGIN_NAME>:build` to only build the desired plugin.

Finally, the resulting binaries are located in:
- `apps/lgc/target/release/lgc` (command-line utility)
- `plugins/<PLUGIN_NAME>/target/release/<PLUGIN_NAME>` (Plugin)

## Build commands

The following table summarise the most used commands to work in this repository.

| Command                    | What is does                   |
|----------------------------|--------------------------------|
| `moon :build`              | Build everything               |
| `moon lgc:build`           | Build the `lgc` command        |
| `moon '#plugin:build'`     | Build all plugins              |
| `moon <PLUGIN_NAME>:build` | Build the plugin <PLUGIN_NAME> |
| `moon docs:build`          | Build this documentation :)    |

Other `moon` commands can be useful depending on your need, please refer to the official documentation of moon.
