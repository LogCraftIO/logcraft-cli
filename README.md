# LogCraft CLI

LogCraft CLI is an open-source tool developed by [LogCraft](https://www.logcraft.io) that simplifies the creation of Detection-as-Code pipelines while leveraging native Version Control System (VCS) capabilities such as GitLab.

With LogCraft CLI, you can easily deploy your security detections into your SIEM, EDR, XDR, and other modern security solutions.

---

**Documentation**: <a href="https://docs.logcraft.io" target="_blank">https://docs.logcraft.io</a>

**Source Code**: <a href="https://github.com/LogCraftIO/logcraft-cli" target="_blank">https://github.com/LogCraftIO/logcraft-cli</a>

---

## Roadmap

- **Alpha:** Current state, open for testing and feedback.
- **v0.1.0:** Planned release in June 2024, will include initial stable features.

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

While our project is open source and licensed under the [Mozilla Public License 2.0](./LICENSE), we understand that some users and organizations might require additional support to ensure seamless integration and optimal usage. To cater to these needs, we offer premium support services, which include:

- Dedicated Assistance: Get access to our team of experts who are well-versed in every aspect of this project, from threat detection to DevOps operations.
- Priority Response: Enjoy faster response times for your queries and issues.
- Custom Solutions: Receive tailored solutions and feature development to meet your specific requirements.
- Training Sessions: Participate in training sessions to help your team get up to speed with LogCraft efficiently.

If you're interested in our premium support services, contact us at hello@logcraft.io
