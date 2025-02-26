# lgc ping

This command validates network connectivity in between lgc and the remote systems by opening a network connection to one or more services. This is a troubleshooting command.

```bash
% lgc ping [<IDENTIFIER>]
```

When ping is called without parameter, all services are contacted.

```bash
% lgc ping
splunk-prod... OK
splunk-dev... when calling ping for plugin `splunk`: ErrorCode::DnsError(DnsErrorPayload { rcode: Some("address not available"), info-code: Some(0) })
tanium-prod... OK
%
```

If the provided identifier is a service, ping only connects to the specified service.

```bash
% lgc ping my-service
my-service... OK
%
```

Finally, if the provided identifier is an environment, then all services belonging to that environment are contacted.

```bash
% lgc ping prod
splunk-prod... OK
tanium-prod... OK
%
```

::: tip
Technically speaking, `lgc ping` opens a socket to the remote host. This ensures that name resolution (DNS), routing, and firewall rules are correctly configured.
:::
