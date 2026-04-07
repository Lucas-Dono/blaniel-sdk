using UnityEngine;

namespace Blaniel.NPC.Models
{
    [Serializable]
    public class Position3D
    {
        public double X;
        public double Y;
        public double Z;
        public string World = "overworld";

        public Vector3 ToVector3() => new Vector3((float)X, (float)Y, (float)Z);

        public static Position3D FromVector3(Vector3 pos, string world = "overworld")
        {
            return new Position3D { X = pos.x, Y = pos.y, Z = pos.z, World = world };
        }
    }

    [Serializable]
    public class NpcState
    {
        public string Id;
        public string Name;
        public Position3D Position;
        public string CurrentAction = "idle";
        public string Animation = "idle";
        public string FacingDirection = "north";
        public string CachedEmotion = "neutral";
        public long CachedAt;
        public string Metadata;
    }

    [Serializable]
    public class HealthResponse
    {
        public string Status;
        public long Timestamp;
        public string Version;
    }

    [Serializable]
    public class ChatRequest
    {
        public string Message;
        public ChatContext Context;
    }

    [Serializable]
    public class ChatContext
    {
        public Position3D Position;
        public string Activity;
        public string[] NearbyPlayers;
        public int? TimeOfDay;
        public string Weather;
        public string[] NearbyNpcs;
    }

    [Serializable]
    public class ChatResponse
    {
        public string Response;
        public string Emotion;
        public string Animation;
        public string Source;
        public long LatencyMs;
        public bool? Cached;
    }

    [Serializable]
    public class NearbyNpcsResponse
    {
        public NpcState[] Npcs;
        public int Total;
    }

    [Serializable]
    public class BatchStateRequest
    {
        public string[] AgentIds;
    }

    [Serializable]
    public class BatchStateResponse
    {
        public NpcState[] States;
    }

    [Serializable]
    public class MoveRequest
    {
        public Position3D Position;
        public string Action;
        public double? FacingDirection;
    }

    [Serializable]
    public class PathfindRequest
    {
        public Position3D Start;
        public Position3D Goal;
        public PathfindOptions Options;
    }

    [Serializable]
    public class PathfindOptions
    {
        public bool AvoidWater = true;
        public bool AvoidLava = true;
        public int MaxFallDistance = 3;
        public bool AllowDiagonal = true;
        public int? MaxIterations;
    }

    [Serializable]
    public class PathResult
    {
        public Position3D[] Path;
        public double Cost;
        public long ComputedInMs;
        public bool? Cached;
    }

    [Serializable]
    public class AmbientDialogueQuery
    {
        public string[] ParticipantIds;
        public string Context;
        public int? MaxExchanges;
    }

    [Serializable]
    public class AmbientDialoguesResponse
    {
        public AmbientDialogue[] Dialogues;
        public bool Cached;
        public string GroupHash;
    }

    [Serializable]
    public class AmbientDialogue
    {
        public string SpeakerId;
        public string Message;
        public string Emotion;
        public string Animation;
    }

    [Serializable]
    public class InvalidationRequest
    {
        public string EventType;
        public string AgentId;
        public long Timestamp;
    }

    [Serializable]
    public class ApiErrorResponse
    {
        public string Error;
        public string Code;
        public string Details;
    }
}
