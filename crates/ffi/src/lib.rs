use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::sync::Arc;
use tokio::runtime::Runtime;

use blaniel_sdk::{BlanielClient, BlanielConfig};

thread_local! {
    static LAST_ERROR: std::cell::RefCell<String> = std::cell::RefCell::new(String::new());
}

fn set_error(msg: String) {
    LAST_ERROR.with(|e| *e.borrow_mut() = msg);
}

fn ok_json(val: &str) -> *mut c_char {
    CString::new(val).unwrap().into_raw()
}

fn err_json(msg: &str) -> *mut c_char {
    let obj = serde_json::json!({"error": msg});
    match serde_json::to_string(&obj) {
        Ok(s) => CString::new(s).unwrap().into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

struct FfiState {
    client: BlanielClient,
    rt: Runtime,
}

static mut FFI_STATE: Option<Arc<FfiState>> = None;

unsafe fn with_state<F, T: serde::Serialize>(f: F) -> *mut c_char
where
    F: FnOnce(&BlanielClient, &Runtime) -> Result<T, blaniel_sdk::SdkError>,
{
    let state = match FFI_STATE.clone() {
        Some(s) => s,
        None => {
            let msg = "Not initialized. Call blaniel_init() first.".to_string();
            set_error(msg.clone());
            return err_json(&msg);
        }
    };

    match f(&state.client, &state.rt) {
        Ok(val) => match serde_json::to_string(&val) {
            Ok(s) => ok_json(&s),
            Err(e) => {
                let msg = format!("Serialization error: {}", e);
                set_error(msg.clone());
                ptr::null_mut()
            }
        },
        Err(e) => {
            let msg = e.to_string();
            set_error(msg.clone());
            err_json(&msg)
        }
    }
}

fn parse_cstr(ptr: *const c_char, name: &str) -> Result<String, String> {
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map(|s| s.to_string())
        .map_err(|_| format!("Invalid {} string", name))
}

#[no_mangle]
pub unsafe extern "C" fn blaniel_init(
    api_url: *const c_char,
    api_key: *const c_char,
    timeout_ms: u64,
) -> bool {
    let url = match parse_cstr(api_url, "API URL") {
        Ok(s) => s,
        Err(e) => {
            set_error(e);
            return false;
        }
    };
    let key = match parse_cstr(api_key, "API key") {
        Ok(s) => s,
        Err(e) => {
            set_error(e);
            return false;
        }
    };

    let config = BlanielConfig::new(&url, &key).with_timeout(timeout_ms);
    let client = match BlanielClient::new(config) {
        Ok(c) => c,
        Err(e) => {
            set_error(e.to_string());
            return false;
        }
    };
    let rt = match Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            set_error(format!("Runtime error: {}", e));
            return false;
        }
    };

    FFI_STATE = Some(Arc::new(FfiState { client, rt }));
    true
}

#[no_mangle]
pub unsafe extern "C" fn blaniel_cleanup() {
    FFI_STATE = None;
}

#[no_mangle]
pub unsafe extern "C" fn blaniel_last_error() -> *mut c_char {
    LAST_ERROR.with(|e| {
        let borrowed = e.borrow();
        if borrowed.is_empty() {
            ptr::null_mut()
        } else {
            CString::new(borrowed.as_str()).unwrap().into_raw()
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn blaniel_free_string(s: *mut c_char) {
    if !s.is_null() {
        let _ = CString::from_raw(s);
    }
}

#[no_mangle]
pub unsafe extern "C" fn blaniel_health() -> *mut c_char {
    with_state(|c, rt| rt.block_on(c.health()))
}

#[no_mangle]
pub unsafe extern "C" fn blaniel_get_npc(npc_id: *const c_char) -> *mut c_char {
    let id = match parse_cstr(npc_id, "NPC ID") {
        Ok(s) => s,
        Err(e) => return err_json(&e),
    };
    with_state(|c, rt| rt.block_on(c.get_npc(&id)))
}

#[no_mangle]
pub unsafe extern "C" fn blaniel_chat(
    npc_id: *const c_char,
    message: *const c_char,
) -> *mut c_char {
    let id = match parse_cstr(npc_id, "NPC ID") {
        Ok(s) => s,
        Err(e) => return err_json(&e),
    };
    let msg = match parse_cstr(message, "message") {
        Ok(s) => s,
        Err(e) => return err_json(&e),
    };
    with_state(|c, rt| rt.block_on(c.chat_simple(&id, &msg)))
}

#[no_mangle]
pub unsafe extern "C" fn blaniel_move_npc(
    npc_id: *const c_char,
    x: f64,
    y: f64,
    z: f64,
    world: *const c_char,
) -> *mut c_char {
    let id = match parse_cstr(npc_id, "NPC ID") {
        Ok(s) => s,
        Err(e) => return err_json(&e),
    };
    let w = match parse_cstr(world, "world") {
        Ok(s) => s,
        Err(e) => return err_json(&e),
    };
    let pos = npc_types::Position3D::new(x, y, z, w);
    with_state(|c, rt| rt.block_on(c.move_npc(&id, pos, None, None)))
}

#[no_mangle]
pub unsafe extern "C" fn blaniel_navigate_to(
    npc_id: *const c_char,
    target_name: *const c_char,
) -> *mut c_char {
    let id = match parse_cstr(npc_id, "NPC ID") {
        Ok(s) => s,
        Err(e) => return err_json(&e),
    };
    let name = match parse_cstr(target_name, "target") {
        Ok(s) => s,
        Err(e) => return err_json(&e),
    };
    let target = npc_types::NavigationTarget::Word { name };
    with_state(|c, rt| rt.block_on(c.navigate(&id, target, None)))
}

#[no_mangle]
pub unsafe extern "C" fn blaniel_navigate_to_coords(
    npc_id: *const c_char,
    x: f64,
    y: f64,
    z: f64,
    world: *const c_char,
) -> *mut c_char {
    let id = match parse_cstr(npc_id, "NPC ID") {
        Ok(s) => s,
        Err(e) => return err_json(&e),
    };
    let w = match parse_cstr(world, "world") {
        Ok(s) => s,
        Err(e) => return err_json(&e),
    };
    let target = npc_types::NavigationTarget::Coordinate {
        position: npc_types::Position3D::new(x, y, z, w),
    };
    with_state(|c, rt| rt.block_on(c.navigate(&id, target, None)))
}

#[no_mangle]
pub unsafe extern "C" fn blaniel_pathfind(
    npc_id: *const c_char,
    from_x: f64,
    from_y: f64,
    from_z: f64,
    to_x: f64,
    to_y: f64,
    to_z: f64,
    world: *const c_char,
) -> *mut c_char {
    let id = match parse_cstr(npc_id, "NPC ID") {
        Ok(s) => s,
        Err(e) => return err_json(&e),
    };
    let w = match parse_cstr(world, "world") {
        Ok(s) => s,
        Err(e) => return err_json(&e),
    };
    let start = npc_types::Position3D::new(from_x, from_y, from_z, w.clone());
    let goal = npc_types::Position3D::new(to_x, to_y, to_z, w);
    with_state(|c, rt| rt.block_on(c.pathfind(&id, start, goal, None)))
}

#[no_mangle]
pub unsafe extern "C" fn blaniel_get_nearby_npcs(
    x: f64,
    y: f64,
    z: f64,
    radius: f64,
    world: *const c_char,
) -> *mut c_char {
    let w = match parse_cstr(world, "world") {
        Ok(s) => s,
        Err(e) => return err_json(&e),
    };
    with_state(|c, rt| rt.block_on(c.get_nearby_npcs(x, y, z, &w, radius)))
}

#[no_mangle]
pub unsafe extern "C" fn blaniel_get_navigation_config(npc_id: *const c_char) -> *mut c_char {
    let id = match parse_cstr(npc_id, "NPC ID") {
        Ok(s) => s,
        Err(e) => return err_json(&e),
    };
    with_state(|c, rt| rt.block_on(c.get_navigation_config(&id)))
}

#[no_mangle]
pub unsafe extern "C" fn blaniel_set_navigation_config(
    npc_id: *const c_char,
    config_json: *const c_char,
) -> *mut c_char {
    let id = match parse_cstr(npc_id, "NPC ID") {
        Ok(s) => s,
        Err(e) => return err_json(&e),
    };
    let json_str = match parse_cstr(config_json, "config") {
        Ok(s) => s,
        Err(e) => return err_json(&e),
    };
    let config: npc_types::NavigationConfig = match serde_json::from_str(&json_str) {
        Ok(c) => c,
        Err(e) => return err_json(&format!("Invalid config JSON: {}", e)),
    };
    with_state(|c, rt| rt.block_on(c.set_navigation_config(&id, &config)))
}

#[no_mangle]
pub unsafe extern "C" fn blaniel_register_scene(scene_json: *const c_char) -> *mut c_char {
    let json_str = match parse_cstr(scene_json, "scene") {
        Ok(s) => s,
        Err(e) => return err_json(&e),
    };
    let req: npc_types::RegisterSceneRequest = match serde_json::from_str(&json_str) {
        Ok(r) => r,
        Err(e) => return err_json(&format!("Invalid scene JSON: {}", e)),
    };
    let scene_id = req.scene_id.clone();
    let world = req.world.clone();
    let positions = req.positions;
    with_state(|c, rt| rt.block_on(c.register_scene(&scene_id, &world, positions)))
}

#[no_mangle]
pub unsafe extern "C" fn blaniel_get_action_config(npc_id: *const c_char) -> *mut c_char {
    let id = match parse_cstr(npc_id, "NPC ID") {
        Ok(s) => s,
        Err(e) => return err_json(&e),
    };
    with_state(|c, rt| rt.block_on(c.get_action_config(&id)))
}

#[no_mangle]
pub unsafe extern "C" fn blaniel_set_action_config(
    npc_id: *const c_char,
    config_json: *const c_char,
) -> *mut c_char {
    let id = match parse_cstr(npc_id, "NPC ID") {
        Ok(s) => s,
        Err(e) => return err_json(&e),
    };
    let json_str = match parse_cstr(config_json, "config") {
        Ok(s) => s,
        Err(e) => return err_json(&e),
    };
    let req: npc_types::SetActionSystemConfigRequest = match serde_json::from_str(&json_str) {
        Ok(r) => r,
        Err(e) => return err_json(&format!("Invalid config JSON: {}", e)),
    };
    with_state(|c, rt| rt.block_on(c.set_action_config(&id, &req)))
}

#[no_mangle]
pub unsafe extern "C" fn blaniel_list_llm_providers() -> *mut c_char {
    with_state(|c, rt| rt.block_on(c.list_llm_providers()))
}

#[no_mangle]
pub unsafe extern "C" fn blaniel_get_llm_presets() -> *mut c_char {
    with_state(|c, rt| rt.block_on(c.get_llm_presets()))
}

#[no_mangle]
pub unsafe extern "C" fn blaniel_register_llm_provider(
    provider_type: *const c_char,
    api_key: *const c_char,
    model: *const c_char,
    base_url: *const c_char,
) -> *mut c_char {
    let pt_str = unsafe { CStr::from_ptr(provider_type) }
        .to_str()
        .unwrap_or("custom");
    let key = unsafe { CStr::from_ptr(api_key) }.to_str().unwrap_or("");
    let mdl = unsafe {
        if model.is_null() {
            None
        } else {
            CStr::from_ptr(model).to_str().ok()
        }
    };
    let url = unsafe {
        if base_url.is_null() {
            None
        } else {
            CStr::from_ptr(base_url).to_str().ok()
        }
    };

    let provider_type = match pt_str {
        "openai" => npc_types::LLMProviderType::OpenAI,
        "anthropic" => npc_types::LLMProviderType::Anthropic,
        "gemini" => npc_types::LLMProviderType::Gemini,
        "groq" => npc_types::LLMProviderType::Groq,
        "deepseek" => npc_types::LLMProviderType::DeepSeek,
        "mistral" => npc_types::LLMProviderType::Mistral,
        "together" => npc_types::LLMProviderType::Together,
        "venice" => npc_types::LLMProviderType::Venice,
        "fireworks" => npc_types::LLMProviderType::Fireworks,
        "openrouter" => npc_types::LLMProviderType::OpenRouter,
        "xai" => npc_types::LLMProviderType::XAi,
        "perplexity" => npc_types::LLMProviderType::Perplexity,
        "ollama" => npc_types::LLMProviderType::Ollama,
        "lmstudio" => npc_types::LLMProviderType::LMStudio,
        _ => npc_types::LLMProviderType::Custom,
    };

    with_state(|c, rt| rt.block_on(c.register_provider_simple(provider_type, key, mdl, url)))
}

#[no_mangle]
pub unsafe extern "C" fn blaniel_unregister_llm_provider(name: *const c_char) -> *mut c_char {
    let n = match parse_cstr(name, "provider name") {
        Ok(s) => s,
        Err(e) => return err_json(&e),
    };
    with_state(|c, rt| rt.block_on(c.unregister_llm_provider(&n)))
}

#[no_mangle]
pub unsafe extern "C" fn blaniel_get_llm_provider_status(name: *const c_char) -> *mut c_char {
    let n = match parse_cstr(name, "provider name") {
        Ok(s) => s,
        Err(e) => return err_json(&e),
    };
    with_state(|c, rt| rt.block_on(c.get_llm_provider_status(&n)))
}

#[no_mangle]
pub unsafe extern "C" fn blaniel_ambient_dialogue(
    participant_ids_json: *const c_char,
    context: *const c_char,
) -> *mut c_char {
    let json_str = match parse_cstr(participant_ids_json, "IDs") {
        Ok(s) => s,
        Err(e) => return err_json(&e),
    };
    let ids: Vec<String> = match serde_json::from_str(&json_str) {
        Ok(v) => v,
        Err(e) => return err_json(&format!("Invalid IDs JSON: {}", e)),
    };
    let ctx = unsafe { CStr::from_ptr(context) }
        .to_str()
        .unwrap_or("general")
        .to_string();
    with_state(|c, rt| rt.block_on(c.ambient_dialogue(&ids, &ctx, None)))
}

#[no_mangle]
pub unsafe extern "C" fn blaniel_get_action_catalog() -> *mut c_char {
    with_state(|c, rt| rt.block_on(c.get_game_catalog()))
}
