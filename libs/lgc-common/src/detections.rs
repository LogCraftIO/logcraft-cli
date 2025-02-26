// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use serde_json::Value;
use std::collections;

// Helper types to store detections per service
pub type PluginsDetections = collections::HashMap<String, collections::HashMap<String, Value>>;

/// Detection type alias for a detection path and its content.
pub type Detection = (String, Vec<u8>);
