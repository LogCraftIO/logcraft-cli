// Copyright (c) 2023 LogCraft, SAS.
// SPDX-License-Identifier: MPL-2.0

// TODO: Find a better way to manage configuration. 
const LGC_KCL_BASE: &str = r#"
import yaml
import file
import logcraft

project = logcraft.Project _config
_config = yaml.decode(file.read("lgc.yml"))
"#;

pub struct Plugin {
  name: String,
  path: Option<String>,
  url: Option<String>
}
