using System.ComponentModel;
using System.IO;
using System.Windows;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Threading;
using EmptyFolderCleaner.Ui.Bridge;
using EmptyFolderCleaner.Core.Logging;
using EmptyFolderCleaner.Core.Localization;
using EmptyFolderCleaner.Core.Settings;
using Microsoft.Web.WebView2.Core;

namespace EmptyFolderCleaner.Ui;

public partial class MainWindow : Window
{
    private const string VirtualHostName = "app.local";
    private const string IndexFileName = "index.html";
    private static readonly TimeSpan ShutdownTimeout = TimeSpan.FromSeconds(10);

    private AppRuntime? _runtime;
    private WebViewBridge? _bridge;
    private bool _allowClose;
    private bool _isShutdownStarted;
    private bool _isInitialized;

    public MainWindow()
    {
        InitializeComponent();
        Title = AppText.Resolve(SupportedLanguages.Detect(), AppText.TextKeys.Title);
        FallbackLogo.Source = LoadFallbackLogo();
        Loaded += OnLoaded;
        Closing += OnClosing;
    }

    private static BitmapImage? LoadFallbackLogo()
    {
        try
        {
            var path = Path.Combine(AppContext.BaseDirectory, "Ui", "Web", "assets", "logo.png");
            if (!File.Exists(path))
            {
                return null;
            }

            var image = new BitmapImage();
            image.BeginInit();
            image.CacheOption = BitmapCacheOption.OnLoad;
            image.UriSource = new Uri(path);
            image.EndInit();
            image.Freeze();
            return image;
        }
        catch (Exception exception) when (exception is IOException or NotSupportedException or UriFormatException)
        {
            return null;
        }
    }

    private async void OnLoaded(object sender, RoutedEventArgs e)
    {
        Loaded -= OnLoaded;
        await InitializeAsync().ConfigureAwait(true);
    }

    private async Task InitializeAsync()
    {
        if (_isInitialized)
        {
            return;
        }

        var language = SupportedLanguages.Detect();
        ShowFallback(
            AppText.Resolve(language, AppText.TextKeys.Title),
            AppText.Resolve(language, AppText.TextKeys.Starting),
            isBusy: true);

        var wwwroot = ResolveWwwRoot();
        if (wwwroot is null)
        {
            ShowFallback(
                AppText.Resolve(language, AppText.TextKeys.Title),
                AppText.Resolve(language, AppText.TextKeys.FallbackMissingFiles),
                isBusy: false);
            return;
        }

        try
        {
            await Browser.EnsureCoreWebView2Async().ConfigureAwait(true);
            var core = Browser.CoreWebView2
                ?? throw new InvalidOperationException("core_webview_unavailable");
            ConfigureWebView(core);
            core.SetVirtualHostNameToFolderMapping(
                VirtualHostName,
                wwwroot,
                CoreWebView2HostResourceAccessKind.DenyCors);

            _runtime ??= new AppRuntime(new SettingsStore());
            _bridge ??= new WebViewBridge(_runtime, core, Dispatcher);
            _bridge.Attach();
            core.Navigate($"https://{VirtualHostName}/{IndexFileName}");
            _isInitialized = true;
        }
        catch (Exception exception)
        {
            AppLog.Error("window initialization failed", exception.ToString());
            ShowFallback(
                AppText.Resolve(language, AppText.TextKeys.Title),
                DescribeFallbackFailure(exception, language),
                isBusy: false);
        }
    }

    private static string? ResolveWwwRoot()
    {
        foreach (var candidate in new[]
        {
            Path.Combine(AppContext.BaseDirectory, "Ui", "Web"),
            Path.Combine(AppContext.BaseDirectory, "Assets", "Ui", "Web")
        })
        {
            if (File.Exists(Path.Combine(candidate, IndexFileName)))
            {
                return candidate;
            }
        }

        return null;
    }

    private void ConfigureWebView(CoreWebView2 core)
    {
        var settings = core.Settings;
        settings.AreDefaultContextMenusEnabled = false;
        settings.AreDevToolsEnabled = false;
        settings.AreHostObjectsAllowed = false;
        settings.IsStatusBarEnabled = false;
        settings.IsZoomControlEnabled = false;
        settings.IsGeneralAutofillEnabled = false;
        settings.IsPasswordAutosaveEnabled = false;
        settings.AreBrowserAcceleratorKeysEnabled = false;
        settings.IsWebMessageEnabled = true;
        settings.IsBuiltInErrorPageEnabled = true;
        core.NavigationStarting += OnNavigationStarting;
        core.NavigationCompleted += OnNavigationCompleted;
        core.ProcessFailed += (_, args) => Dispatcher.BeginInvoke(() => HandleProcessFailure(args));
    }

    private void OnNavigationCompleted(object? sender, CoreWebView2NavigationCompletedEventArgs e)
    {
        if (e.IsSuccess)
        {
            RevealInterface();
            return;
        }

        var language = SupportedLanguages.Detect();
        ShowFallback(
            AppText.Resolve(language, AppText.TextKeys.Title),
            AppText.Resolve(language, AppText.TextKeys.FallbackStartFailed),
            isBusy: false);
    }

    private void OnNavigationStarting(object? sender, CoreWebView2NavigationStartingEventArgs e)
    {
        var isLocal = e.Uri.StartsWith($"https://{VirtualHostName}/", StringComparison.OrdinalIgnoreCase);
        if (isLocal)
        {
            return;
        }

        e.Cancel = true;
        ShowFallback(
            AppText.Resolve(SupportedLanguages.Detect(), AppText.TextKeys.Title),
            AppText.Resolve(SupportedLanguages.Detect(), AppText.TextKeys.FallbackStartFailed),
            isBusy: false);
    }

    private void HandleProcessFailure(CoreWebView2ProcessFailedEventArgs args)
    {
        var language = SupportedLanguages.Detect();
        ShowFallback(
            AppText.Resolve(language, AppText.TextKeys.Title),
            AppText.Resolve(language, AppText.TextKeys.FallbackStartFailed),
            isBusy: false);
    }

    private void ShowFallback(string title, string message, bool isBusy)
    {
        Browser.Visibility = Visibility.Collapsed;
        FallbackPanel.Visibility = Visibility.Visible;
        FallbackTitle.Text = title;
        FallbackMessage.Text = message;
        RetryButton.IsEnabled = !isBusy;
        RetryButton.Visibility = isBusy ? Visibility.Hidden : Visibility.Visible;
    }

    private void RevealInterface()
    {
        FallbackPanel.Visibility = Visibility.Collapsed;
        Browser.Visibility = Visibility.Visible;
        Browser.Focus();
    }

    public void ShowUnexpectedError()
    {
        var language = SupportedLanguages.Detect();
        ShowFallback(
            AppText.Resolve(language, AppText.TextKeys.Title),
            AppText.Resolve(language, AppText.TextKeys.UnexpectedError)
                + Environment.NewLine
                + AppText.Resolve(language, AppText.TextKeys.UnexpectedErrorDetail),
            isBusy: false);
    }

    private async void OnRetryClick(object sender, RoutedEventArgs e)
    {
        await ShutdownRuntimeAsync().ConfigureAwait(true);
        _isInitialized = false;
        Browser.Visibility = Visibility.Collapsed;
        await InitializeAsync().ConfigureAwait(true);
    }

    private static string DescribeFallbackFailure(Exception exception, string language)
    {
        if (exception is WebView2RuntimeNotFoundException)
        {
            return AppText.Resolve(language, AppText.TextKeys.FallbackMissingRuntime);
        }

        return AppText.Resolve(language, AppText.TextKeys.FallbackStartFailed);
    }

    private async void OnClosing(object? sender, CancelEventArgs e)
    {
        if (_allowClose)
        {
            return;
        }

        e.Cancel = true;
        if (_isShutdownStarted)
        {
            return;
        }

        _isShutdownStarted = true;
        var language = SupportedLanguages.Detect();
        ShowFallback(
            AppText.Resolve(language, AppText.TextKeys.Stopping),
            AppText.Resolve(language, AppText.TextKeys.WaitingForTask),
            isBusy: true);

        try
        {
            await ShutdownRuntimeAsync().ConfigureAwait(true);
        }
        catch (Exception)
        {
        }
        finally
        {
            _bridge?.Dispose();
            _bridge = null;
            _runtime?.Dispose();
            _runtime = null;
            _ = Dispatcher.BeginInvoke(() =>
            {
                _allowClose = true;
                Close();
            });
        }
    }

    private async Task ShutdownRuntimeAsync()
    {
        _bridge?.Detach();
        try
        {
            if (_runtime is not null)
            {
                await _runtime.ShutdownAsync(ShutdownTimeout).ConfigureAwait(true);
            }
        }
        catch (Exception)
        {
        }

        try
        {
            if (_bridge is not null)
            {
                await _bridge.ShutdownAsync(ShutdownTimeout).ConfigureAwait(true);
            }
        }
        catch (Exception)
        {
        }

        _bridge?.Dispose();
        _bridge = null;
        _runtime?.Dispose();
        _runtime = null;
    }
}
