using EmptyFolderCleaner.Core.Models;
using Microsoft.Win32;

namespace EmptyFolderCleaner.Core.Settings;

internal sealed class LegacySettingsReader : ILegacySettingsSource
{
    private const string LegacyRegistryPath = @"Software\EmptyFolderCleaner\Settings";
    private const char LegacyListSeparator = '|';

    public bool TryRead(out AppSettings settings)
    {
        settings = new AppSettings();
        if (!OperatingSystem.IsWindows())
        {
            return false;
        }

        try
        {
            using var key = Registry.CurrentUser.OpenSubKey(LegacyRegistryPath);
            if (key is null)
            {
                return false;
            }

            settings = new AppSettings
            {
                Language = ReadString(key, "language") ?? "",
                ThemeMode = ReadString(key, "theme_mode") ?? "",
                UseRecycleBin = ReadBoolean(key, "use_trash", defaultValue: true),
                ScanDirectory = ReadString(key, "scan_dir") ?? "",
                RecentDirectories = ReadStringList(key, "recent_dirs"),
                IgnoredPaths = ReadStringList(key, "ignored_paths"),
                IgnorePatterns = ReadStringList(key, "ignore_patterns")
            };
            return true;
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException or System.Security.SecurityException)
        {
            return false;
        }
    }

    public bool TryClear()
    {
        if (!OperatingSystem.IsWindows())
        {
            return true;
        }

        try
        {
            using var key = Registry.CurrentUser.OpenSubKey(LegacyRegistryPath);
            if (key is null)
            {
                return true;
            }

            Registry.CurrentUser.DeleteSubKeyTree(LegacyRegistryPath, throwOnMissingSubKey: false);
            return true;
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException or System.Security.SecurityException or ArgumentException)
        {
            return false;
        }
    }

    private static string? ReadString(RegistryKey key, string name)
    {
        return key.GetValue(name) as string;
    }

    private static bool ReadBoolean(RegistryKey key, string name, bool defaultValue)
    {
        return key.GetValue(name) switch
        {
            bool typed => typed,
            int integer => integer != 0,
            long integer => integer != 0,
            string text when bool.TryParse(text, out var parsed) => parsed,
            string text when int.TryParse(text, out var parsed) => parsed != 0,
            _ => defaultValue
        };
    }

    private static List<string> ReadStringList(RegistryKey key, string name)
    {
        var values = ReadRawList(key, name);
        return values
            .Where(value => !string.IsNullOrWhiteSpace(value))
            .Select(value => value.Trim())
            .ToList();
    }

    private static string[] ReadRawList(RegistryKey key, string name)
    {
        if (key.GetValue(name) is string[] array)
        {
            return array;
        }

        if (key.GetValue(name) is string single && !single.StartsWith("@Variant", StringComparison.Ordinal))
        {
            var separator = new[] { LegacyListSeparator };
            return single.Split(separator, StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
        }

        return [];
    }
}
