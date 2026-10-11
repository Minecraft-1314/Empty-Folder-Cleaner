using System.IO;
using EmptyFolderCleaner.Core.Deleting;
using EmptyFolderCleaner.Core.Filtering;
using EmptyFolderCleaner.Core.Localization;
using EmptyFolderCleaner.Core.Models;
using EmptyFolderCleaner.Core.Scanning;
using EmptyFolderCleaner.Core.Security;
using EmptyFolderCleaner.Core.Services;
using EmptyFolderCleaner.Core.Settings;

namespace EmptyFolderCleaner.Ui.Bridge;

internal sealed record OperationStart(bool Accepted, string? ErrorCode);

internal sealed record RuntimeState(
    AppSettings Settings,
    string EffectiveLanguage,
    string EffectiveTheme,
    bool IsRecycleBinAvailable,
    bool IsDarkSystemTheme,
    string? LoadWarning,
    bool ConfigExists,
    string SettingsPath,
    int ClusterSize);

internal sealed class AppRuntime : IDisposable
{
    private const int MaxParallelScan = 8;
    private const int MaxTrackedIgnoredPaths = 5000;

    private readonly object _gate = new();
    private readonly SettingsStore _settingsStore;
    private readonly SemaphoreSlim _saveGate = new(1, 1);
    private AppSettings _settings;
    private string? _loadWarning;
    private CancellationTokenSource? _operationCancellation;
    private Task? _operationTask;
    private bool _isDisposed;

    public AppRuntime(SettingsStore settingsStore)
    {
        _settingsStore = settingsStore;
        var loaded = settingsStore.Load();
        _settings = loaded.Settings;
        _loadWarning = loaded.WarningCode;
    }

    public event Action<string, object?>? EventRaised;

    public RuntimeState GetState()
    {
        lock (_gate)
        {
            var language = SupportedLanguages.Normalize(_settings.Language) is { Length: > 0 } stored
                ? stored
                : SupportedLanguages.Detect();
            var theme = _settings.ThemeMode is "light" or "dark" ? _settings.ThemeMode : "system";
            return new RuntimeState(
                CloneSettings(_settings),
                language,
                theme,
                OperatingSystem.IsWindows(),
                SystemTheme.IsDark,
                _loadWarning,
                File.Exists(_settingsStore.SettingsPath),
                _settingsStore.SettingsPath,
                ClusterSizeService.GetClusterSize(_settings.ScanDirectory));
        }
    }

    public async Task<bool> SaveSettingsAsync(SaveSettingsInput input, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(input);
        await _saveGate.WaitAsync(cancellationToken).ConfigureAwait(false);
        try
        {
            AppSettings previous;
            AppSettings updated;
            lock (_gate)
            {
                previous = CloneSettings(_settings);
                updated = CloneSettings(_settings);
                updated.Language = input.Language;
                updated.ThemeMode = input.ThemeMode;
                updated.UseRecycleBin = input.UseRecycleBin;
                updated.ScanDirectory = input.ScanDirectory;
                updated.RecentDirectories = input.RecentDirectories.ToList();
                updated.IgnoredPaths = input.IgnoredPaths.ToList();
                updated.IgnorePatterns = input.IgnorePatterns.ToList();
                _settings = SettingsStore.Normalize(updated);
            }

            var outcome = await _settingsStore.SaveAsync(_settings, cancellationToken).ConfigureAwait(false);
            if (!outcome.Saved)
            {
                lock (_gate)
                {
                    _settings = previous;
                }

                RaiseEvent("settingsSaveFailed", new { outcome.WarningCode });
                return false;
            }

            lock (_gate)
            {
                _loadWarning = null;
            }

            RaiseEvent("settingsSaved", new { outcome.SettingsPath });
            return true;
        }
        finally
        {
            _saveGate.Release();
        }
    }

    public SettingsResetOutcome ResetSettings()
    {
        var outcome = _settingsStore.Reset();
        lock (_gate)
        {
            _settings = new AppSettings();
            _loadWarning = outcome.WarningCode;
        }

        RaiseEvent("settingsReset", new { outcome.WarningCode });
        return outcome;
    }

    public OperationStart StartScan(string? rootDirectory, IEnumerable<string>? ignoredPaths, IEnumerable<string>? ignorePatterns)
    {
        if (!PathGuard.TryNormalize(rootDirectory, out var root))
        {
            return new OperationStart(false, ErrorCategories.InvalidInput);
        }

        if (!Directory.Exists(root))
        {
            return new OperationStart(false, ErrorCategories.NotFound);
        }

        var normalizedPaths = SettingsStore.NormalizePathSet(ignoredPaths);
        var normalizedPatterns = SettingsStore.NormalizePatterns(ignorePatterns);
        return StartOperation(cancellation => RunScanAsync(root, normalizedPaths, normalizedPatterns, cancellation));
    }

    public OperationStart StartDelete(IReadOnlyList<string>? paths, bool useRecycleBin)
    {
        if (paths is null || paths.Count == 0)
        {
            return new OperationStart(false, "no_paths");
        }

        if (paths.Count > MaxTrackedIgnoredPaths)
        {
            return new OperationStart(false, ErrorCategories.InvalidInput);
        }

        string root;
        IReadOnlySet<string> ignoredPaths;
        IReadOnlyList<string> ignorePatterns;
        lock (_gate)
        {
            root = _settings.ScanDirectory;
            ignoredPaths = SettingsStore.NormalizePathSet(_settings.IgnoredPaths);
            ignorePatterns = SettingsStore.NormalizePatterns(_settings.IgnorePatterns);
        }

        if (!PathGuard.TryNormalize(root, out var normalizedRoot) || !Directory.Exists(normalizedRoot))
        {
            return new OperationStart(false, ErrorCategories.NotFound);
        }

        var request = new DeleteRequest(
            normalizedRoot,
            paths.ToList(),
            useRecycleBin,
            ignoredPaths,
            ignorePatterns);
        return StartOperation(cancellation => RunDeleteAsync(request, cancellation));
    }

    public void StopCurrentOperation()
    {
        CancellationTokenSource? cancellation;
        lock (_gate)
        {
            cancellation = _operationCancellation;
        }

        cancellation?.Cancel();
    }

    public async Task ShutdownAsync(TimeSpan timeout)
    {
        CancellationTokenSource? cancellation;
        Task? operation;
        lock (_gate)
        {
            cancellation = _operationCancellation;
            operation = _operationTask;
        }

        cancellation?.Cancel();
        if (operation is not null)
        {
            try
            {
                var completed = await Task.WhenAny(operation, Task.Delay(timeout)).ConfigureAwait(false);
                if (ReferenceEquals(completed, operation))
                {
                    await operation.ConfigureAwait(false);
                }
            }
            catch (Exception)
            {
            }
        }

        using var saveTimeout = new CancellationTokenSource(timeout);
        try
        {
            await _saveGate.WaitAsync(saveTimeout.Token).ConfigureAwait(false);
            _saveGate.Release();
        }
        catch (OperationCanceledException)
        {
        }
    }

    private async Task RunScanAsync(
        string root,
        IReadOnlySet<string> ignoredPaths,
        IReadOnlyList<string> ignorePatterns,
        CancellationTokenSource cancellation)
    {
        var wasCancelled = false;
        try
        {
            RaiseEvent("scanStarted", new { root, ClusterSize = ClusterSizeService.GetClusterSize(root) });
            var report = await EmptyFolderScanner.ScanAsync(
                new ScanOptions(root, ignoredPaths, ignorePatterns, MaxParallelScan),
                batch => RaiseEvent("scanBatch", new { entries = batch }),
                stats => RaiseEvent("scanProgress", stats),
                cancellation.Token).ConfigureAwait(false);
            wasCancelled = report.WasCancelled;
            RaiseEvent("scanCompleted", new { report.Stats, report.WasCancelled });
        }
        catch (OperationCanceledException)
        {
            wasCancelled = true;
        }
        catch (Exception exception)
        {
            RaiseEvent("operationFailed", new { code = ErrorCategories.FromException(exception) });
        }
        finally
        {
            RaiseEvent(wasCancelled ? "scanStopped" : "scanFinished", new { });
            EndOperation(cancellation);
        }
    }

    private async Task RunDeleteAsync(DeleteRequest request, CancellationTokenSource cancellation)
    {
        try
        {
            RaiseEvent("deleteStarted", new { total = request.Paths.Count });
            var progress = new InlineProgress<DeleteProgress>(value => RaiseEvent("deleteProgress", value));
            var report = await FolderDeleter.DeleteAsync(
                request,
                onEvent => RaiseEvent("deleteEvent", onEvent),
                progress,
                cancellation.Token).ConfigureAwait(false);

            RememberFailedPaths(report.FailedPaths);
            RaiseEvent("deleteCompleted", new
            {
                report.DeletedPaths,
                report.FailedPaths,
                report.RemainingPaths,
                report.WasCancelled
            });
        }
        catch (OperationCanceledException)
        {
            RaiseEvent("deleteCompleted", new
            {
                DeletedPaths = Array.Empty<string>(),
                FailedPaths = Array.Empty<string>(),
                RemainingPaths = request.Paths,
                WasCancelled = true
            });
        }
        catch (Exception exception)
        {
            RaiseEvent("operationFailed", new { code = ErrorCategories.FromException(exception) });
        }
        finally
        {
            EndOperation(cancellation);
        }
    }

    private void RememberFailedPaths(IEnumerable<string> paths)
    {
        lock (_gate)
        {
            var updated = CloneSettings(_settings);
            var merged = new HashSet<string>(updated.IgnoredPaths, PathGuard.PathComparer);
            foreach (var path in paths)
            {
                if (PathGuard.TryNormalize(path, out var normalized))
                {
                    merged.Add(normalized);
                }
            }

            updated.IgnoredPaths = merged.Take(MaxTrackedIgnoredPaths).ToList();
            _settings = updated;
        }
    }

    private OperationStart StartOperation(Func<CancellationTokenSource, Task> operation)
    {
        lock (_gate)
        {
            if (_operationCancellation is not null)
            {
                return new OperationStart(false, ErrorCategories.Busy);
            }

            var cancellation = new CancellationTokenSource();
            _operationCancellation = cancellation;
            try
            {
                _operationTask = Task.Run(() => operation(cancellation), CancellationToken.None);
                return new OperationStart(true, null);
            }
            catch
            {
                _operationCancellation = null;
                _operationTask = null;
                cancellation.Dispose();
                throw;
            }
        }
    }

    private void EndOperation(CancellationTokenSource cancellation)
    {
        lock (_gate)
        {
            if (!ReferenceEquals(_operationCancellation, cancellation))
            {
                return;
            }

            _operationCancellation = null;
            _operationTask = null;
        }

        cancellation.Dispose();
    }

    private void RaiseEvent(string type, object? payload)
    {
        EventRaised?.Invoke(type, payload);
    }

    public void Dispose()
    {
        if (_isDisposed)
        {
            return;
        }

        _isDisposed = true;
        _saveGate.Dispose();
    }

    private static AppSettings CloneSettings(AppSettings settings)
    {
        return new AppSettings
        {
            SchemaVersion = settings.SchemaVersion,
            Language = settings.Language,
            ThemeMode = settings.ThemeMode,
            UseRecycleBin = settings.UseRecycleBin,
            ScanDirectory = settings.ScanDirectory,
            RecentDirectories = settings.RecentDirectories.ToList(),
            IgnoredPaths = settings.IgnoredPaths.ToList(),
            IgnorePatterns = settings.IgnorePatterns.ToList()
        };
    }

    private sealed class InlineProgress<T>(Action<T> report) : IProgress<T>
    {
        public void Report(T value)
        {
            report(value);
        }
    }
}
