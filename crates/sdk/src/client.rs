use crate::{BlanielConfig, SdkError, SdkResult};
use npc_types::*;
use reqwest::{Client, Method, RequestBuilder};
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;

#[derive(Debug, Clone)]
pub struct BlanielClient {
    config: BlanielConfig,
    http: Client,
}

impl BlanielClient {
    pub fn new(config: BlanielConfig) -> SdkResult<Self> {
        let http = Client::builder()
            .timeout(std::time::Duration::from_millis(config.timeout_ms))
            .build()
            .map_err(SdkError::Http)?;

        Ok(Self { config, http })
    }

    pub fn from_env() -> SdkResult<Self> {
        let config = BlanielConfig::load().ok_or_else(|| {
            SdkError::Config(
                "No configuration found. Run `blaniel init` or create ~/.blaniel/config.json"
                    .to_string(),
            )
        })?;
        Self::new(config)
    }

    pub fn connect(api_url: impl Into<String>, api_key: impl Into<String>) -> SdkResult<Self> {
        Self::new(BlanielConfig::new(api_url, api_key))
    }

    pub fn config(&self) -> &BlanielConfig {
        &self.config
    }

    async fn request<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
    ) -> SdkResult<T> {
        let builder = self.build_request(method, path)?;
        self.execute_request(builder).await
    }

    async fn request_with_body<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        method: Method,
        path: &str,
        body: &B,
    ) -> SdkResult<T> {
        let builder = self
            .build_request(method, path)?
            .json(body);
        self.execute_request(builder).await
    }

    fn build_request(&self, method: Method, path: &str) -> SdkResult<RequestBuilder> {
        let url = format!("{}{}", self.config.api_url.trim_end_matches('/'), path);
        Ok(self
            .http
            .request(method, &url)
            .bearer_auth(&self.config.api_key)
            .header("Content-Type", "application/json"))
    }

    async fn execute_request<T: DeserializeOwned>(
        &self,
        builder: RequestBuilder,
    ) -> SdkResult<T> {
        let resp = builder.send().await?;

        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(SdkError::Auth("Invalid API key".to_string()));
        }
        if status == reqwest::StatusCode::NOT_FOUND {
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            let msg = body["error"].as_str().unwrap_or("Not found");
            return Err(SdkError::NotFound(msg.to_string()));
        }
        if status.is_client_error() || status.is_server_error() {
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            let msg = body["error"].as_str().unwrap_or("Unknown error");
            let code = body["code"].as_str().unwrap_or("UNKNOWN");
            return Err(SdkError::api_error(msg, code));
        }

        resp.json::<T>().await.map_err(SdkError::Http)
    }

    // ─── Health ───────────────────────────────────────────────

    pub async fn health(&self) -> SdkResult<HealthResponse> {
        self.request(Method::GET, "/health").await
    }

    // ─── NPC State ────────────────────────────────────────────

    pub async fn get_npc(&self, npc_id: &str) -> SdkResult<NpcState> {
        self.request(Method::GET, &format!("/api/v1/npc/{}", npc_id))
            .await
    }

    pub async fn get_nearby_npcs(
        &self,
        x: f64,
        y: f64,
        z: f64,
        world: &str,
        radius: f64,
    ) -> SdkResult<NearbyNpcsResponse> {
        let path = format!(
            "/api/v1/npc/nearby?x={}&y={}&z={}&world={}&radius={}",
            x, y, z, world, radius
        );
        self.request(Method::GET, &path).await
    }

    pub async fn batch_get_states(
        &self,
        agent_ids: &[String],
    ) -> SdkResult<BatchStateResponse> {
        let req = BatchStateRequest {
            agent_ids: agent_ids.to_vec(),
        };
        self.request_with_body(Method::POST, "/api/v1/npc/batch-state", &req)
            .await
    }

    pub async fn move_npc(
        &self,
        npc_id: &str,
        position: Position3D,
        action: Option<NpcAction>,
        facing: Option<f64>,
    ) -> SdkResult<NpcState> {
        let req = MoveRequest {
            position,
            action,
            facing_direction: facing,
        };
        self.request_with_body(Method::POST, &format!("/api/v1/npc/{}/move", npc_id), &req)
            .await
    }

    // ─── Chat ─────────────────────────────────────────────────

    pub async fn chat(
        &self,
        npc_id: &str,
        message: &str,
        context: Option<ChatContext>,
    ) -> SdkResult<ChatResponse> {
        let req = ChatRequest {
            message: message.to_string(),
            context,
        };
        self.request_with_body(Method::POST, &format!("/api/v1/npc/{}/chat", npc_id), &req)
            .await
    }

    pub async fn chat_simple(&self, npc_id: &str, message: &str) -> SdkResult<ChatResponse> {
        self.chat(npc_id, message, None).await
    }

    // ─── Pathfinding ──────────────────────────────────────────

    pub async fn pathfind(
        &self,
        npc_id: &str,
        start: Position3D,
        goal: Position3D,
        options: Option<PathfindOptions>,
    ) -> SdkResult<PathResult> {
        let req = PathfindRequest {
            start,
            goal,
            options: options.unwrap_or_default(),
        };
        self.request_with_body(Method::POST, &format!("/api/v1/npc/{}/pathfind", npc_id), &req)
            .await
    }

    // ─── Navigation ───────────────────────────────────────────

    pub async fn navigate(
        &self,
        npc_id: &str,
        target: NavigationTarget,
        speed: Option<f64>,
    ) -> SdkResult<NavigationResponse> {
        let req = NavigationRequest {
            target,
            speed,
            callback_url: None,
        };
        self.request_with_body(Method::POST, &format!("/api/v1/npc/{}/navigate", npc_id), &req)
            .await
    }

    pub async fn navigate_to_word(
        &self,
        npc_id: &str,
        location_name: &str,
    ) -> SdkResult<NavigationResponse> {
        self.navigate(npc_id, NavigationTarget::Word { name: location_name.to_string() }, None)
            .await
    }

    pub async fn navigate_to_coords(
        &self,
        npc_id: &str,
        position: Position3D,
    ) -> SdkResult<NavigationResponse> {
        self.navigate(npc_id, NavigationTarget::Coordinate { position }, None)
            .await
    }

    pub async fn get_navigation_config(
        &self,
        npc_id: &str,
    ) -> SdkResult<NavigationConfig> {
        self.request(Method::GET, &format!("/api/v1/npc/{}/navigation-config", npc_id))
            .await
    }

    pub async fn set_navigation_config(
        &self,
        npc_id: &str,
        config: &NavigationConfig,
    ) -> SdkResult<NavigationConfig> {
        self.request_with_body(
            Method::POST,
            &format!("/api/v1/npc/{}/navigation-config", npc_id),
            config,
        )
        .await
    }

    // ─── Scenes ───────────────────────────────────────────────

    pub async fn register_scene(
        &self,
        scene_id: &str,
        world: &str,
        positions: Vec<ScenePosition>,
    ) -> SdkResult<serde_json::Value> {
        let req = RegisterSceneRequest {
            scene_id: scene_id.to_string(),
            world: world.to_string(),
            positions,
        };
        self.request_with_body(Method::POST, "/api/v1/navigation/scene/register", &req)
            .await
    }

    pub async fn get_scene_positions(
        &self,
        scene_id: &str,
    ) -> SdkResult<ScenePositionsResponse> {
        self.request(Method::GET, &format!("/api/v1/navigation/scene/{}/positions", scene_id))
            .await
    }

    // ─── Action System ────────────────────────────────────────

    pub async fn get_game_catalog(&self) -> SdkResult<GameCatalogResponse> {
        self.request(Method::GET, "/api/v1/actions/catalog").await
    }

    pub async fn get_genre_actions(&self, genre: &str) -> SdkResult<serde_json::Value> {
        self.request(Method::GET, &format!("/api/v1/actions/catalog/{}", genre))
            .await
    }

    pub async fn get_action_config(
        &self,
        npc_id: &str,
    ) -> SdkResult<ActionSystemConfig> {
        self.request(Method::GET, &format!("/api/v1/npc/{}/action-config", npc_id))
            .await
    }

    pub async fn set_action_config(
        &self,
        npc_id: &str,
        config: &SetActionSystemConfigRequest,
    ) -> SdkResult<ActionSystemConfig> {
        self.request_with_body(
            Method::POST,
            &format!("/api/v1/npc/{}/action-config", npc_id),
            config,
        )
        .await
    }

    pub async fn get_ai_context(&self, npc_id: &str) -> SdkResult<serde_json::Value> {
        self.request(Method::GET, &format!("/api/v1/npc/{}/ai-context", npc_id))
            .await
    }

    pub async fn create_action_plan(
        &self,
        npc_id: &str,
        plan: ActionPlan,
    ) -> SdkResult<ActionPlanResponse> {
        let req = ActionPlanRequest {
            plan,
            navigation_config: None,
        };
        self.request_with_body(Method::POST, &format!("/api/v1/npc/{}/action-plan", npc_id), &req)
            .await
    }

    // ─── LLM Providers ────────────────────────────────────────

    pub async fn get_llm_presets(&self) -> SdkResult<LLMProvidersResponse> {
        self.request(Method::GET, "/api/v1/llm/presets").await
    }

    pub async fn list_llm_providers(&self) -> SdkResult<Vec<LLMProviderStatus>> {
        self.request(Method::GET, "/api/v1/llm/providers").await
    }

    pub async fn register_llm_provider(
        &self,
        req: &SetLLMProviderRequest,
    ) -> SdkResult<LLMProviderStatus> {
        self.request_with_body(Method::POST, "/api/v1/llm/providers", req)
            .await
    }

    pub async fn get_llm_provider_status(
        &self,
        name: &str,
    ) -> SdkResult<LLMProviderStatus> {
        self.request(Method::GET, &format!("/api/v1/llm/providers/{}", name))
            .await
    }

    pub async fn unregister_llm_provider(&self, name: &str) -> SdkResult<serde_json::Value> {
        self.request(Method::POST, &format!("/api/v1/llm/providers/{}", name))
            .await
    }

    pub async fn register_provider_simple(
        &self,
        provider_type: LLMProviderType,
        api_key: &str,
        model: Option<&str>,
        base_url: Option<&str>,
    ) -> SdkResult<LLMProviderStatus> {
        let resolved_base_url = base_url
            .map(|s| s.to_string())
            .or_else(|| provider_type.default_base_url().map(|s| s.to_string()))
            .unwrap_or_default();

        let resolved_model = model
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                LLMProviderType::all_presets()
                    .iter()
                    .find(|p| p.provider_type == provider_type)
                    .map(|p| p.default_model.clone())
                    .unwrap_or_else(|| "default".to_string())
            });

        let req = SetLLMProviderRequest {
            provider_type,
            base_url: if base_url.is_none() { None } else { Some(resolved_base_url) },
            api_key: secrecy::Secret::new(api_key.to_string()),
            model: Some(resolved_model),
            max_tokens: 150,
            temperature: 0.8,
            system_prompt: None,
            extra_headers: std::collections::HashMap::new(),
            enabled: true,
            priority: 0,
            timeout_ms: 30000,
        };
        self.register_llm_provider(&req).await
    }

    // ─── LLM Metrics ──────────────────────────────────────────

    pub async fn get_llm_metrics_global(&self) -> SdkResult<serde_json::Value> {
        self.request(Method::GET, "/api/v1/llm/metrics/global").await
    }

    pub async fn get_llm_metrics_by_agent(
        &self,
        agent_id: &str,
    ) -> SdkResult<serde_json::Value> {
        self.request(
            Method::GET,
            &format!("/api/v1/llm/metrics/agent/{}", agent_id),
        )
        .await
    }

    pub async fn get_llm_metrics_by_user(
        &self,
        user_id: &str,
    ) -> SdkResult<serde_json::Value> {
        self.request(Method::GET, &format!("/api/v1/llm/metrics/user/{}", user_id))
            .await
    }

    pub async fn get_llm_metrics_top_agents(&self) -> SdkResult<serde_json::Value> {
        self.request(Method::GET, "/api/v1/llm/metrics/top-agents")
            .await
    }

    pub async fn get_llm_metrics_recent(&self) -> SdkResult<serde_json::Value> {
        self.request(Method::GET, "/api/v1/llm/metrics/recent").await
    }

    // ─── Ambient Dialogue ─────────────────────────────────────

    pub async fn ambient_dialogue(
        &self,
        participant_ids: &[String],
        context: &str,
        max_exchanges: Option<usize>,
    ) -> SdkResult<AmbientDialoguesResponse> {
        let req = AmbientDialogueQuery {
            participant_ids: participant_ids.to_vec(),
            context: context.to_string(),
            max_exchanges,
        };
        self.request_with_body(Method::POST, "/api/v1/dialogue/ambient", &req)
            .await
    }

    // ─── Convenience: Quick NPC Setup ─────────────────────────

    pub async fn setup_npc(
        &self,
        npc_id: &str,
        game_genre: GameGenre,
        nav_mode: NavigationMode,
        scene_id: Option<&str>,
        scene_positions: Vec<ScenePosition>,
    ) -> SdkResult<NpcSetupResult> {
        let mut steps: Vec<String> = Vec::new();

        if let Some(sid) = scene_id {
            if !scene_positions.is_empty() {
                match self
                    .register_scene(sid, "overworld", scene_positions)
                    .await
                {
                    Ok(_) => steps.push("scene_registered".to_string()),
                    Err(e) => steps.push(format!("scene_failed: {}", e)),
                }
            }
        }

        let nav_config = NavigationConfig {
            mode: nav_mode,
            restrictions: None,
            scene_id: scene_id.map(|s| s.to_string()),
            default_speed: 4.0,
            introduction: String::new(),
            explain: String::new(),
        };
        match self.set_navigation_config(npc_id, &nav_config).await {
            Ok(_) => steps.push("navigation_configured".to_string()),
            Err(e) => steps.push(format!("navigation_failed: {}", e)),
        }

        let default_actions = game_genre.default_actions();
        let action_req = SetActionSystemConfigRequest {
            game_genre: game_genre.clone(),
            enabled_actions: default_actions,
            custom_actions: vec![],
            introduction: format!("You are an NPC in a {} game.", game_genre.as_str()),
            action_explain: "Use your available actions to interact with the world.".to_string(),
            movement_explain: format!(
                "You can move using {} navigation.",
                nav_mode.as_str()
            ),
        };
        match self.set_action_config(npc_id, &action_req).await {
            Ok(_) => steps.push("actions_configured".to_string()),
            Err(e) => steps.push(format!("actions_failed: {}", e)),
        }

        Ok(NpcSetupResult {
            npc_id: npc_id.to_string(),
            steps,
            success: true,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcSetupResult {
    pub npc_id: String,
    pub steps: Vec<String>,
    pub success: bool,
}
