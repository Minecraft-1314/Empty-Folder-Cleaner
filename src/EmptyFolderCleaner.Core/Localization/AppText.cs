namespace EmptyFolderCleaner.Core.Localization;

public static class AppText
{
    private static readonly string[] Chinese = BuildChinese();
    private static readonly string[] English = BuildEnglish();

    public static string Resolve(string language, string key)
    {
        var index = Array.IndexOf(Keys, key);
        if (index < 0)
        {
            return key;
        }

        var table = SupportedLanguages.Normalize(language) == SupportedLanguages.Chinese ? Chinese : English;
        return table[index];
    }

    private static string[] BuildChinese()
    {
        return
        [
            "空文件夹清理器",
            "查找并删除电脑中的空文件夹",
            "正在启动本地界面...",
            "重试",
            "正在停止",
            "正在等待当前任务结束...",
            "本地界面文件缺失。请重新安装程序或恢复 Ui/Web 目录后重试。",
            "未检测到 Microsoft Edge WebView2 运行时。请安装 Evergreen WebView2 Runtime 后重试。",
            "本地界面启动失败。请检查 WebView2 运行时后重试，期间未修改任何文件。",
            "选择文件夹",
            "该文件夹已不存在。",
            "路径为空。",
            "程序遇到意外问题，已恢复到安全状态，未修改任何文件。",
            "详细信息已写入日志。",
            "取消",
            "确定"
        ];
    }

    private static string[] BuildEnglish()
    {
        return
        [
            "Empty Folder Cleaner",
            "Find and delete empty folders on your computer",
            "Starting the local interface...",
            "Retry",
            "Stopping",
            "Waiting for the current task to stop...",
            "The local interface files are missing. Reinstall the application or restore the Ui/Web folder, then retry.",
            "Microsoft Edge WebView2 Runtime was not found. Install the Evergreen WebView2 Runtime, then retry.",
            "The local interface could not start. Check the WebView2 Runtime and retry. No files were changed.",
            "Select a folder",
            "This folder no longer exists.",
            "The path is empty.",
            "Empty Folder Cleaner hit an unexpected problem and recovered to a safe state. No files were changed.",
            "Details were written to the local log.",
            "Cancel",
            "OK"
        ];
    }

    private static readonly string[] Keys =
    [
        TextKeys.Title,
        TextKeys.Subtitle,
        TextKeys.Starting,
        TextKeys.Retry,
        TextKeys.Stopping,
        TextKeys.WaitingForTask,
        TextKeys.FallbackMissingFiles,
        TextKeys.FallbackMissingRuntime,
        TextKeys.FallbackStartFailed,
        TextKeys.SelectFolder,
        TextKeys.FolderMissing,
        TextKeys.PathEmpty,
        TextKeys.UnexpectedError,
        TextKeys.UnexpectedErrorDetail,
        TextKeys.Cancel,
        TextKeys.Confirm
    ];

    public static class TextKeys
    {
        public const string Title = "title";
        public const string Subtitle = "subtitle";
        public const string Starting = "starting";
        public const string Retry = "retry";
        public const string Stopping = "stopping";
        public const string WaitingForTask = "waiting_for_task";
        public const string FallbackMissingFiles = "fallback_missing_files";
        public const string FallbackMissingRuntime = "fallback_missing_runtime";
        public const string FallbackStartFailed = "fallback_start_failed";
        public const string SelectFolder = "select_folder";
        public const string FolderMissing = "folder_missing";
        public const string PathEmpty = "path_empty";
        public const string UnexpectedError = "unexpected_error";
        public const string UnexpectedErrorDetail = "unexpected_error_detail";
        public const string Cancel = "cancel";
        public const string Confirm = "confirm";
    }
}
