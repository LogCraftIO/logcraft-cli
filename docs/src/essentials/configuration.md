# Configuration

This section details the core options of `lgc.toml`. For services options, please refer to the appropriate [plugin documentation](../plugins/index.md).



## `workspace`

```toml
[core]
workspace = "rules"
```

This parameter defines the base directory in which detections are stored (default: rules). This parameter can be overridden with the environment variable `LGC_CORE_WORKSPACE`.

## `base_dir`

```toml
[core]
base_dir = "/opt/logcraft-cli"
```

This parameter defines the home directory of lgc, where the binary and plugins directory are located (default: `/opt/logcraft-cli`). This shouldn't be changed in most situations. This parameter can be overridden with the environment variable `LGC_CORE_BASE_DIR`.
