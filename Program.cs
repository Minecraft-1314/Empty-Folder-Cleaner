using System;
using System.Windows;
using System.Windows.Threading;
using EmptyFolderCleaner.Core.Logging;
using EmptyFolderCleaner.Ui;

namespace EmptyFolderCleaner;

public static class Program
{
    [STAThread]
    public static int Main()
    {
        AppLog.Bind();
        var application = new Application
        {
            ShutdownMode = ShutdownMode.OnMainWindowClose
        };
        application.Dispatcher.UnhandledException += OnDispatcherUnhandled;
        AppDomain.CurrentDomain.UnhandledException += OnDomainUnhandled;
        TaskScheduler.UnobservedTaskException += OnUnobservedTask;

        var window = new MainWindow();
        application.MainWindow = window;
        window.Show();
        return application.Run();
    }

    private static void OnDispatcherUnhandled(object sender, DispatcherUnhandledExceptionEventArgs args)
    {
        AppLog.HandleUnhandled(args.Exception);
        args.Handled = true;
        if (Application.Current?.MainWindow is MainWindow window)
        {
            window.ShowUnexpectedError();
        }
    }

    private static void OnDomainUnhandled(object sender, UnhandledExceptionEventArgs args)
    {
        if (args.ExceptionObject is Exception error)
        {
            AppLog.HandleUnhandled(error);
        }
    }

    private static void OnUnobservedTask(object? sender, UnobservedTaskExceptionEventArgs args)
    {
        AppLog.Error("unobserved task exception", args.Exception.ToString());
        args.SetObserved();
    }
}
