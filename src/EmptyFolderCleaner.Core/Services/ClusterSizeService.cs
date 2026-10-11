using System.Runtime.InteropServices;
using EmptyFolderCleaner.Core.Security;

namespace EmptyFolderCleaner.Core.Services;

public static class ClusterSizeService
{
    private const int DefaultClusterSize = 4096;

    public static int GetClusterSize(string? path)
    {
        if (!PathGuard.TryNormalize(path, out var normalized) || !Directory.Exists(normalized))
        {
            return DefaultClusterSize;
        }

        if (!OperatingSystem.IsWindows())
        {
            return DefaultClusterSize;
        }

        var root = Path.GetPathRoot(normalized);
        if (string.IsNullOrWhiteSpace(root))
        {
            return DefaultClusterSize;
        }

        try
        {
            if (!GetDiskFreeSpaceW(
                    root,
                    out var sectorsPerCluster,
                    out var bytesPerSector,
                    out _,
                    out _)
                || sectorsPerCluster == 0
                || bytesPerSector == 0)
            {
                return DefaultClusterSize;
            }

            var clusterSize = (ulong)sectorsPerCluster * bytesPerSector;
            return clusterSize is > 0 and <= 1024 * 1024 ? (int)clusterSize : DefaultClusterSize;
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException or System.Security.SecurityException)
        {
            return DefaultClusterSize;
        }
    }

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetDiskFreeSpaceW(
        string rootPath,
        out uint sectorsPerCluster,
        out uint bytesPerSector,
        out uint freeClusters,
        out uint totalClusters);
}
