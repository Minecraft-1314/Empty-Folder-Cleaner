using EmptyFolderCleaner.Core.Models;

namespace EmptyFolderCleaner.Core.Settings;

public interface ILegacySettingsSource
{
    bool TryRead(out AppSettings settings);

    bool TryClear();
}
