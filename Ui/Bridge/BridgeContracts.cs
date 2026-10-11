using System.Text.Json;
using System.Text.Json.Serialization;
using EmptyFolderCleaner.Core.Models;

namespace EmptyFolderCleaner.Ui.Bridge;

internal sealed record BridgeRequest(string Id, string Action, JsonElement Payload);

internal sealed record BridgeResponse(string Id, bool Ok, object? Result, BridgeError? Error);

internal sealed record BridgeError(string Code, string[] Fields);

internal sealed record BridgeEvent(string Type, object? Payload);

internal sealed record SaveSettingsInput(
    string Language,
    string ThemeMode,
    bool UseRecycleBin,
    string ScanDirectory,
    IReadOnlyList<string> RecentDirectories,
    IReadOnlyList<string> IgnoredPaths,
    IReadOnlyList<string> IgnorePatterns);

internal sealed record ScanRequest(
    string Root,
    IReadOnlyList<string> IgnoredPaths,
    IReadOnlyList<string> IgnorePatterns);

internal sealed record DeleteFoldersInput(IReadOnlyList<string> Paths, bool UseRecycleBin);

internal static class BridgeJson
{
    public static readonly JsonSerializerOptions Options = new()
    {
        PropertyNameCaseInsensitive = true,
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
        NumberHandling = JsonNumberHandling.AllowReadingFromString
    };

    public static string Serialize(object value)
    {
        return JsonSerializer.Serialize(value, Options);
    }

    public static T? Deserialize<T>(JsonElement element)
    {
        return element.ValueKind == JsonValueKind.Undefined || element.ValueKind == JsonValueKind.Null
            ? default
            : element.Deserialize<T>(Options);
    }
}
