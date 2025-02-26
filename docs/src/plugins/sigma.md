---
title: Sigma
category: Format
stage: alpha
---

# Sigma

<PluginStatus :category="$frontmatter.category" :stage="$frontmatter.stage" />

::: tip Work In Progress
The Sigma plugin is currently under development. If you want to contribute by testing this plugin, just [reach out](mailto:hello@logcraft.io)
:::

## Example

```yaml
title: Exploitation Indicator Of CVE-2022-42475
id: 293ccb8c-bed8-4868-8296-bef30e303b7e
status: test
description: Detects exploitation indicators of CVE-2022-42475 a heap-based buffer overflow in sslvpnd.
references:
    - https://www.fortiguard.com/psirt/FG-IR-22-398
    - https://www.bleepingcomputer.com/news/security/fortinet-says-ssl-vpn-pre-auth-rce-bug-is-exploited-in-attacks/
    - https://www.deepwatch.com/labs/customer-advisory-fortios-ssl-vpn-vulnerability-cve-2022-42475-exploited-in-the-wild/
    - https://community.fortinet.com/t5/FortiGate/Technical-Tip-Critical-vulnerability-Protect-against-heap-based/ta-p/239420
author: Nasreddine Bencherchali (Nextron Systems), Nilaa Maharjan, Douglasrose75
date: 2024-02-08
tags:
    - attack.initial-access
    - cve.2022-42475
    - detection.emerging-threats
logsource:
    product: fortios
    service: sslvpnd
    definition: 'Requirements: file creation events or equivalent must be collected from the FortiOS SSL-VPN appliance in order for this detection to function correctly'
detection:
    keywords:
        - '/data/etc/wxd.conf'
        - '/data/lib/libgif.so'
        ...
    condition: keywords
falsepositives:
    - Unknown
level: high
```

Source: https://github.com/SigmaHQ/sigma/blob/master/rules-emerging-threats/2022/Exploits/CVE-2022-42475/fortios_sslvpnd_exploit_cve_2022_42475_exploitation_indicators.yml/
