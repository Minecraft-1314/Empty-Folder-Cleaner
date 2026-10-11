using System.Text.Json.Serialization;

namespace EmptyFolderCleaner.Core.Models;

public sealed class AppSettings
{
    public const int CurrentSchemaVersion = 2;

    public int SchemaVersion { get; set; } = CurrentSchemaVersion;

    public string Language { get; set; } = "";

    public string ThemeMode { get; set; } = "";

    public bool UseRecycleBin { get; set; } = true;

    public string ScanDirectory { get; set; } = "";

    public List<string> RecentDirectories { get; set; } = [];

    public List<string> IgnoredPaths { get; set; } = [];

    public List<string> IgnorePatterns { get; set; } = [];

    [JsonIgnore]
    public bool HasStoredValues =>
        Language.Length > 0
        || ThemeMode.Length > 0
        || ScanDirectory.Length > 0
        || RecentDirectories.Count > 0
        || IgnoredPaths.Count > 0
        || IgnorePatterns.Count > 0;
}

public enum SettingsSource
{
    None,
    Json,
    Legacy,
    Recovered
}

public sealed record SettingsLoadResult(AppSettings Settings, SettingsSource Source, string? WarningCode);

public sealed record SettingsSaveOutcome(bool Saved, string? WarningCode, string SettingsPath);

public sealed record SettingsResetOutcome(bool ClearedJson, bool ClearedLegacy, string? WarningCode);
