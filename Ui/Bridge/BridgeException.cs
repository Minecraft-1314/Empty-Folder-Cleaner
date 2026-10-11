namespace EmptyFolderCleaner.Ui.Bridge;

internal sealed class BridgeException : Exception
{
    public BridgeException(string code, params string[] fields)
        : base(code)
    {
        Code = code;
        Fields = fields.Length == 0 ? [code] : fields;
    }

    public string Code { get; }

    public string[] Fields { get; }
}
