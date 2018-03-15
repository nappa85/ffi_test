#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate dynamic_reload;
extern crate toml;

use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use std::time::Duration;
use std::thread;
use std::mem::transmute;

use self::dynamic_reload::{DynamicReload, Search, Lib, UpdateState, Symbol, PlatformName};

use self::toml::Value as TomlValue;

use self::serde_json::value::Value as JsonValue;

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    a: i32,
    b: i32,
    c: i32,
}

impl ToString for Request {
    fn to_string(&self) -> String {
        String::from(r#"{"a":"#) + &self.a.to_string() + r#","b":"# + &self.b.to_string() + r#","c":"# + &self.c.to_string() + "}"
    }
}

pub struct Plugin {
    name: String,
    config: Arc<RwLock<TomlValue>>,
    session: Arc<RwLock<HashMap<String, JsonValue>>>,
    plugins: Vec<Arc<Lib>>,
    symbols: Vec<Arc<Symbol<'static, extern "system" fn(config: *const Arc<RwLock<TomlValue>>, session: *const Arc<RwLock<HashMap<String, JsonValue>>>, secret: &str, request: *const &Request) -> *const Result<JsonValue, String>>>>,
}

impl Plugin {
    pub fn new(name: &str) -> Result<Plugin, String> {
        Ok(Plugin {
            name: name.to_owned(),
            config: Arc::new(RwLock::new(Plugin::load_config()?)),
            session: Arc::new(RwLock::new(HashMap::new())),
            plugins: Vec::new(),
            symbols: Vec::new()
        })
    }

    fn add_plugin(&mut self, plugin: &Arc<Lib>) {
        match unsafe { plugin.lib.get(b"test_call\0") } {
            Ok(temp) => {
                self.plugins.push(plugin.clone());
                let f: Symbol<extern "system" fn(config: *const Arc<RwLock<TomlValue>>, session: *const Arc<RwLock<HashMap<String, JsonValue>>>, secret: &str, request: *const &Request) -> *const Result<JsonValue, String>> = temp;
                self.symbols.push(Arc::new(unsafe { transmute(f) }));
            },
            Err(e) => println!("Failed to load symbol for {}: {:?}", self.name, e),
        }
    }

    fn unload_plugins(&mut self, lib: &Arc<Lib>) {
        for i in (0..self.plugins.len()).rev() {
            if &self.plugins[i] == lib {
                self.plugins.swap_remove(i);
                self.symbols.swap_remove(i);
            }
        }
    }

    fn reload_plugin(&mut self, lib: &Arc<Lib>) {
        Self::add_plugin(self, lib);
    }

    // called when a lib needs to be reloaded.
    fn reload_callback(&mut self, state: UpdateState, lib: Option<&Arc<Lib>>) {
        match state {
            UpdateState::Before => Self::unload_plugins(self, lib.unwrap()),
            UpdateState::After => Self::reload_plugin(self, lib.unwrap()),
            UpdateState::ReloadFailed(_) => println!("Failed to reload"),
        }
    }

    pub fn run(&self, secret: String, request: &Request) -> Result<&JsonValue, String> {
        if self.symbols.len() > 0 {
            let f = &self.symbols[0];
            println!("before");
            let res = f(Box::into_raw(Box::new(self.config.clone())), Box::into_raw(Box::new(self.session.clone())), &secret, Box::into_raw(Box::new(request)));
            println!("after");
            print!("config: ");
            match self.config.read() {
                Ok(cfg) => println!("Ok {}", cfg.to_string()),
                Err(e) => println!("Err {}", e),
            }
            print!("session: ");
            match self.session.read() {
                Ok(sess) => println!("Ok {:?}", sess),
                Err(e) => println!("Err {}", e),
            }
            println!("request {}", request.to_string());

            unsafe {
                if res.is_null() {
                    Err(format!("Null pointer exception"))
                }
                else {
                    match *res {
                        Ok(ref v) => Ok(v),
                        Err(ref e) => Err(e.to_string()),
                    }
                }
            }
        }
        else {
            Err(format!("Lib {} not loaded", self.name))
        }
    }

    fn load_config() -> Result<TomlValue, String> {
        match toml::from_str(r#"key = "value""#) {
            Ok(config) => Ok(config),
            Err(e) => Err(format!("Syntax error on Toml: {:?}", e)),
        }
    }
}

fn main() {
    let mut reload_handler = DynamicReload::new(Some(vec!["test_lib/target/debug"]), Some("test_lib/target/debug"), Search::Default);

    let mut plugin = Plugin::new("test_lib").unwrap();
    match reload_handler.add_library("test_lib", PlatformName::Yes) {
        Ok(lib) => plugin.add_plugin(&lib),
        Err(e) => {
            println!("Unable to load dynamic lib, err {:?}", e);
            return;
        }
    }

    loop {
        reload_handler.update(Plugin::reload_callback, &mut plugin);

        let req = Request {
            a: 1,
            b: 2,
            c: 3,
        };

        let res = plugin.run(String::from("test"), &req);
        println!("Returned");
        match res {
            Ok(value) => println!("OK: {}", value.to_string()),
            Err(e) => println!("ERR: {}", e),
        }

        // Wait for 0.5 sec
        thread::sleep(Duration::from_millis(500));
    }
}
