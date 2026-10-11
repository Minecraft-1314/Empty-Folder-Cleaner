using System.Globalization;
using System.IO;
using System.Text;
using System.Text.RegularExpressions;

namespace EmptyFolderCleaner.Core.Logging;

public static partial class AppLog
{
    private const long MaxBytes = 512 * 1024;
    private static readonly object Gate = new();

    public static string FilePath { get; private set; } = "";

    public static void Bind()
    {
        try
        {
            FilePath = Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
                "EmptyFolderCleaner",
                "app.log");
        }
        catch (Exception)
        {
            FilePath = Path.Combine(Path.GetTempPath(), "empty-folder-cleaner.log");
        }
    }

    public static void Error(string message, string? detail = null) => Write("ERROR", message, detail);

    public static void HandleUnhandled(Exception error)
    {
        Error($"unhandled {error.GetType().Name}: {error.Message}", error.ToString());
    }

    private static void Write(string level, string message, string? detail)
    {
        var builder = new StringBuilder();
        builder.Append(DateTime.Now.ToString("yyyy-MM-dd HH:mm:ss", CultureInfo.InvariantCulture));
        builder.Append(" [").Append(level).Append("] ").Append(message);
        if (!string.IsNullOrEmpty(detail))
        {
            builder.Append(Environment.NewLine).Append(detail);
        }

        var line = Redact(builder.ToString());
        lock (Gate)
        {
            try
            {
                if (!string.IsNullOrEmpty(FilePath) && File.Exists(FilePath) && new FileInfo(FilePath).Length > MaxBytes)
                {
                    File.Delete(FilePath);
                }

                File.AppendAllText(FilePath, line + Environment.NewLine, Encoding.UTF8);
            }
            catch (Exception)
            {
                var fallbackPath = Path.Combine(Path.GetTempPath(), "empty-folder-cleaner.log");
                if (string.Equals(FilePath, fallbackPath, StringComparison.OrdinalIgnoreCase))
                {
                    return;
                }

                FilePath = fallbackPath;
                try
                {
                    File.AppendAllText(FilePath, line + Environment.NewLine, Encoding.UTF8);
                }
                catch (Exception fallbackError)
                {
                    System.Diagnostics.Debug.WriteLine(fallbackError);
                }
            }
        }
    }

    [GeneratedRegex(@"(?i)\bBearer\s+[A-Za-z0-9._~+/=-]+")]
    private static partial Regex BearerSecret();

    [GeneratedRegex(@"(?i)\b(authorization|cookie|set-cookie|password|passwd|pwd|access[_-]?token|refresh[_-]?token|token|secret|api[_-]?key)(""?\s*[:=]\s*""?)[^\s"";,}]+")]
    private static partial Regex NamedSecret();

    [GeneratedRegex(@"(?i)([?&](?:authorization|cookie|password|passwd|pwd|access[_-]?token|refresh[_-]?token|token|secret|api[_-]?key|code|state)=)[^&\s]+")]
    private static partial Regex QuerySecret();

    private static string Redact(string value)
    {
        var result = BearerSecret().Replace(value, "Bearer [REDACTED]");
        result = QuerySecret().Replace(result, "$1[REDACTED]");
        return NamedSecret().Replace(result, "$1$2[REDACTED]");
    }
}
