# Detections

This section aims at demystifying what security detections are and what is detection as code.

## What is a detection ?

A detection in LogCraft is **the smallest amount of _code_** that a security tool such as a SIEM, an EDR, a XDR, etc, considers at a time.

The outcome of a security detection is most often a security alert, that a SOAR or a security analyst investigate (or ignore :face_with_peeking_eye:)

Often, security detections are wrongly considered as only search queries. In reality, a detection is more complex as it embodies at least 3 things:

- The search query, of course. The _what_ to looking for.
- Some contextual information. Typically a description field containing information about the thing expected to be found.
- Optional parameters. This could range from common scheduling options to esoteric fields deeply tied to the technology used (hi security vendors :wave:).

## Plethora of query languages

First, it is important to understand that there isn't a single, universal format, to describe security detections _effectively_. And you probably noticed that security vendors love to introduce their own query language.

<!-- vale Google.We = NO -->

Moreover, at LogCraft, we recognize that each organization choose its own security tech stack.

<!-- vale Google.We = YES -->

This also means, that regardless of the detection technology chosen, you should be able to instrument detection as code (DaC) in your environment. That's a requirement for any SecOps team willing to operate at high speed and with high quality standards.

To accommodate for this broken up landscape, lgc relies on a plugin system to understand and interact with security tools such as SIEM, EDR, and XDR.

In short, lgc has as many plugins as there are technologies or format out there.

## Detections as code

Adopting detection as code (DaC) may influence your team’s workflows and operational structure, but for the better.

Detection as code (DaC) enables security teams to provision and manage security detections using code, rather than relying on manual processes and configurations.

Traditionally, security detections are developed in a testing environment, validated in staging, and then deployed to production. Without detection as code, this workflow is largely manual, making it time-consuming and error-prone, especially when managing detections at scale.

Detection as code allows defining the desired state of the detections, while LogCraft automates the intermediary deployment steps. This frees security experts to focus on developing and fine-tuning detections instead of manually migrating them across environments.

Adopting detection as code may require adjustments to team operations, as version control systems like GitLab or GitHub handle deployments, acting as a deployment server.

By ensuring consistency, traceability, and the ability to detect configuration drifts, detection as code helps teams verify that what is running in production aligns with the intended configuration.

Organizations implement detection as code to reduce risk and respond more quickly to emerging threats.

## Detections filenames

To store detection rules as individual files, the first step is to adopt a naming convention.

There are many conventions on naming things and the most popular ones are Snake case (`snake_case`), Kebab case (`kebab-case`), Camel case (`camelCase`) and Pascal case (`PascalCase`).

Examples:

- `my-detection.yaml`
- `my_detection.yaml`
- `myDetection.yaml`
- `MyDetection.yaml`

The convention your team choose doesn't change anything for LogCraft. Pick one that your team agrees on and stick to it, that's the most important part.

Finally, avoid mixing up naming styles, things get ~~ugly~~ more difficult to maintain :grimacing:

::: tip Filename and title
A detection commonly has a `title` field such as `title: My Awesome Detection`. While filenames aren't that important to lgc, it is best to name them according to the title of the detection. In this example, a good name could be `my-awesome-detection.yaml` and a bad name would be `something-unrelated.yaml`
:::
