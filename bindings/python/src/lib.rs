use blaniel_sdk::{BlanielClient, BlanielConfig};
use pyo3::prelude::*;
use std::sync::Arc;
use tokio::runtime::Runtime;

struct ClientState {
    client: BlanielClient,
    rt: Runtime,
}

#[pyclass]
struct Blaniel {
    state: Arc<ClientState>,
}

fn run<F, T>(state: &Arc<ClientState>, f: F) -> PyResult<String>
where
    F: FnOnce(&BlanielClient) -> Result<T, blaniel_sdk::SdkError>,
    T: serde::Serialize,
{
    match f(&state.client) {
        Ok(val) => serde_json::to_string(&val)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string())),
        Err(e) => Err(pyo3::exceptions::PyRuntimeError::new_err(e.to_string())),
    }
}

fn to_py(result: PyResult<String>, py: Python) -> PyResult<PyObject> {
    let json_str = result?;
    let json_mod = py.import("json")?;
    let parsed = json_mod.call_method1("loads", (json_str,))?;
    Ok(parsed.into())
}

#[pymethods]
impl Blaniel {
    #[new]
    #[pyo3(signature = (api_url, api_key, timeout_ms=30000))]
    fn new(api_url: &str, api_key: &str, timeout_ms: u64) -> PyResult<Self> {
        let config = BlanielConfig::new(api_url, api_key).with_timeout(timeout_ms);
        let client = BlanielClient::new(config)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        let rt =
            Runtime::new().map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(Self {
            state: Arc::new(ClientState { client, rt }),
        })
    }

    fn health(&self, py: Python) -> PyResult<PyObject> {
        let state = self.state.clone();
        let result = py.allow_threads(|| run(&state, |c| state.rt.block_on(c.health())));
        to_py(result, py)
    }

    fn get_npc(&self, npc_id: &str, py: Python) -> PyResult<PyObject> {
        let state = self.state.clone();
        let id = npc_id.to_string();
        let result = py.allow_threads(|| run(&state, |c| state.rt.block_on(c.get_npc(&id))));
        to_py(result, py)
    }

    fn chat(&self, npc_id: &str, message: &str, py: Python) -> PyResult<PyObject> {
        let state = self.state.clone();
        let id = npc_id.to_string();
        let msg = message.to_string();
        let result =
            py.allow_threads(|| run(&state, |c| state.rt.block_on(c.chat_simple(&id, &msg))));
        to_py(result, py)
    }

    fn move_npc(
        &self,
        npc_id: &str,
        x: f64,
        y: f64,
        z: f64,
        world: &str,
        py: Python,
    ) -> PyResult<PyObject> {
        let state = self.state.clone();
        let id = npc_id.to_string();
        let pos = npc_types::Position3D::new(x, y, z, world.to_string());
        let result = py.allow_threads(|| {
            run(&state, |c| {
                state.rt.block_on(c.move_npc(&id, pos, None, None))
            })
        });
        to_py(result, py)
    }

    fn navigate_to(&self, npc_id: &str, name: &str, py: Python) -> PyResult<PyObject> {
        let state = self.state.clone();
        let id = npc_id.to_string();
        let target = npc_types::NavigationTarget::Word {
            name: name.to_string(),
        };
        let result =
            py.allow_threads(|| run(&state, |c| state.rt.block_on(c.navigate(&id, target, None))));
        to_py(result, py)
    }

    fn navigate_to_coords(
        &self,
        npc_id: &str,
        x: f64,
        y: f64,
        z: f64,
        world: &str,
        py: Python,
    ) -> PyResult<PyObject> {
        let state = self.state.clone();
        let id = npc_id.to_string();
        let target = npc_types::NavigationTarget::Coordinate {
            position: npc_types::Position3D::new(x, y, z, world.to_string()),
        };
        let result =
            py.allow_threads(|| run(&state, |c| state.rt.block_on(c.navigate(&id, target, None))));
        to_py(result, py)
    }

    fn pathfind(
        &self,
        npc_id: &str,
        fx: f64,
        fy: f64,
        fz: f64,
        tx: f64,
        ty: f64,
        tz: f64,
        world: &str,
        py: Python,
    ) -> PyResult<PyObject> {
        let state = self.state.clone();
        let id = npc_id.to_string();
        let start = npc_types::Position3D::new(fx, fy, fz, world.to_string());
        let goal = npc_types::Position3D::new(tx, ty, tz, world.to_string());
        let result = py.allow_threads(|| {
            run(&state, |c| {
                state.rt.block_on(c.pathfind(&id, start, goal, None))
            })
        });
        to_py(result, py)
    }

    fn get_nearby_npcs(
        &self,
        x: f64,
        y: f64,
        z: f64,
        radius: f64,
        world: &str,
        py: Python,
    ) -> PyResult<PyObject> {
        let state = self.state.clone();
        let w = world.to_string();
        let result = py.allow_threads(|| {
            run(&state, |c| {
                state.rt.block_on(c.get_nearby_npcs(x, y, z, &w, radius))
            })
        });
        to_py(result, py)
    }

    fn get_navigation_config(&self, npc_id: &str, py: Python) -> PyResult<PyObject> {
        let state = self.state.clone();
        let id = npc_id.to_string();
        let result =
            py.allow_threads(|| run(&state, |c| state.rt.block_on(c.get_navigation_config(&id))));
        to_py(result, py)
    }

    fn set_navigation_config(
        &self,
        npc_id: &str,
        config_json: &str,
        py: Python,
    ) -> PyResult<PyObject> {
        let state = self.state.clone();
        let id = npc_id.to_string();
        let config: npc_types::NavigationConfig =
            serde_json::from_str(config_json).map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!("Invalid config JSON: {}", e))
            })?;
        let result = py.allow_threads(|| {
            run(&state, |c| {
                state.rt.block_on(c.set_navigation_config(&id, &config))
            })
        });
        to_py(result, py)
    }

    fn get_action_config(&self, npc_id: &str, py: Python) -> PyResult<PyObject> {
        let state = self.state.clone();
        let id = npc_id.to_string();
        let result =
            py.allow_threads(|| run(&state, |c| state.rt.block_on(c.get_action_config(&id))));
        to_py(result, py)
    }

    fn set_action_config(&self, npc_id: &str, config_json: &str, py: Python) -> PyResult<PyObject> {
        let state = self.state.clone();
        let id = npc_id.to_string();
        let req: npc_types::SetActionSystemConfigRequest = serde_json::from_str(config_json)
            .map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!("Invalid config JSON: {}", e))
            })?;
        let result = py.allow_threads(|| {
            run(&state, |c| {
                state.rt.block_on(c.set_action_config(&id, &req))
            })
        });
        to_py(result, py)
    }

    fn list_llm_providers(&self, py: Python) -> PyResult<PyObject> {
        let state = self.state.clone();
        let result =
            py.allow_threads(|| run(&state, |c| state.rt.block_on(c.list_llm_providers())));
        to_py(result, py)
    }

    #[pyo3(signature = (provider_type, api_key, model=None, base_url=None))]
    fn register_llm_provider(
        &self,
        provider_type: &str,
        api_key: &str,
        model: Option<&str>,
        base_url: Option<&str>,
        py: Python,
    ) -> PyResult<PyObject> {
        let state = self.state.clone();
        let pt = parse_provider_type(provider_type)?;
        let key = api_key.to_string();
        let mdl = model.map(|s| s.to_string());
        let url = base_url.map(|s| s.to_string());
        let result = py.allow_threads(|| {
            run(&state, |c| {
                state.rt.block_on(c.register_provider_simple(
                    pt,
                    &key,
                    mdl.as_deref(),
                    url.as_deref(),
                ))
            })
        });
        to_py(result, py)
    }

    fn ambient_dialogue(
        &self,
        participant_ids: Vec<String>,
        context: &str,
        py: Python,
    ) -> PyResult<PyObject> {
        let state = self.state.clone();
        let ctx = context.to_string();
        let result = py.allow_threads(|| {
            run(&state, |c| {
                state
                    .rt
                    .block_on(c.ambient_dialogue(&participant_ids, &ctx, None))
            })
        });
        to_py(result, py)
    }

    fn register_scene(&self, scene_json: &str, py: Python) -> PyResult<PyObject> {
        let state = self.state.clone();
        let req: npc_types::RegisterSceneRequest =
            serde_json::from_str(scene_json).map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!("Invalid scene JSON: {}", e))
            })?;
        let scene_id = req.scene_id.clone();
        let world = req.world.clone();
        let positions = req.positions;
        let result = py.allow_threads(|| {
            run(&state, |c| {
                state
                    .rt
                    .block_on(c.register_scene(&scene_id, &world, positions))
            })
        });
        to_py(result, py)
    }
}

fn parse_provider_type(s: &str) -> PyResult<npc_types::LLMProviderType> {
    match s.to_lowercase().as_str() {
        "openai" => Ok(npc_types::LLMProviderType::OpenAI),
        "anthropic" | "claude" => Ok(npc_types::LLMProviderType::Anthropic),
        "gemini" | "google" => Ok(npc_types::LLMProviderType::Gemini),
        "groq" => Ok(npc_types::LLMProviderType::Groq),
        "deepseek" => Ok(npc_types::LLMProviderType::DeepSeek),
        "mistral" => Ok(npc_types::LLMProviderType::Mistral),
        "together" => Ok(npc_types::LLMProviderType::Together),
        "venice" => Ok(npc_types::LLMProviderType::Venice),
        "fireworks" => Ok(npc_types::LLMProviderType::Fireworks),
        "openrouter" => Ok(npc_types::LLMProviderType::OpenRouter),
        "xai" | "grok" => Ok(npc_types::LLMProviderType::XAi),
        "perplexity" => Ok(npc_types::LLMProviderType::Perplexity),
        "ollama" => Ok(npc_types::LLMProviderType::Ollama),
        "lmstudio" => Ok(npc_types::LLMProviderType::LMStudio),
        _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Unknown provider: {}",
            s
        ))),
    }
}

#[pymodule]
fn blaniel(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Blaniel>()?;
    Ok(())
}
