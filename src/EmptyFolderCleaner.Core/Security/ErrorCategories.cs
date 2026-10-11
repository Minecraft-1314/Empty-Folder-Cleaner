namespace EmptyFolderCleaner.Core.Security;

public static class ErrorCategories
{
    public const string Unknown = "operation_failed";
    public const string PermissionDenied = "permission_denied";
    public const string NotFound = "not_found";
    public const string PathTooLong = "path_too_long";
    public const string IoError = "io_error";
    public const string Cancelled = "cancelled";
    public const string Timeout = "timeout";
    public const string InvalidInput = "invalid_input";
    public const string Busy = "operation_busy";

    public static string FromException(Exception exception)
    {
        ArgumentNullException.ThrowIfNull(exception);
        return exception switch
        {
            UnauthorizedAccessException or System.Security.SecurityException => PermissionDenied,
            DirectoryNotFoundException or FileNotFoundException => NotFound,
            PathTooLongException => PathTooLong,
            IOException => IoError,
            TimeoutException => Timeout,
            OperationCanceledException => Cancelled,
            ArgumentException or NotSupportedException or FormatException => InvalidInput,
            _ => Unknown
        };
    }

}
