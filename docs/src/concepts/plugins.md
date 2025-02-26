# Plugins

LogCraft rely on plugins to connect to remote security systems.

<!-- vale Google.Headings = NO -->

## Web Assembly (WASM)

<!-- vale Google.Headings = YES -->

LogCraft plugins are built on the open standard WebAssembly (WASM).

**These plugins run in memory-safe sandboxes**, making them secure by design. Beyond security, WASM modules offer exceptional performance due to their low-level binary format, optimized for modern processors. **This enables near-native execution speeds**.

Additionally, WASM supports a wide range of programming languages, allowing LogCraft plugins to be [developed](../developers/how-to-create-plugins.md) in almost any language.

::: tip NIST
In a recent study, [NIST](https://csrc.nist.gov/) emphasized the use of WebAssembly to enhance data protection strategies ([NIST IR 8505](https://csrc.nist.gov/News/2024/nist-has-published-nist-ir-8505))
:::
