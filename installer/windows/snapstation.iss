; Inno Setup script for the SnapStation Windows installer.
; Compiled in CI (see .github/workflows/release.yml) into SnapStation-Setup.exe.
; Produces a normal installer with Start-menu/desktop shortcuts and an uninstaller.

#define MyAppName "SnapStation"
#define MyAppVersion "0.6.2"

[Setup]
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher=SnapStation
; Per-user install: no admin rights required, and the install dir stays
; writable so the in-app updater can replace the binary.
PrivilegesRequired=lowest
DefaultDirName={autopf}\SnapStation
DefaultGroupName=SnapStation
DisableProgramGroupPage=yes
OutputDir=Output
OutputBaseFilename=SnapStation-Setup
Compression=lzma2
SolidCompression=yes
ArchitecturesInstallIn64BitMode=x64compatible
WizardStyle=modern
; Installer + uninstaller branding.
SetupIconFile=..\..\branding\icon.ico
UninstallDisplayIcon={app}\SnapStation.exe

[Languages]
Name: "german"; MessagesFile: "compiler:Languages\German.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Files]
Source: "..\..\target\release\snapstation.exe"; DestDir: "{app}"; DestName: "SnapStation.exe"; Flags: ignoreversion
Source: "..\..\README.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\SnapStation"; Filename: "{app}\SnapStation.exe"
Name: "{autodesktop}\SnapStation"; Filename: "{app}\SnapStation.exe"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Run]
Filename: "{app}\SnapStation.exe"; Description: "SnapStation jetzt starten"; Flags: nowait postinstall skipifsilent
