using System.Text.Json;
using System.Text.Json.Serialization;
using EmptyFolderCleaner.Core.Localization;
using EmptyFolderCleaner.Core.Models;
using EmptyFolderCleaner.Core.Security;

namespace EmptyFolderCleaner.Core.Settings;

public sealed class SettingsStore
{
    public const int MaxSettingsFileBytes = 2 * 1024 * 1024;

    private const int MaxRecentDirectories = 8;
    private const int MaxIgnoredPaths = 5000;
    private const int MaxIgnorePatterns = 200;
    private const int WriteBufferSize = 16384;

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNameCaseInsensitive = true,
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        WriteIndented = true,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
        AllowTrailingCommas = true,
        ReadCommentHandling = JsonCommentHandling.Skip
    };

    private readonly string _settingsPath;
    private readonly ILegacySettingsSource _legacySource;

    public SettingsStore(string? settingsPath = null, ILegacySettingsSource? legacySource = null)
    {
        _legacySource = legacySource ?? new LegacySettingsReader();
        _settingsPath = settingsPath ?? Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            "EmptyFolderCleaner",
            "settings.json");
    }

    public string SettingsPath => _settingsPath;

    public SettingsLoadResult Load()
    {
        if (!File.Exists(_settingsPath))
        {
            return LoadWithoutJson();
        }

        try
        {
            var json = ReadTextWithinLimit(_settingsPath);
            if (json is null)
            {
                return RecoverFromBackup();
            }

            var parsed = JsonSerializer.Deserialize<AppSettings>(json, JsonOptions);
            if (parsed is null)
            {
                return new SettingsLoadResult(new AppSettings(), SettingsSource.None, "settings_empty");
            }

            var warning = parsed.SchemaVersion < AppSettings.CurrentSchemaVersion ? "settings_upgraded" : null;
            return new SettingsLoadResult(Normalize(parsed), SettingsSource.Json, warning);
        }
        catch (Exception exception) when (IsSettingsReadFailure(exception))
        {
            return RecoverFromBackup();
        }
    }

    public async Task<SettingsSaveOutcome> SaveAsync(AppSettings settings, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(settings);
        var normalized = Normalize(settings);
        var directory = Path.GetDirectoryName(_settingsPath);
        if (string.IsNullOrWhiteSpace(directory))
        {
            return new SettingsSaveOutcome(false, "settings_path_invalid", _settingsPath);
        }

        string temporaryPath = "";
        try
        {
            Directory.CreateDirectory(directory);
            temporaryPath = Path.Combine(directory, $"settings-{Guid.NewGuid():N}.tmp");
            await WriteAsync(temporaryPath, normalized, cancellationToken).ConfigureAwait(false);
            ArchiveExistingSettings();
            File.Move(temporaryPath, _settingsPath, overwrite: true);
            return new SettingsSaveOutcome(true, null, _settingsPath);
        }
        catch (OperationCanceledException)
        {
            TryDelete(temporaryPath);
            throw;
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException or JsonException)
        {
            TryDelete(temporaryPath);
            return new SettingsSaveOutcome(false, ErrorCategories.FromException(exception), _settingsPath);
        }
    }

    public SettingsResetOutcome Reset()
    {
        var clearedJson = TryDelete(_settingsPath);
        var clearedLegacy = _legacySource.TryClear();
        TryDelete(BuildBackupPath());
        return new SettingsResetOutcome(
            clearedJson,
            clearedLegacy,
            clearedJson && clearedLegacy ? null : "settings_reset_incomplete");
    }

    public static AppSettings Normalize(AppSettings settings)
    {
        ArgumentNullException.ThrowIfNull(settings);
        return new AppSettings
        {
            SchemaVersion = AppSettings.CurrentSchemaVersion,
            Language = SupportedLanguages.Normalize(settings.Language),
            ThemeMode = NormalizeTheme(settings.ThemeMode),
            UseRecycleBin = settings.UseRecycleBin,
            ScanDirectory = NormalizeDirectory(settings.ScanDirectory),
            RecentDirectories = DistinctDirectories(settings.RecentDirectories, MaxRecentDirectories),
            IgnoredPaths = DistinctDirectories(settings.IgnoredPaths, MaxIgnoredPaths),
            IgnorePatterns = NormalizePatterns(settings.IgnorePatterns)
        };
    }

    public static List<string> NormalizePatterns(IEnumerable<string>? patterns)
    {
        if (patterns is null)
        {
            return [];
        }

        return patterns
            .Select(pattern => pattern?.Trim() ?? "")
            .Where(pattern => pattern.Length is > 0 and <= 260 && !pattern.Any(char.IsControl))
            .Distinct(StringComparer.OrdinalIgnoreCase)
            .Take(MaxIgnorePatterns)
            .ToList();
    }

    public static IReadOnlySet<string> NormalizePathSet(IEnumerable<string>? paths)
    {
        var result = new HashSet<string>(PathGuard.PathComparer);
        if (paths is null)
        {
            return result;
        }

        foreach (var path in paths)
        {
            if (PathGuard.TryNormalize(path, out var normalized))
            {
                result.Add(normalized);
            }
        }

        return result;
    }

    public static List<string> DistinctDirectories(IEnumerable<string>? paths, int limit)
    {
        if (paths is null)
        {
            return [];
        }

        var result = new List<string>();
        var seen = new HashSet<string>(PathGuard.PathComparer);
        foreach (var path in paths)
        {
            if (result.Count >= limit)
            {
                break;
            }

            if (PathGuard.TryNormalize(path, out var normalized) && seen.Add(normalized))
            {
                result.Add(normalized);
            }
        }

        return result;
    }

    private static async Task WriteAsync(string path, AppSettings settings, CancellationToken cancellationToken)
    {
        var streamOptions = new FileStreamOptions
        {
            Mode = FileMode.CreateNew,
            Access = FileAccess.Write,
            Share = FileShare.None,
            BufferSize = WriteBufferSize,
            Options = FileOptions.Asynchronous | FileOptions.SequentialScan
        };

        await using var stream = new FileStream(path, streamOptions);
        await JsonSerializer.SerializeAsync(stream, settings, JsonOptions, cancellationToken).ConfigureAwait(false);
        await stream.FlushAsync(cancellationToken).ConfigureAwait(false);
    }

    private static string NormalizeDirectory(string? value)
    {
        return PathGuard.TryNormalize(value, out var normalized) ? normalized : "";
    }

    private static string NormalizeTheme(string? value)
    {
        return value is "light" or "dark" or "system" ? value : "";
    }

    private SettingsLoadResult LoadWithoutJson()
    {
        return _legacySource.TryRead(out var legacy)
            ? new SettingsLoadResult(Normalize(legacy), SettingsSource.Legacy, "legacy_settings_detected")
            : new SettingsLoadResult(new AppSettings(), SettingsSource.None, null);
    }

    private SettingsLoadResult RecoverFromBackup()
    {
        var backup = TryReadBackup();
        return backup is null
            ? new SettingsLoadResult(new AppSettings(), SettingsSource.None, "settings_corrupt")
            : new SettingsLoadResult(Normalize(backup), SettingsSource.Recovered, "settings_recovered_from_backup");
    }

    private AppSettings? TryReadBackup()
    {
        var backupPath = BuildBackupPath();
        if (!File.Exists(backupPath))
        {
            return null;
        }

        try
        {
            var json = ReadTextWithinLimit(backupPath);
            return json is null ? null : JsonSerializer.Deserialize<AppSettings>(json, JsonOptions);
        }
        catch (Exception exception) when (IsSettingsReadFailure(exception))
        {
            return null;
        }
    }

    private static string? ReadTextWithinLimit(string path)
    {
        try
        {
            var info = new FileInfo(path);
            if (!info.Exists || info.Length > MaxSettingsFileBytes)
            {
                return null;
            }

            return File.ReadAllText(path);
        }
        catch (Exception exception) when (IsSettingsReadFailure(exception))
        {
            return null;
        }
    }

    private static bool IsSettingsReadFailure(Exception exception)
    {
        return exception is JsonException
            or IOException
            or UnauthorizedAccessException
            or System.Security.SecurityException
            or NotSupportedException;
    }

    private string BuildBackupPath()
    {
        return $"{_settingsPath}.bak";
    }

    private void ArchiveExistingSettings()
    {
        if (!File.Exists(_settingsPath))
        {
            return;
        }

        try
        {
            File.Copy(_settingsPath, BuildBackupPath(), overwrite: true);
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
        }
    }

    private static bool TryDelete(string path)
    {
        try
        {
            if (!string.IsNullOrEmpty(path) && File.Exists(path))
            {
                File.Delete(path);
            }

            return true;
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            return false;
        }
    }
}
