using System.IO;
using Microsoft.Win32;

namespace EmptyFolderCleaner.Ui.Bridge;

internal static class SystemTheme
{
    private const string AppsUseLightTheme = "AppsUseLightTheme";

    public static bool IsDark
    {
        get
        {
            try
            {
                using var key = Registry.CurrentUser.OpenSubKey(
                    @"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
                return key?.GetValue(AppsUseLightTheme) is not int lightTheme || lightTheme == 0;
            }
            catch (Exception exception) when (exception is IOException or UnauthorizedAccessException or System.Security.SecurityException)
            {
                return false;
            }
        }
    }
}
