namespace EmptyFolderCleaner.Core.Security;

public static class PathGuard
{
    private const int MaxPathLength = 32767;

    public static StringComparison PathComparison => OperatingSystem.IsWindows()
        ? StringComparison.OrdinalIgnoreCase
        : StringComparison.Ordinal;

    public static StringComparer PathComparer => OperatingSystem.IsWindows()
        ? StringComparer.OrdinalIgnoreCase
        : StringComparer.Ordinal;

    public static bool TryNormalize(string? value, out string normalized)
    {
        normalized = "";
        var trimmed = value?.Trim();
        if (string.IsNullOrEmpty(trimmed)
            || trimmed.Length > MaxPathLength
            || trimmed.Any(char.IsControl)
            || HasDeviceNamespace(trimmed))
        {
            return false;
        }

        if (!IsRootedCandidate(trimmed))
        {
            return false;
        }

        try
        {
            var full = Path.GetFullPath(trimmed);
            if (full.Length == 0 || full.Length > MaxPathLength)
            {
                return false;
            }

            var trimmedFull = Path.TrimEndingDirectorySeparator(full);
            normalized = trimmedFull.Length > 0 ? trimmedFull : full;
            return true;
        }
        catch (Exception exception) when (exception is ArgumentException or NotSupportedException or PathTooLongException or IOException or System.Security.SecurityException)
        {
            return false;
        }
    }

    public static bool TryNormalizeNetworkShare(string? value, out string normalized)
    {
        normalized = "";
        if (!TryNormalize(value, out var candidate))
        {
            return false;
        }

        var isUnc = candidate.StartsWith(@"\\", StringComparison.Ordinal);
        if (!isUnc)
        {
            return false;
        }

        var segments = candidate.Split(['\\', '/'], StringSplitOptions.RemoveEmptyEntries);
        if (segments.Length < 2 || segments.Any(string.IsNullOrWhiteSpace))
        {
            return false;
        }

        normalized = candidate;
        return true;
    }

    public static bool IsWithinRoot(string rootDirectory, string candidatePath)
    {
        if (string.IsNullOrWhiteSpace(rootDirectory) || string.IsNullOrWhiteSpace(candidatePath))
        {
            return false;
        }

        if (!TryNormalize(rootDirectory, out var root) || !TryNormalize(candidatePath, out var candidate))
        {
            return false;
        }

        if (string.Equals(root, candidate, PathComparison))
        {
            return true;
        }

        var relative = Path.GetRelativePath(root, candidate);
        return relative.Length > 0
            && !Path.IsPathFullyQualified(relative)
            && !relative.Equals("..", StringComparison.Ordinal)
            && !relative.StartsWith($"..{Path.DirectorySeparatorChar}", StringComparison.Ordinal)
            && !relative.StartsWith($"..{Path.AltDirectorySeparatorChar}", StringComparison.Ordinal);
    }

    public static bool IsSamePath(string left, string right)
    {
        return TryNormalize(left, out var normalizedLeft)
            && TryNormalize(right, out var normalizedRight)
            && string.Equals(normalizedLeft, normalizedRight, PathComparison);
    }

    public static string GetParent(string path)
    {
        var parent = Path.GetDirectoryName(path);
        return string.IsNullOrEmpty(parent) ? "" : parent;
    }

    public static int GetDepth(string path)
    {
        return path.Count(character => character is '\\' or '/');
    }

    private static bool HasDeviceNamespace(string value)
    {
        return value.StartsWith(@"\\?\", StringComparison.Ordinal)
            || value.StartsWith(@"\\.\", StringComparison.Ordinal)
            || value.StartsWith("//?/", StringComparison.Ordinal)
            || value.StartsWith("//./", StringComparison.Ordinal);
    }

    private static bool IsRootedCandidate(string value)
    {
        if (Path.IsPathFullyQualified(value))
        {
            return true;
        }

        return value.StartsWith(@"\\", StringComparison.Ordinal)
            || value.StartsWith("//", StringComparison.Ordinal);
    }
}
