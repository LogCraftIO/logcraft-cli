---
title: LimaCharlie
category: EDR
stage: planning
---

# LimaCharlie

<PluginStatus :category="$frontmatter.category" :stage="$frontmatter.stage" />

::: tip Need This?
[Open a ticket](https://github.com/LogCraftIO/logcraft-cli/issues) or [reach out](mailto:hello@logcraft.io) to initiate the integration of this technology :tada:
:::

## Example

```yaml
# Detection
op: ends with
event: NEW_PROCESS
path: event/FILE_PATH
value: wanadecryptor.exe
case sensitive: false

# Response
- action: report
  name: wanacry
- action: task
  command: history_dump
- action: task
  command:
    - deny_tree
    - <<routing/this>>
```
