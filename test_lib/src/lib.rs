#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate toml;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use serde_json::value::Value as JsonValue;

use toml::Value as TomlValue;

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    a: i32,
    b: i32,
    c: i32,
}

fn get_json(cfg: &str) -> String {
    String::from(r#"{
    "name": ""#) + cfg + r#"",
    "age": 43,
    "phones": [
        "+44 1234567",
        "+44 2345678"
    ]
}"#
}

#[no_mangle]
pub extern fn test_call(ptr_config: *const Arc<RwLock<TomlValue>>, ptr_session: *const Arc<RwLock<HashMap<String, JsonValue>>>, secret: &str, ptr_request: *const &Request) -> *const Result<JsonValue, String> {
    let config = unsafe {
        assert!(!ptr_config.is_null());
        &*ptr_config
    };
    let session = unsafe {
        assert!(!ptr_session.is_null());
        &*ptr_session
    };
    let request = unsafe {
        assert!(!ptr_request.is_null());
        &*ptr_request
    };

    Box::into_raw(Box::new(match config.read() {
        Ok(cfg) => Ok(serde_json::from_str(&get_json(cfg["key"].as_str().unwrap())).unwrap()),
        Err(e) => Err(format!("{}", e)),
    }))
}
