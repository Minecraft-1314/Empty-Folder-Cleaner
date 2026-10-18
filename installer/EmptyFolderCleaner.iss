[Setup]
AppId={{8A1B76E5-3B4E-4E8A-9E48-6C5F0A9F2D11}
AppName=Empty Folder Cleaner
AppVersion=0.0.1
AppPublisher=Empty Folder Cleaner Contributors
VersionInfoVersion=0.0.1
DefaultDirName={autopf}\EmptyFolderCleaner
DefaultGroupName=Empty Folder Cleaner
DisableProgramGroupPage=yes
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\dist
OutputBaseFilename=EmptyFolderCleaner-Setup-0.0.1-win-x64
SetupIconFile=..\app.ico
UninstallIconFile=..\app.ico
UninstallDisplayIcon={app}\app.ico
UninstallDisplayName=Empty Folder Cleaner
Uninstallable=yes
CreateUninstallRegKey=yes
WizardStyle=modern dynamic windows11
ShowLanguageDialog=yes
UsePreviousLanguage=yes
Compression=lzma2
SolidCompression=yes
CloseApplications=yes
RestartApplications=no

[Languages]
Name: "zh"; MessagesFile: "compiler:Languages\ChineseSimplified.isl"
Name: "en"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "..\dist\EmptyFolderCleaner-win-x64\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\target\release\uninstall.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\app.ico"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Empty Folder Cleaner"; Filename: "{app}\EmptyFolderCleaner.exe"; IconFilename: "{app}\app.ico"
Name: "{group}\Uninstall Empty Folder Cleaner"; Filename: "{uninstallexe}"; IconFilename: "{app}\app.ico"
Name: "{autodesktop}\Empty Folder Cleaner"; Filename: "{app}\EmptyFolderCleaner.exe"; IconFilename: "{app}\app.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\EmptyFolderCleaner.exe"; Description: "{cm:LaunchProgram,Empty Folder Cleaner}"; Flags: nowait postinstall skipifsilent
