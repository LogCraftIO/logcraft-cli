# How to create a plugin

This page details the process of creating a plugin.

::: tip Programming Language
A plugin can be created in virtually any language (C/C++, Rust, Python, Go, and many other languages) as plugins are built using WebAssembly (wasm). See [plugins](../concepts/plugins.md#web-assembly-wasm) for more information.
:::

## Interfaces

A plugin, regardless of the language of implementation, needs to expose the following interfaces:

| Interface                                                | What it does                                |
| -------------------------------------------------------- | ------------------------------------------- |
| `create(config, name, params) -> result<string, string>` | Create a detection on the remote system     |
| `read(config, name, params) -> result<string, string>`   | Get/Read a detection from the remote system |
| `update(config, name, params) -> result<string, string>` | Update a detection on the remote system     |
| `delete(config, name, params) -> result<string, string>` | Remove a detection from the remote system   |
| `load() -> metadata`                                     | Load the plugin and return its metadata     |
| `settings() -> string`                                   | Returns the expected configuration          |
| `schema() -> string`                                     | Return the schema                           |
| `ping(config) -> result<bool: string>`                   | Open a connection to the remote system      |

::: details Wasm Interface (WIT)
Plugin interfaces are defined in the [Plugin Wasm Interface Type (WIT)](https://github.com/LogCraftIO/logcraft-cli/blob/main/libs/bindings/plugin.wit). Always refer to this file to ensure your interfaces return the appropriate data types.
:::

## Python plugin

This section details the steps to create a python plugin. Splunk is targeted for demonstration purpose but note that [an official Splunk plugin written in Rust exists](../plugins/splunk.md).

::: tip Python Global Interpreter (GIL)

<!-- vale Google.Will = NO -->

As a python developer, your probably know that python is a slow language and that's mainly a consequence of the Python Global Interpreter (GIL). Guess what? Compiling your python code in WebAssembly will actually make it fly :rocket:

<!-- vale Google.Will = YES -->

:::

### Package manager

This guide uses [Poetry](https://python-poetry.org/) to manage python packages but feel free to use your preferred package manager.

```bash
poetry new pysplunk
```

### Dependencies

Then, install `componentize-py`, a tool to convert a Python code to a WebAssembly component.

```bash
cd pysplunk
poetry add componentize-py
```

### WIT

Next, copy the [bindings/wit files](https://github.com/LogCraftIO/logcraft-cli/tree/main/libs/bindings) from the repository into the python app. These files are basically defining the contracts between lgc and the plugin.

```bash
~$ ls -1 pysplunk/wit
world.wit
plugin.wit
~$
```

### Bindings

This step is optional as it only creates python bindings in the working directory to integrate with your IDE. Later, when the python code is built as a WASM component, these bindings are automatically generated.

```bash
poetry run componentize-py --wit-path pysplunk/wit --world plugins bindings pysplunk
```

This results in a new directory `plugins` that contains the bindings in `pysplunk`.

### App

Then, create a `main.py` to implement the interfaces expected by the WIT files.

```python
#!/usr/bin/env python3
from plugins import Plugins
from plugins.exports.plugin import Metadata
from plugins.types import Err, Ok, Some, Result


class Plugin(Plugins):
    def load(self) -> Metadata:
        return Metadata("my-plugin", "0.1.0", "The Batman", "This is a famous plugin")

    def settings(self):
        pass

    def schema(self):
        pass

    def create(self, config: str, name: str, params: str):
        pass

    def read(self, config: str, name: str, params: str):
        pass

    def update(self, config: str, name: str, params: str):
        pass

    def delete(self, config: str, name: str, params: str):
        pass

    def ping(self, config: str) -> int:
        pass
```

The class name is important as it is inherited from the WIT files, hence the plugin must start with:

```python
class Plugin(Plugins):
```

::: warning
Regarding the `load()` function, make sure to respect the [identifiers convention](../concepts/identifiers.md) (kebab-case).
:::

Finally, compile the plugin:

```bash
poetry run componentize-py --wit-path pysplunk/wit --world plugins componentize -p pysplunk main -o my-plugin.wasm
```
