using System.IO;
using System.Diagnostics;
using System.Text.Json;
using System.Windows;
using System.Windows.Threading;
using EmptyFolderCleaner.Core.Localization;
using EmptyFolderCleaner.Core.Models;
using EmptyFolderCleaner.Core.Security;
using EmptyFolderCleaner.Core.Services;
using EmptyFolderCleaner.Core.Settings;
using Microsoft.Web.WebView2.Core;

namespace EmptyFolderCleaner.Ui.Bridge;

internal sealed class WebViewBridge : IDisposable
{
    private const int MaxPayloadChars = 4 * 1024 * 1024;
    private static readonly TimeSpan SaveTimeout = TimeSpan.FromSeconds(8);

    private readonly AppRuntime _runtime;
    private readonly CoreWebView2 _webView;
    private readonly Dispatcher _dispatcher;
    private readonly object _lifecycleGate = new();
    private readonly CancellationTokenSource _lifetimeCancellation = new();
    private readonly TaskCompletionSource<bool> _drained = new(TaskCreationOptions.RunContinuationsAsynchronously);
    private int _activeRequests;
    private bool _isShutdownRequested;
    private bool _isLifetimeDisposed;
    private bool _isAttached;
    private bool _isDisposed;

    public WebViewBridge(AppRuntime runtime, CoreWebView2 webView, Dispatcher dispatcher)
    {
        _runtime = runtime;
        _webView = webView;
        _dispatcher = dispatcher;
    }

    public void Attach()
    {
        if (_isAttached)
        {
            return;
        }

        _isAttached = true;
        _webView.WebMessageReceived += OnWebMessageReceived;
        _runtime.EventRaised += OnRuntimeEvent;
    }

    public void Detach()
    {
        if (!_isAttached)
        {
            return;
        }

        _isAttached = false;
        _webView.WebMessageReceived -= OnWebMessageReceived;
        _runtime.EventRaised -= OnRuntimeEvent;
    }

    private async void OnWebMessageReceived(object? sender, CoreWebView2WebMessageReceivedEventArgs e)
    {
        lock (_lifecycleGate)
        {
            if (_isShutdownRequested)
            {
                return;
            }

            _activeRequests++;
        }

        var requestId = "";
        try
        {
            var request = ReadRequest(e.WebMessageAsJson);
            requestId = request.Id;
            var result = await ExecuteAsync(request, _lifetimeCancellation.Token).ConfigureAwait(true);
            Post(BridgeJson.Serialize(new BridgeResponse(request.Id, true, result, null)));
        }
        catch (OperationCanceledException)
        {
            PostError(requestId, ErrorCategories.Cancelled, ["request"]);
        }
        catch (BridgeException exception)
        {
            PostError(requestId, exception.Code, exception.Fields);
        }
        catch (JsonException)
        {
            PostError(requestId, "invalid_request", ["request"]);
        }
        catch (Exception exception)
        {
            PostError(requestId, ErrorCategories.FromException(exception), ["request"]);
        }
        finally
        {
            CompleteRequest();
        }
    }

    private static BridgeRequest ReadRequest(string json)
    {
        if (json.Length > MaxPayloadChars)
        {
            throw new BridgeException("payload_too_large", ["request"]);
        }

        var request = JsonSerializer.Deserialize<BridgeRequest>(json, BridgeJson.Options)
            ?? throw new BridgeException("invalid_request", ["request"]);
        if (string.IsNullOrWhiteSpace(request.Id) || string.IsNullOrWhiteSpace(request.Action))
        {
            throw new BridgeException("invalid_request", ["request"]);
        }

        return request;
    }

    private async Task<object?> ExecuteAsync(BridgeRequest request, CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        switch (request.Action)
        {
            case Actions.GetState:
                return DescribeState();
            case Actions.GetSystemTheme:
                return new { isDark = SystemTheme.IsDark };
            case Actions.ChooseDirectory:
                return ChooseDirectory(_runtime.GetState().EffectiveLanguage);
            case Actions.StartScan:
                return StartScan(request);
            case Actions.StopOperation:
                _runtime.StopCurrentOperation();
                return new { accepted = true };
            case Actions.DeleteFolders:
                return StartDelete(request);
            case Actions.SaveSettings:
                return await SaveSettingsAsync(request, cancellationToken).ConfigureAwait(true);
            case Actions.ResetSettings:
                return DescribeReset(_runtime.ResetSettings());
            case Actions.OpenFolder:
                return OpenFolder(request.Payload.GetString());
            case Actions.CopyPath:
                return CopyPath(request.Payload.GetString());
            default:
                throw new BridgeException("unknown_action", ["action"]);
        }
    }

    private object DescribeState()
    {
        var state = _runtime.GetState();
        return new
        {
            settings = DescribeSettings(state.Settings),
            effectiveLanguage = state.EffectiveLanguage,
            effectiveTheme = state.EffectiveTheme,
            isDarkSystemTheme = state.IsDarkSystemTheme,
            isRecycleBinAvailable = state.IsRecycleBinAvailable,
            loadWarning = state.LoadWarning,
            configExists = state.ConfigExists,
            settingsPath = state.SettingsPath,
            schemaVersion = AppSettings.CurrentSchemaVersion
        };
    }

    private static object DescribeSettings(AppSettings settings)
    {
        return new
        {
            settings.Language,
            settings.ThemeMode,
            settings.UseRecycleBin,
            settings.ScanDirectory,
            settings.RecentDirectories,
            settings.IgnoredPaths,
            settings.IgnorePatterns
        };
    }

    private object StartScan(BridgeRequest request)
    {
        var payload = BridgeJson.Deserialize<ScanRequest>(request.Payload)
            ?? throw new BridgeException("invalid_request", ["request"]);
        var started = _runtime.StartScan(payload.Root, payload.IgnoredPaths, payload.IgnorePatterns);
        if (!started.Accepted)
        {
            throw new BridgeException(started.ErrorCode ?? ErrorCategories.Busy, ["root"]);
        }

        return new { accepted = true };
    }

    private object StartDelete(BridgeRequest request)
    {
        var payload = BridgeJson.Deserialize<DeleteRequest>(request.Payload)
            ?? throw new BridgeException("invalid_request", ["request"]);
        var started = _runtime.StartDelete(payload.Paths, payload.UseRecycleBin);
        if (!started.Accepted)
        {
            throw new BridgeException(started.ErrorCode ?? ErrorCategories.Busy, ["paths"]);
        }

        return new { accepted = true };
    }

    private async Task<object> SaveSettingsAsync(BridgeRequest request, CancellationToken cancellationToken)
    {
        var payload = BridgeJson.Deserialize<SaveSettingsInput>(request.Payload)
            ?? throw new BridgeException("invalid_request", ["request"]);
        using var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        timeout.CancelAfter(SaveTimeout);
        var saved = await _runtime.SaveSettingsAsync(payload, timeout.Token).ConfigureAwait(true);
        return new { saved, warning = saved ? null : "settings_save_failed" };
    }

    private static object DescribeReset(SettingsResetOutcome outcome)
    {
        return new
        {
            cleared = outcome.ClearedJson,
            clearedLegacy = outcome.ClearedLegacy,
            outcome.WarningCode,
            settings = new
            {
                language = "",
                themeMode = "",
                useRecycleBin = true,
                scanDirectory = "",
                recentDirectories = Array.Empty<string>(),
                ignoredPaths = Array.Empty<string>(),
                ignorePatterns = Array.Empty<string>()
            }
        };
    }

    private static object ChooseDirectory(string language)
    {
        var dialog = new Microsoft.Win32.OpenFolderDialog
        {
            Title = AppText.Resolve(language, AppText.TextKeys.SelectFolder),
            Multiselect = false,
            ShowHiddenItems = false
        };
        return new { path = dialog.ShowDialog() == true ? dialog.FolderName : null };
    }

    private static object OpenFolder(string? path)
    {
        if (string.IsNullOrWhiteSpace(path) || !Directory.Exists(path))
        {
            throw new BridgeException(ErrorCategories.NotFound, ["path"]);
        }

        try
        {
            using var process = Process.Start(new ProcessStartInfo
            {
                FileName = path,
                UseShellExecute = true
            });
            return new { opened = true };
        }
        catch (Exception exception) when (exception is System.ComponentModel.Win32Exception or InvalidOperationException or IOException)
        {
            throw new BridgeException(ErrorCategories.FromException(exception), ["path"]);
        }
    }

    private static object CopyPath(string? path)
    {
        if (string.IsNullOrWhiteSpace(path))
        {
            throw new BridgeException("path_empty", ["path"]);
        }

        try
        {
            Clipboard.SetText(path);
            return new { copied = true };
        }
        catch (Exception exception) when (exception is System.Runtime.InteropServices.COMException or InvalidOperationException)
        {
            throw new BridgeException(ErrorCategories.FromException(exception), ["path"]);
        }
    }

    private void OnRuntimeEvent(string type, object? payload)
    {
        _dispatcher.BeginInvoke(new Action(() =>
        {
            try
            {
                Post(BridgeJson.Serialize(new BridgeEvent(type, payload)));
            }
            catch (Exception exception) when (exception is InvalidOperationException or ObjectDisposedException)
            {
            }
        }));
    }

    private void Post(string json)
    {
        try
        {
            _webView.PostWebMessageAsJson(json);
        }
        catch (Exception exception) when (exception is InvalidOperationException or ObjectDisposedException)
        {
        }
    }

    private void PostError(string requestId, string code, string[] fields)
    {
        if (string.IsNullOrWhiteSpace(requestId))
        {
            return;
        }

        Post(BridgeJson.Serialize(new BridgeResponse(requestId, false, null, new BridgeError(code, fields))));
    }

    public void Dispose()
    {
        if (_isDisposed)
        {
            return;
        }

        _isDisposed = true;
        Detach();
        var shouldDisposeLifetime = false;
        lock (_lifecycleGate)
        {
            _isShutdownRequested = true;
            if (_activeRequests == 0 && !_isLifetimeDisposed)
            {
                _isLifetimeDisposed = true;
                shouldDisposeLifetime = true;
            }
        }

        _lifetimeCancellation.Cancel();
        if (shouldDisposeLifetime)
        {
            _lifetimeCancellation.Dispose();
        }
        else
        {
            _ = _drained.Task.ContinueWith(
                _ => DisposeLifetime(),
                CancellationToken.None,
                TaskContinuationOptions.ExecuteSynchronously,
                TaskScheduler.Default);
        }
    }

    public async Task<bool> ShutdownAsync(TimeSpan timeout)
    {
        if (_isDisposed)
        {
            return true;
        }

        var shouldComplete = false;
        lock (_lifecycleGate)
        {
            _isShutdownRequested = true;
            shouldComplete = _activeRequests == 0;
        }

        if (shouldComplete)
        {
            _drained.TrySetResult(true);
        }

        _lifetimeCancellation.Cancel();
        var completed = await Task.WhenAny(_drained.Task, Task.Delay(timeout)).ConfigureAwait(false);
        return ReferenceEquals(completed, _drained.Task);
    }

    private void CompleteRequest()
    {
        var shouldComplete = false;
        lock (_lifecycleGate)
        {
            _activeRequests--;
            if (_activeRequests <= 0 && _isShutdownRequested)
            {
                _activeRequests = 0;
                shouldComplete = true;
            }
        }

        if (shouldComplete)
        {
            _drained.TrySetResult(true);
        }
    }

    private void DisposeLifetime()
    {
        lock (_lifecycleGate)
        {
            if (_isLifetimeDisposed)
            {
                return;
            }

            _isLifetimeDisposed = true;
        }

        _lifetimeCancellation.Dispose();
    }

    private static class Actions
    {
        public const string GetState = "app.getState";
        public const string GetSystemTheme = "app.getSystemTheme";
        public const string ChooseDirectory = "app.chooseDirectory";
        public const string StartScan = "scan.start";
        public const string StopOperation = "operation.stop";
        public const string DeleteFolders = "delete.start";
        public const string SaveSettings = "settings.save";
        public const string ResetSettings = "settings.reset";
        public const string OpenFolder = "shell.openFolder";
        public const string CopyPath = "shell.copyPath";
    }

}
