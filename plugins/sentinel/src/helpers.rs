// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

pub fn filter_response(
    detection: &serde_json::Value,
    mut response: serde_json::Value,
) -> serde_json::Value {
    match (detection, &mut response) {
        // If both detection and response are objects, iterate and filter in place.
        (serde_json::Value::Object(det_obj), serde_json::Value::Object(resp_obj)) => {
            // Collect keys from response as we may remove some.
            let keys: Vec<String> = resp_obj.keys().cloned().collect();
            for key in keys {
                if let Some(det_val) = det_obj.get(&key) {
                    // Recursively filter the value if the key exists in detection.
                    if let Some(entry) = resp_obj.get_mut(&key) {
                        let filtered = filter_response(det_val, std::mem::take(entry));
                        *entry = filtered;
                    }
                } else {
                    // Remove keys that are not present in detection.
                    resp_obj.remove(&key);
                }
            }
            response
        }
        // If both detection and response are arrays, use the first element of detection as a template.
        (serde_json::Value::Array(det_arr), serde_json::Value::Array(resp_arr)) => {
            if let Some(template) = det_arr.first() {
                for item in resp_arr.iter_mut() {
                    let filtered = filter_response(template, std::mem::take(item));
                    *item = filtered;
                }
            }
            response
        }
        // For non-object types, just return the response as is.
        _ => response,
    }
}
