# LogCraft CLI

LogCraft CLI is an open-source tool that simplifies the creation of Detection-as-Code pipelines while leveraging native Version Control System (VCS) capabilities such as GitLab.

With LogCraft CLI, you can easily deploy your security detections into your SIEM, EDR, XDR, and other modern security solutions.

---

**Documentation**: <a href="https://docs.logcraft.io" target="_blank">https://docs.logcraft.io</a>

**Source Code**: <a href="https://github.com/LogCraftIO/logcraft-cli" target="_blank">https://github.com/LogCraftIO/logcraft-cli</a>

**Plugins**: <a href="https://github.com/LogCraftIO/logcraft-cli-plugins" target="_blank">https://github.com/LogCraftIO/logcraft-cli-plugins</a>

---

## Versions

- **v0.1.0:** 2024-06-24, initial stable release.

## Download LogCraft CLI
To download the latest stable version of LogCraft CLI, simply [go to the release page](https://github.com/LogCraftIO/logcraft-cli/releases) and pick the latest available version for your architecture.

Once downloaded, add the `lgc` binary to your PATH, for example in `/usr/local/bin`

```bash
~$ tar xf lgc-x86_64-unknown-linux-gnu.tar.xz
~$ sudo cp lgc-x86_64-unknown-linux-gnu/lgc /usr/local/bin/
```
Finally, ensure lgc is correctly installed

```bash
~$ cd
~$ lgc --version
LogCraft CLI v0.1.0
~$
```
Congratulation, LogCraft CLI is installed and working :tada:

## Build from sources

```bash
git clone git@github.com:LogCraftIO/logcraft-cli.git
cd logcraft-cli
cargo build --release
```

The resulting binary will be available at `./target/release/lgc`

Once built, add the `lgc` binary to your `PATH`, for example in `/usr/local/bin`

```bash
sudo cp target/release/lgc /usr/local/bin/
```

Finally, ensure `lgc` is correctly installed

```bash
cd
lgc --version
```

## Support

### Community Support

We highly encourage community contributions, ranging from feature requests and bug reports to plugin creation. We are always here to support you through our community channels.

Join our public [Slack channel](https://join.slack.com/t/logcraft/shared_invite/zt-2jdw7ntts-yVhw8rIji5ZFpPt_d6HM9w) for free support and collaboration with other users and developers.

### Premium Support 

While our project is open source and licensed under the [Mozilla Public License 2.0](./LICENSE), we understand that some users and organizations might require additional support to ensure seamless integration and optimal usage. To cater to these needs, we offer premium support services, which include dedicated support, priority response, custom solutions, training sessions and more.

Contact us at hello@logcraft.io to learn more about our premium services

## Contribute!

We warmly invite developers, security enthusiasts, and professionals to connect and contribute to LogCraft CLI. 

Your expertise and insights can greatly enhance LogCraft CLI and whether it's by contributing code, reporting issues, or sharing your unique perspectives, your participation is invaluable to us. 

Join our [community](https://join.slack.com/t/logcraft/shared_invite/zt-2jdw7ntts-yVhw8rIji5ZFpPt_d6HM9w) and help us democratize Detection-as-Code.
