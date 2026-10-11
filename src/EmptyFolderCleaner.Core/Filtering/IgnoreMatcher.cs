using System.Text.RegularExpressions;
using EmptyFolderCleaner.Core.Security;

namespace EmptyFolderCleaner.Core.Filtering;

public sealed partial class IgnoreMatcher
{
    private readonly HashSet<string> _ignoredPaths;
    private readonly Regex[] _patterns;

    public IgnoreMatcher(IEnumerable<string> ignoredPaths, IEnumerable<string> patterns, bool caseInsensitive)
    {
        _ignoredPaths = new HashSet<string>(
            ignoredPaths.Where(path => !string.IsNullOrWhiteSpace(path)),
            caseInsensitive ? StringComparer.OrdinalIgnoreCase : StringComparer.Ordinal);
        _patterns = patterns
            .Select(pattern => pattern.Trim())
            .Where(IsUsablePattern)
            .Select(WildcardToRegex)
            .ToArray();
    }

    public bool IsIgnored(string path)
    {
        if (string.IsNullOrEmpty(path))
        {
            return false;
        }

        if (_ignoredPaths.Count > 0 && _ignoredPaths.Contains(Normalize(path)))
        {
            return true;
        }

        return MatchesName(Path.GetFileName(path.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar)));
    }

    public bool MatchesName(string? name)
    {
        if (string.IsNullOrEmpty(name) || _patterns.Length == 0)
        {
            return false;
        }

        foreach (var pattern in _patterns)
        {
            if (pattern.IsMatch(name))
            {
                return true;
            }
        }

        return false;
    }

    public static bool IsUsablePattern(string pattern)
    {
        return pattern.Length is > 0 and <= 260 && !pattern.Any(char.IsControl);
    }

    private static string Normalize(string path)
    {
        return PathGuard.TryNormalize(path, out var normalized) ? normalized : path.Trim();
    }

    private static Regex WildcardToRegex(string wildcard)
    {
        var builder = new System.Text.StringBuilder(wildcard.Length + 8);
        builder.Append('^');
        foreach (var character in wildcard)
        {
            switch (character)
            {
                case '*':
                    builder.Append(".*");
                    break;
                case '?':
                    builder.Append('.');
                    break;
                default:
                    builder.Append(Regex.Escape(character.ToString()));
                    break;
            }
        }

        builder.Append('$');
        return new Regex(
            builder.ToString(),
            RegexOptions.IgnoreCase | RegexOptions.CultureInvariant | RegexOptions.NonBacktracking,
            RegexTimeout);
    }

    private static readonly TimeSpan RegexTimeout = TimeSpan.FromMilliseconds(100);
}
