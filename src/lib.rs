
use wasm_bindgen::prelude::*;
use rhai::{Engine, Scope, Dynamic, Map, Array};
use std::cell::RefCell;
use std::collections::HashMap;
use std::panic::{self, AssertUnwindSafe};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window, js_name = wasm_print)]
    fn js_print(s: &str);
    
    #[wasm_bindgen(js_namespace = window, js_name = js_dom_set_html)]
    fn js_dom_set_html(html: &str);
    #[wasm_bindgen(js_namespace = window, js_name = js_dom_get_value)]
    fn js_dom_get_value(id: &str) -> String;
    #[wasm_bindgen(js_namespace = window, js_name = js_dom_set_value)]
    fn js_dom_set_value(id: &str, val: &str);
    #[wasm_bindgen(js_namespace = window, js_name = js_dom_set_inner)]
    fn js_dom_set_inner(id: &str, html: &str);

    #[wasm_bindgen(js_namespace = window, js_name = js_gfx_clear)]
    fn js_gfx_clear(color: &str);
    #[wasm_bindgen(js_namespace = window, js_name = js_gfx_rect)]
    fn js_gfx_rect(x: f64, y: f64, w: f64, h: f64, color: &str);
    #[wasm_bindgen(js_namespace = window, js_name = js_gfx_text)]
    fn js_gfx_text(x: f64, y: f64, text: &str, color: &str, font: &str, align: &str);
    #[wasm_bindgen(js_namespace = window, js_name = js_gfx_mode)]
    fn js_gfx_mode(mode: &str);
    
    #[wasm_bindgen(js_namespace = window, js_name = js_hardware_torch)]
    fn js_hardware_torch(enable: bool);
}

pub struct SystemState {
    pub vfs: HashMap<String, String>,
    pub app_ast: Option<rhai::AST>,
}

thread_local! {
    static STATE: RefCell<SystemState> = RefCell::new(SystemState { vfs: HashMap::new(), app_ast: None });
    static ENGINE: RefCell<Engine> = RefCell::new(create_engine());
}

fn create_engine() -> Engine {
    let mut engine = Engine::new();
    engine.set_max_operations(500_000); 

    engine.on_print(|s| js_print(s));
    engine.on_debug(|s, _src, _pos| js_print(&format!("[DEBUG] {}", s)));

    engine.register_fn("vfs_write_text", |path: String, content: String| { STATE.with(|s| s.borrow_mut().vfs.insert(path, content)); });
    engine.register_fn("vfs_read_text", |path: String| -> String { STATE.with(|s| s.borrow().vfs.get(&path).cloned().unwrap_or_else(|| "".to_string())) });
    
    engine.register_fn("dom_set_html", |html: &str| { js_dom_set_html(html); });
    engine.register_fn("dom_get_value", |id: &str| -> String { js_dom_get_value(id) });
    engine.register_fn("dom_set_value", |id: &str, val: &str| { js_dom_set_value(id, val); });
    engine.register_fn("dom_set_inner", |id: &str, html: &str| { js_dom_set_inner(id, html); });

    engine.register_fn("gfx_clear", |color: &str| { js_gfx_clear(color); });
    engine.register_fn("gfx_rect", |x: f64, y: f64, w: f64, h: f64, color: &str| { js_gfx_rect(x, y, w, h, color); });
    engine.register_fn("gfx_text", |x: f64, y: f64, text: &str, color: &str, font: &str, align: &str| { js_gfx_text(x, y, text, color, font, align); });
    engine.register_fn("gfx_mode", |mode: &str| { js_gfx_mode(mode); });

    engine.register_fn("hardware_torch", |enable: bool| { js_hardware_torch(enable); });

    engine
}

#[wasm_bindgen]
pub fn vfs_push_from_idb(path: &str, content: &str) { STATE.with(|s| s.borrow_mut().vfs.insert(path.to_string(), content.to_string())); }

#[wasm_bindgen]
pub fn app_run(script: &str) -> String {
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        ENGINE.with(|e| {
            let engine = e.borrow();
            match engine.compile(script) {
                Ok(ast) => {
                    STATE.with(|s| s.borrow_mut().app_ast = Some(ast.clone()));
                    let mut scope = Scope::new();
                    if let Err(err) = engine.eval_ast_with_scope::<()>(&mut scope, &ast) { return format!("❌ Setup Error: {}", err); }
                    match engine.call_fn::<()>(&mut scope, &ast, "init", ()) {
                        Ok(_) => "✅ App Initialized".to_string(),
                        Err(err) => {
                            if err.to_string().contains("not found") { "⚠️ Running (No init func)".to_string() } 
                            else { format!("❌ Init Error: {}", err) }
                        }
                    }
                },
                Err(err) => format!("❌ Compile Error: {}", err)
            }
        })
    }));
    match result { Ok(msg) => msg, Err(_) => "❌ FATAL: Rust WASM module panicked.".to_string() }
}

#[wasm_bindgen]
pub fn app_event(id: &str) -> String {
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        let ast_opt = STATE.with(|s| s.borrow().app_ast.clone());
        if let Some(ast) = ast_opt {
            ENGINE.with(|e| {
                let engine = e.borrow();
                let mut scope = Scope::new();
                match engine.call_fn::<()>(&mut scope, &ast, "on_event", (id.to_string(),)) {
                    Ok(_) => "".to_string(),
                    Err(err) => {
                        if err.to_string().contains("not found") { "".to_string() } 
                        else { format!("❌ Event Error: {}", err) }
                    }
                }
            })
        } else { "❌ System: No app is running.".to_string() }
    }));
    match result { Ok(msg) => msg, Err(_) => "❌ FATAL EVENT PANIC".to_string() }
}

#[wasm_bindgen]
pub fn app_stop() { STATE.with(|s| s.borrow_mut().app_ast = None); }
