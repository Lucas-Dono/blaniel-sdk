using System;
using System.Collections.Generic;
using System.Net.Http;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Threading.Tasks;

namespace Blaniel.NPC
{
    public class BlanielClient : IDisposable
    {
        private readonly HttpClient _http;
        private readonly string _baseUrl;

        public BlanielClient(string apiUrl, string apiKey, int timeoutMs = 30000)
        {
            _baseUrl = apiUrl.TrimEnd('/');
            _http = new HttpClient { Timeout = TimeSpan.FromMilliseconds(timeoutMs) };
            _http.DefaultRequestHeaders.Add("Authorization", $"Bearer {apiKey}");
            _http.DefaultRequestHeaders.Add("Content-Type", "application/json");
        }

        public void Dispose() => _http.Dispose();

        private async Task<T> GetAsync<T>(string path)
        {
            var resp = await _http.GetAsync($"{_baseUrl}{path}");
            resp.EnsureSuccessStatusCode();
            return await resp.Content.ReadFromJsonAsync<T>();
        }

        private async Task<T> PostAsync<T>(string path, object body)
        {
            var json = JsonSerializer.Serialize(body);
            var content = new StringContent(json, Encoding.UTF8, "application/json");
            var resp = await _http.PostAsync($"{_baseUrl}{path}", content);
            resp.EnsureSuccessStatusCode();
            return await resp.Content.ReadFromJsonAsync<T>();
        }

        public Task<HealthResponse> Health() =>
            GetAsync<HealthResponse>("/health");

        public Task<NpcState> GetNpc(string id) =>
            GetAsync<NpcState>($"/api/v1/npc/{id}");

        public Task<NearbyNpcsResponse> GetNearbyNpcs(double x, double y, double z, string world, double radius) =>
            GetAsync<NearbyNpcsResponse>($"/api/v1/npc/nearby?x={x}&y={y}&z={z}&world={world}&radius={radius}");

        public Task<ChatResponse> Chat(string npcId, string message) =>
            PostAsync<ChatResponse>($"/api/v1/npc/{npcId}/chat", new { message, context = (object)null });

        public Task<NpcState> MoveNpc(string npcId, double x, double y, double z, string world) =>
            PostAsync<NpcState>($"/api/v1/npc/{npcId}/move", new
            {
                position = new { x, y, z, world },
                action = (string)null,
                facing_direction = (double?)null
            });

        public Task<PathResult> Pathfind(string npcId, Position from, Position to) =>
            PostAsync<PathResult>($"/api/v1/npc/{npcId}/pathfind", new
            {
                start = from,
                goal = to,
                options = new { avoid_water = true, avoid_lava = true, max_fall_distance = 3, allow_diagonal = true }
            });

        public Task<NavigationResponse> NavigateTo(string npcId, string name) =>
            PostAsync<NavigationResponse>($"/api/v1/npc/{npcId}/navigate", new
            {
                target = new { type = "word", name },
                speed = (double?)null,
                callback_url = (string)null
            });

        public Task<NavigationResponse> NavigateToCoords(string npcId, double x, double y, double z, string world) =>
            PostAsync<NavigationResponse>($"/api/v1/npc/{npcId}/navigate", new
            {
                target = new { type = "coordinate", position = new { x, y, z, world } },
                speed = (double?)null,
                callback_url = (string)null
            });

        public Task<NavigationConfig> GetNavigationConfig(string npcId) =>
            GetAsync<NavigationConfig>($"/api/v1/npc/{npcId}/navigation-config");

        public Task<NavigationConfig> SetNavigationConfig(string npcId, NavigationConfig config) =>
            PostAsync<NavigationConfig>($"/api/v1/npc/{npcId}/navigation-config", config);

        public Task<GameCatalogResponse> GetGameCatalog() =>
            GetAsync<GameCatalogResponse>("/api/v1/actions/catalog");

        public Task<JsonElement> GetActionConfig(string npcId) =>
            GetAsync<JsonElement>($"/api/v1/npc/{npcId}/action-config");

        public Task<JsonElement> SetActionConfig(string npcId, object config) =>
            PostAsync<JsonElement>($"/api/v1/npc/{npcId}/action-config", config);

        public Task<JsonElement> GetAiContext(string npcId) =>
            GetAsync<JsonElement>($"/api/v1/npc/{npcId}/ai-context");

        public Task<LLMProvidersResponse> GetLlmPresets() =>
            GetAsync<LLMProvidersResponse>("/api/v1/llm/presets");

        public Task<List<LLMProviderStatus>> ListLlmProviders() =>
            GetAsync<List<LLMProviderStatus>>("/api/v1/llm/providers");

        public Task<LLMProviderStatus> RegisterLlmProvider(string providerType, string apiKey, string model = null, string baseUrl = null) =>
            PostAsync<LLMProviderStatus>("/api/v1/llm/providers", new
            {
                provider_type = providerType,
                api_key = apiKey,
                model,
                base_url = baseUrl,
                max_tokens = 150,
                temperature = 0.8,
                enabled = true
            });

        public Task<AmbientDialoguesResponse> AmbientDialogue(string[] participantIds, string context) =>
            PostAsync<AmbientDialoguesResponse>("/api/v1/dialogue/ambient", new
            {
                participant_ids = participantIds,
                context,
                max_exchanges = (int?)null
            });

        public Task<JsonElement> RegisterScene(string sceneId, string world, ScenePosition[] positions) =>
            PostAsync<JsonElement>("/api/v1/navigation/scene/register", new
            {
                scene_id = sceneId,
                world,
                positions
            });
    }

    #region Models

    public class Position
    {
        [JsonPropertyName("x")] public double X { get; set; }
        [JsonPropertyName("y")] public double Y { get; set; }
        [JsonPropertyName("z")] public double Z { get; set; }
        [JsonPropertyName("world")] public string World { get; set; } = "overworld";

        public Position(double x, double y, double z, string world = "overworld")
        {
            X = x; Y = y; Z = z; World = world;
        }
    }

    public class NpcState
    {
        [JsonPropertyName("id")] public string Id { get; set; }
        [JsonPropertyName("name")] public string Name { get; set; }
        [JsonPropertyName("position")] public Position Position { get; set; }
        [JsonPropertyName("current_action")] public string CurrentAction { get; set; }
        [JsonPropertyName("animation")] public string Animation { get; set; }
        [JsonPropertyName("facing_direction")] public string FacingDirection { get; set; }
        [JsonPropertyName("cached_emotion")] public string CachedEmotion { get; set; }
        [JsonPropertyName("cached_at")] public long CachedAt { get; set; }
    }

    public class ChatResponse
    {
        [JsonPropertyName("response")] public string Response { get; set; }
        [JsonPropertyName("emotion")] public string Emotion { get; set; }
        [JsonPropertyName("animation")] public string Animation { get; set; }
        [JsonPropertyName("source")] public string Source { get; set; }
        [JsonPropertyName("latency_ms")] public long LatencyMs { get; set; }
        [JsonPropertyName("cached")] public bool? Cached { get; set; }
    }

    public class PathResult
    {
        [JsonPropertyName("path")] public List<Position> Path { get; set; }
        [JsonPropertyName("cost")] public double Cost { get; set; }
        [JsonPropertyName("computed_in_ms")] public long ComputedInMs { get; set; }
    }

    public class NavigationResponse
    {
        [JsonPropertyName("success")] public bool Success { get; set; }
        [JsonPropertyName("target_name")] public string TargetName { get; set; }
        [JsonPropertyName("distance")] public double? Distance { get; set; }
        [JsonPropertyName("estimated_time_ms")] public long? EstimatedTimeMs { get; set; }
        [JsonPropertyName("navigation_mode")] public string NavigationMode { get; set; }
    }

    public class NavigationConfig
    {
        [JsonPropertyName("mode")] public string Mode { get; set; } = "hybrid";
        [JsonPropertyName("scene_id")] public string SceneId { get; set; }
        [JsonPropertyName("default_speed")] public double DefaultSpeed { get; set; } = 4.0;
        [JsonPropertyName("introduction")] public string Introduction { get; set; }
        [JsonPropertyName("explain")] public string Explain { get; set; }
    }

    public class NearbyNpcsResponse
    {
        [JsonPropertyName("npcs")] public List<NpcState> Npcs { get; set; }
        [JsonPropertyName("total")] public int Total { get; set; }
    }

    public class HealthResponse
    {
        [JsonPropertyName("status")] public string Status { get; set; }
        [JsonPropertyName("version")] public string Version { get; set; }
        [JsonPropertyName("timestamp")] public long Timestamp { get; set; }
    }

    public class GameCatalogResponse
    {
        [JsonPropertyName("genres")] public List<GameGenreInfo> Genres { get; set; }
        [JsonPropertyName("total")] public int Total { get; set; }
    }

    public class GameGenreInfo
    {
        [JsonPropertyName("genre")] public string Genre { get; set; }
        [JsonPropertyName("default_actions")] public List<string> DefaultActions { get; set; }
        [JsonPropertyName("description")] public string Description { get; set; }
    }

    public class LLMProvidersResponse
    {
        [JsonPropertyName("presets")] public List<LLMProviderPreset> Presets { get; set; }
        [JsonPropertyName("total")] public int Total { get; set; }
    }

    public class LLMProviderPreset
    {
        [JsonPropertyName("name")] public string Name { get; set; }
        [JsonPropertyName("label")] public string Label { get; set; }
        [JsonPropertyName("base_url")] public string BaseUrl { get; set; }
        [JsonPropertyName("default_model")] public string DefaultModel { get; set; }
        [JsonPropertyName("description")] public string Description { get; set; }
        [JsonPropertyName("supports_streaming")] public bool SupportsStreaming { get; set; }
    }

    public class LLMProviderStatus
    {
        [JsonPropertyName("name")] public string Name { get; set; }
        [JsonPropertyName("model")] public string Model { get; set; }
        [JsonPropertyName("enabled")] public bool Enabled { get; set; }
        [JsonPropertyName("healthy")] public bool Healthy { get; set; }
        [JsonPropertyName("total_requests")] public long TotalRequests { get; set; }
        [JsonPropertyName("total_errors")] public long TotalErrors { get; set; }
        [JsonPropertyName("avg_latency_ms")] public double AvgLatencyMs { get; set; }
        [JsonPropertyName("priority")] public int Priority { get; set; }
    }

    public class AmbientDialoguesResponse
    {
        [JsonPropertyName("dialogues")] public List<AmbientDialogue> Dialogues { get; set; }
        [JsonPropertyName("cached")] public bool Cached { get; set; }
    }

    public class AmbientDialogue
    {
        [JsonPropertyName("speaker_id")] public string SpeakerId { get; set; }
        [JsonPropertyName("message")] public string Message { get; set; }
        [JsonPropertyName("emotion")] public string Emotion { get; set; }
        [JsonPropertyName("animation")] public string Animation { get; set; }
    }

    public class ScenePosition
    {
        [JsonPropertyName("name")] public string Name { get; set; }
        [JsonPropertyName("position")] public Position Position { get; set; }
        [JsonPropertyName("aliases")] public List<string> Aliases { get; set; } = new();
        [JsonPropertyName("tags")] public List<string> Tags { get; set; } = new();
    }

    #endregion
}
