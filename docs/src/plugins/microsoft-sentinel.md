---
title: Microsoft Sentinel
category: SIEM
stage: beta
---

<!-- vale Google.Headings = NO -->

# Microsoft Azure Sentinel

<!-- vale Google.Headings = YES -->

<PluginStatus :category="$frontmatter.category" :stage="$frontmatter.stage" />

## File format

Microsoft Sentinel detections are normalized as follow:

```yaml
kind: Scheduled
name: <name of the detection>

properties:
  query: |-
    <search query>

  <other parameters>
```

This is a pretty simple and straightforward format (example below).

## File names

Each detection must be stored in its own YAML file under the plugin directory inside the workspace root.

Example:

- `rules/sentinel/detect-foo.yaml`
- `rules/sentinel/high-entropy-domain-name.yaml`

This ensure each detection is tracked individually.

## Example

```yaml
name: Some detection
kind: Scheduled
# ruleId: 04df2776-e230-4df0-9624-56364de3f902
properties:
  enabled: true
  severity: Medium
  query: |-
    AzureDiagnostics
    | where Category == 'JobLogs'
    | extend RunbookName = RunbookName_s
    | project TimeGenerated,RunbookName,ResultType,CorrelationId,JobId_g
    | summarize StartTime = minif(TimeGenerated,ResultType == 'Started'),EndTime = minif(TimeGenerated,ResultType in ('Completed','Failed','Failed')), Status = tostring(parse_json(make_list_if(ResultType,ResultType in  ('Completed','Failed','Stopped')))[0]) by JobId_g,RunbookName
    | extend DurationSec = datetime_diff('second', EndTime,StartTime)
    | join kind=leftouter (AzureDiagnostics
    | where Category == "JobStreams"
    | where StreamType_s == "Error"
    | summarize TotalErrors = dcount(StreamType_s) by JobId_g, StreamType_s) on $left. JobId_g == $right. JobId_g
    | extend HasErrors = iff(StreamType_s == 'Error',true,false)
    | project StartTime, EndTime, DurationSec,RunbookName,Status,HasErrors,TotalErrors,JobId_g
```
