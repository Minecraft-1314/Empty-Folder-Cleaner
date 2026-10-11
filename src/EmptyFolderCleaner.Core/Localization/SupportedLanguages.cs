using System.Globalization;
using System.Runtime.InteropServices;

namespace EmptyFolderCleaner.Core.Localization;

public static class SupportedLanguages
{
    public const string Chinese = "zh-CN";
    public const string English = "en-US";

    public static readonly IReadOnlyList<string> All = [Chinese, English];

    public static bool IsSupported(string? language)
    {
        return Find(language) is not null;
    }

    public static string Normalize(string? language)
    {
        return Find(language) ?? "";
    }

    public static string Detect()
    {
        var culture = ResolveUserCulture();
        return IsChineseCulture(culture.Name) ? Chinese : English;
    }

    private static CultureInfo ResolveUserCulture()
    {
        try
        {
            return CultureInfo.InstalledUICulture;
        }
        catch (CultureNotFoundException)
        {
            return CultureInfo.InvariantCulture;
        }
    }

    private static string? Find(string? language)
    {
        if (string.IsNullOrWhiteSpace(language))
        {
            return null;
        }

        var exact = All.FirstOrDefault(candidate => candidate.Equals(language.Trim(), StringComparison.OrdinalIgnoreCase));
        if (exact is not null)
        {
            return exact;
        }

        var primaryTag = language.Trim().Split('-', '+')[0];
        return All.FirstOrDefault(candidate =>
            candidate.Split('-', '+')[0].Equals(primaryTag, StringComparison.OrdinalIgnoreCase));
    }

    private static bool IsChineseCulture(string cultureName)
    {
        return cultureName.StartsWith("zh", StringComparison.OrdinalIgnoreCase)
            || cultureName.Equals("cmn", StringComparison.OrdinalIgnoreCase);
    }
}
