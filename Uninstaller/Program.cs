using System.Diagnostics;
using System.Text.RegularExpressions;
using Microsoft.Win32;

namespace EmptyFolderCleaner.Uninstaller;

internal static partial class Program
{
    private const string EnglishName = "Empty Folder Cleaner";
    private const string ChineseName = "空文件夹清理器";

    [GeneratedRegex(@"(?i)\bmsiexec(?:\.exe)?\s+/[ix]\s*\{([0-9a-f-]{36})\}")]
    private static partial Regex ProductCodePattern();

    private sealed record InstalledProduct(string UninstallString, string? QuietUninstallString, Guid? ProductCode);

    [STAThread]
    private static int Main(string[] args)
    {
        var checkOnly = HasFlag(args, "--check");
        var quiet = HasFlag(args, "/quiet", "--quiet");
        try
        {
            var products = FindProducts();
            if (checkOnly)
            {
                return products.Length == 0 ? 2 : 0;
            }

            if (products.Length == 0)
            {
                return 2;
            }

            foreach (var product in products)
            {
                var exitCode = RunUninstall(product, quiet);
                if (exitCode != 0 && exitCode != 3010)
                {
                    return exitCode;
                }
            }

            return 0;
        }
        catch (Exception)
        {
            return 5;
        }
    }

    private static bool HasFlag(string[] args, params string[] flags)
    {
        return args.Any(argument => flags.Contains(argument, StringComparer.OrdinalIgnoreCase));
    }

    private static InstalledProduct[] FindProducts()
    {
        var products = new List<InstalledProduct>();
        foreach ((var hive, var view) in new[]
        {
            (RegistryHive.LocalMachine, RegistryView.Registry64),
            (RegistryHive.LocalMachine, RegistryView.Registry32),
            (RegistryHive.CurrentUser, RegistryView.Default)
        })
        {
            try
            {
                using var baseKey = RegistryKey.OpenBaseKey(hive, view);
                using var uninstall = baseKey.OpenSubKey(@"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall");
                if (uninstall is null)
                {
                    continue;
                }

                foreach (var subKeyName in uninstall.GetSubKeyNames())
                {
                    using var item = uninstall.OpenSubKey(subKeyName);
                    var displayName = item?.GetValue("DisplayName") as string;
                    if (!IsTargetProduct(displayName))
                    {
                        continue;
                    }

                    var command = item?.GetValue("UninstallString") as string ?? "";
                    var quietCommand = item?.GetValue("QuietUninstallString") as string;
                    if (command.Length == 0)
                    {
                        continue;
                    }

                    Guid? productCode = null;
                    var match = ProductCodePattern().Match(command);
                    if (match.Success && Guid.TryParse(match.Groups[1].Value, out var parsed))
                    {
                        productCode = parsed;
                    }

                    products.Add(new InstalledProduct(command, quietCommand, productCode));
                }
            }
            catch (Exception error)
            {
                System.Diagnostics.Debug.WriteLine(error);
            }
        }

        return products.Distinct().ToArray();
    }

    private static bool IsTargetProduct(string? displayName)
    {
        return displayName is not null
            && (displayName.Equals(EnglishName, StringComparison.Ordinal)
                || displayName.StartsWith(ChineseName, StringComparison.Ordinal));
    }

    private static int RunUninstall(InstalledProduct product, bool quiet)
    {
        string fileName;
        string arguments;
        if (product.ProductCode is Guid productCode)
        {
            fileName = "msiexec.exe";
            arguments = quiet
                ? $"/x \"{productCode:B}\" /qn /norestart"
                : $"/x \"{productCode:B}\" /passive /norestart";
        }
        else
        {
            var command = quiet && !string.IsNullOrWhiteSpace(product.QuietUninstallString)
                ? product.QuietUninstallString
                : product.UninstallString;
            (fileName, arguments) = SplitCommand(Environment.ExpandEnvironmentVariables(command));
            if (quiet && !Regex.IsMatch(arguments, @"(?i)(?:^|\s)/(?:VERYSILENT|S)(?:\s|$)"))
            {
                arguments = AppendArguments(arguments, "/VERYSILENT /SUPPRESSMSGBOXES /NORESTART");
            }
        }

        using var process = Process.Start(new ProcessStartInfo
        {
            FileName = fileName,
            Arguments = arguments,
            UseShellExecute = true
        });
        if (process is null)
        {
            return 3;
        }

        process.WaitForExit();
        return process.ExitCode;
    }

    private static (string FileName, string Arguments) SplitCommand(string command)
    {
        var value = command.Trim();
        if (value.StartsWith('"'))
        {
            var closingQuote = value.IndexOf('"', 1);
            if (closingQuote > 0)
            {
                return (value[1..closingQuote], value[(closingQuote + 1)..].TrimStart());
            }
        }

        var separator = value.IndexOfAny([' ', '\t']);
        return separator < 0
            ? (value, "")
            : (value[..separator], value[(separator + 1)..].TrimStart());
    }

    private static string AppendArguments(string current, string addition)
    {
        return string.IsNullOrWhiteSpace(current) ? addition : current + " " + addition;
    }
}
