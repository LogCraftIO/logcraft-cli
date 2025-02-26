---
title: Google Chronicle (SecOps)
category: SIEM
stage: planning
---

<!-- vale Google.Headings = NO -->

# Google Chronicle (SecOps)

<!-- vale Google.Headings = YES -->

<PluginStatus :category="$frontmatter.category" :stage="$frontmatter.stage" />

::: tip Need This?
[Open a ticket](https://github.com/LogCraftIO/logcraft-cli/issues) or [reach out](mailto:hello@logcraft.io) to initiate the integration of this technology :tada:
:::

## Example

```
rule malware_httpbrowser
{
  meta:
    author = "Google Cloud Security"
    description = "HTTPBrowser malware"
    reference1 = "https://attack.mitre.org/software/S0070/"
    reference2 = "https://www.zscaler.com/blogs/research/chinese-cyber-espionage-apt-group-leveraging-recently-leaked-hacking-team-exploits-target-financial-services-firm"
    yara_version = "YL2.0"
    rule_version = "1.0"

  events:
    (
      $e1.metadata.event_type = "REGISTRY_CREATION" and
      re.regex($e1.target.registry.registry_key, `(HKCU|HKEY_CURRENT_USER)\\Software\\Microsoft\\Windows\\CurrentVersion\\Run`) nocase and
      $e1.target.registry.registry_value_name = "wdm" nocase
    )
    or
    (
      $e1.metadata.event_type = "FILE_CREATION" and
      re.regex($e1.target.file.full_path, `\\vpdn\\VPDN_LU.exe$`) nocase
    )
    or
    (
      $e1.network.http.user_agent = "HttpBrowser/1.0" and
      re.regex($e1.target.url, `/.*c=.*&l=.*&o=.*&u=.*&r=`)
    )

  condition:
    $e1
}
```

Source: https://github.com/chronicle/detection-rules/blob/main/malware/httpbrowser.yaral
