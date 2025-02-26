# State

lgc must store state about your managed detection rules. This state is used by lgc to determine which changes to make. In short, the state is critical in understanding what has been deployed and where it was deployed.

::: danger Important
Never edit the state manually and let `lgc` manage it properly.
:::

## local

By default, the state is stored locally in a file named `.logcraft/state.json`. This is the case when you run lgc on your workstation for example.

```toml
[state]
type =  "local"
path = ".logcraft/state.json"
```

This whole block is hidden by default in **lgc.toml**.

## http

When lgc is used in a git environment such as GitLab, GitHub and other Version Control Systems (VCS), it is recommended to host the state in a http backend.

The main benefit of using a http state store is centralised management. The state becomes accessible to all CI/CD jobs and team members, ensuring that multiple developers and CI/CD pipelines do not have conflicting or inconsistent state files, which could lead to infrastructure drift or errors.

In addition, the state is automatically locked during operations to prevent simultaneous writes, ensuring safe and consistent updates.

To activate the http backend, edit the `lgc.toml` as follow:

```toml
[state]
type =  "http"
```

Then, it is recommended to adjust the state settings directly in your CI/CD template (see [GitLab template example](../essentials/gitlab.md#gitlab-template)) rather than in lgc.toml, but if you absolutely need to, lgc.toml accepts these parameters too.


```toml
[state]
type =  "http"

# required backend parameters
address = ""
username = ""
password = ""
lock_address = ""
lock_method = ""
unlock_address = ""
unlock_method = ""

# optional backend parameters
update_method = ""
skip_cert_verification  = ""
timeout = ""
client_ca_certificate_pem = ""
client_certificate_pem = ""
client_private_key_pem = ""
headers = ""
```
