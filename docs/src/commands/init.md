# lgc init

This command helps you kickstart the project by creating the base configuration file `lgc.toml`.

```bash
% lgc init
INFO `lgc.toml` saved
% ls -1
lgc.toml
%
```

Without parameters, `lgc init` creates the configuration file in the current directory.

If a `lgc.toml` already exists, the command fails.

```bash
% lgc init
ERROR `lgc.toml` already exists
%
```

## Options

<!-- vale Google.Headings = NO -->

### --root/-r

<!-- vale Google.Headings = YES -->

The `root` option allows specifying where to initialize the configuration (defaults to `.`)

The following example initialize the configuration in the directory `/foo/bar`

```bash
% lgc init --root /foo/bar
INFO `lgc.toml` saved
% ls -1 /foo/bar
lgc.toml
%
```

If the provided path doesn't exist, an error is thrown.

```bash
% lgc init --root /foo/baz
ERROR directory `/foo/baz` does not exist
%
```

<!-- vale Google.Headings = NO -->

### --workspace/-w

<!-- vale Google.Headings = YES -->

This parameter allows defining the base directory in which detections are stored (default: `rules`). This is a sub directory of the root folder.

```bash
% lgc init --root /foo/bar --workspace 'my-rules'
INFO `lgc.toml` saved
% ls -1 /foo/bar
lgc.toml
% cat /foo/bar/lgc.toml | grep workspace
workspace = "my-rules"
%
```

<!-- vale Google.Headings = NO -->

### --create/-c

<!-- vale Google.Headings = YES -->

When this parameter is specified, lgc creates the workspace in the root directory (default: `false`).

```bash
% lgc init --create
INFO workspace directory `rules` created
INFO `lgc.toml` saved
% ls -1
lgc.toml
rules/
%
```

If a directory of the same name already exists, an error is thrown.

```bash
% lgc init --create
ERROR workspace directory `rules` already exists
%
```
