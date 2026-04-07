using System;
using System.Text;
using System.Collections.Generic;
using System.Threading.Tasks;
using UnityEngine;
using UnityEngine.Networking;

namespace Blaniel.NPC
{
    public class NpcApiClient : IDisposable
    {
        private readonly string _baseUrl;
        private readonly string _jwtToken;
        private readonly NpcClientCache _cache;
        private readonly int _timeoutSeconds;

        public NpcApiClient(string baseUrl, string jwtToken, int cacheSize = 100, int timeoutSeconds = 10)
        {
            _baseUrl = baseUrl.TrimEnd('/');
            _jwtToken = jwtToken;
            _cache = new NpcClientCache(cacheSize);
            _timeoutSeconds = timeoutSeconds;
        }

        public NpcClientCache Cache => _cache;

        public async Task<HealthResponse> GetHealthAsync()
        {
            return await GetAsync<HealthResponse>("/api/v1/health");
        }

        public async Task<NpcState> GetNpcStateAsync(string npcId)
        {
            string cacheKey = $"npc:state:{npcId}";
            if (_cache.TryGet(cacheKey, out NpcState cached))
            {
                return cached;
            }

            var state = await GetAsync<NpcState>($"/api/v1/npc/{npcId}");
            _cache.Set(cacheKey, state, 60);
            return state;
        }

        public async Task<NearbyNpcsResponse> GetNearbyNpcsAsync(float x, float y, float z, float radius, string world = "overworld", int limit = 50)
        {
            string url = $"/api/v1/npc/nearby?x={x}&y={y}&z={z}&radius={radius}&world={world}&limit={limit}";
            return await GetAsync<NearbyNpcsResponse>(url);
        }

        public async Task<BatchStateResponse> BatchGetStatesAsync(string[] agentIds)
        {
            var request = new BatchStateRequest { AgentIds = agentIds };
            return await PostAsync<BatchStateResponse>("/api/v1/npc/batch-state", request);
        }

        public async Task<bool> MoveNpcAsync(string npcId, Vector3 position, string world = "overworld", string action = null)
        {
            var request = new MoveRequest
            {
                Position = new Position3D { X = position.x, Y = position.y, Z = position.z, World = world },
                Action = action
            };

            try
            {
                await PostAsync<object>($"/api/v1/npc/{npcId}/move", request);
                _cache.Remove($"npc:state:{npcId}");
                return true;
            }
            catch
            {
                return false;
            }
        }

        public async Task<ChatResponse> ChatAsync(string npcId, string message, ChatContext context = null)
        {
            string cacheKey = $"chat:{npcId}:{message.ToLowerInvariant().GetHashCode()}";
            if (_cache.TryGet(cacheKey, out ChatResponse cached))
            {
                return cached;
            }

            var request = new ChatRequest
            {
                Message = message,
                Context = context
            };

            var response = await PostAsync<ChatResponse>($"/api/v1/npc/{npcId}/chat", request);

            if (response.Source == "rust_cache" || response.Source == "template")
            {
                _cache.Set(cacheKey, response, 300);
            }

            return response;
        }

        public async Task<PathResult> PathfindAsync(string npcId, Vector3 start, Vector3 goal, string world = "overworld", PathfindOptions options = null)
        {
            var request = new PathfindRequest
            {
                Start = new Position3D { X = start.x, Y = start.y, Z = start.z, World = world },
                Goal = new Position3D { X = goal.x, Y = goal.y, Z = goal.z, World = world },
                Options = options ?? new PathfindOptions()
            };

            return await PostAsync<PathResult>($"/api/v1/npc/{npcId}/pathfind", request);
        }

        public async Task<AmbientDialoguesResponse> GetAmbientDialogueAsync(string[] participantIds, string context)
        {
            var request = new AmbientDialogueQuery
            {
                ParticipantIds = participantIds,
                Context = context
            };

            return await PostAsync<AmbientDialoguesResponse>("/api/v1/dialogue/ambient", request);
        }

        public void ClearCache()
        {
            _cache.Clear();
        }

        private async Task<T> GetAsync<T>(string path)
        {
            using var request = CreateRequest(path, "GET");
            var handler = new DownloadHandlerBuffer();
            request.downloadHandler = handler;

            await SendRequestAsync(request);

            return JsonUtility.FromJson<T>(handler.text);
        }

        private async Task<T> PostAsync<T>(string path, object body)
        {
            string json = JsonUtility.ToJson(body);
            using var request = CreateRequest(path, "POST");
            request.uploadHandler = new UploadHandlerRaw(Encoding.UTF8.GetBytes(json));
            request.downloadHandler = new DownloadHandlerBuffer();
            request.SetRequestHeader("Content-Type", "application/json");

            await SendRequestAsync(request);

            if (typeof(T) == typeof(object))
            {
                return default;
            }

            return JsonUtility.FromJson<T>(request.downloadHandler.text);
        }

        private UnityWebRequest CreateRequest(string path, string method)
        {
            var request = new UnityWebRequest($"{_baseUrl}{path}", method);
            request.timeout = _timeoutSeconds;
            request.SetRequestHeader("Authorization", $"Bearer {_jwtToken}");
            request.SetRequestHeader("Accept", "application/json");
            return request;
        }

        private async Task SendRequestAsync(UnityWebRequest request)
        {
            var operation = request.SendWebRequest();

            while (!operation.isDone)
            {
                await Task.Yield();
            }

            if (request.result != UnityWebRequest.Result.Success)
            {
                throw new NpcApiException(
                    request.responseCode,
                    request.error,
                    request.downloadHandler?.text
                );
            }
        }

        public void Dispose()
        {
            _cache?.Clear();
        }
    }

    public class NpcApiException : Exception
    {
        public long StatusCode { get; }
        public string ResponseBody { get; }

        public NpcApiException(long statusCode, string error, string responseBody = null)
            : base($"NPC API Error ({statusCode}): {error}")
        {
            StatusCode = statusCode;
            ResponseBody = responseBody;
        }
    }
}
