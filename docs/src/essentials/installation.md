# Installation

## Getting the bits

Download the latest stable version of lgc from the project's [release page](https://github.com/LogCraftIO/logcraft-cli/releases).

This table summarise the available assets:

| Files                            | What it is                     |
|----------------------------------|--------------------------------|
| `lgc-<os>-<arch>.tar.gz`         | command-line tool with plugins |
| `lgc-minimal-<os>-<arch>.tar.gz` | command-line tool only         |
| `plugins.tar.gz`                 | All plugins                    |
| `<plugin_name>.wasm`             | A specific plugin              |


In general, if you are just starting with lgc, choose the global package `lgc-<os>-<arch>.tar.gz` such as `lgc-linux-amd64.tar.gz` to get the command-line tool and all plugins.

## Local installation

While lgc has been built to be used in CI/CD pipelines, it can still be used locally, on a workstation.

```bash
% mkdir /opt/logcraft-cli
% tar xzf lgc-linux-amd64.tar.gz -C /opt/logcraft-cli
% ls -1 /opt/logcraft-cli
lgc
plugins/
README.md
LICENSE
%
```

Then, add lgc to the system's `PATH` by creating a symlink:

```bash
% sudo ln -s /opt/logcraft-cli/lgc /usr/local/bin/lgc
```

::: details Alternative approach
Instead of creating a symlink, add `/opt/logcraft-cli` to the system's PATH 
:::

Finally, ensure lgc is correctly setup

```bash
% cd
% lgc --version
LogCraft CLI v0.2.0
...
%
```
Congratulations, lgc is now installed on your system 🎉

## CI/CD installation

To easily use LogCraft in GitLab, GitHub, Bitbucket or any other version control system (VCS), pre-built containers are available.


::: tip 0-CVE containers
LogCraft's containers use [Wolfi "Zero-CVE" images](https://www.chainguard.dev), which are specifically designed to minimize the attack surface and enhance the security of the software supply chain.
:::

### GitLab

```yaml
image:
  name: "ghcr.io/logcraftio/logcraft-cli:latest"
```

Refer to the [GitLab Integration guide](./gitlab.md) for detailed instructions in setting up LogCraft in GitLab
