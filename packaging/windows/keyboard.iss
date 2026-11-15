; Windows installer for keyboard (Inno Setup 6). Built by `cargo xtask package`:
;   iscc /DAppVersion=0.1.0 /DBinDir=<dir with keyboard.exe> /DOutDir=<output dir> keyboard.iss
; Relative paths below are relative to this file.

#ifndef AppVersion
  #error Pass /DAppVersion=X.Y.Z
#endif
#ifndef BinDir
  #define BinDir "..\..\target\release"
#endif
#ifndef OutDir
  #define OutDir "..\..\dist"
#endif

[Setup]
; Never change AppId: it is how Windows recognises an upgrade of the same app.
AppId={{01160590-C735-4A36-89B8-1F722AE434CC}
AppName=keyboard
AppVersion={#AppVersion}
AppVerName=keyboard {#AppVersion}
AppPublisher=Nicolae Postica
AppPublisherURL=https://github.com/nicolaepostica/keyboard
AppSupportURL=https://github.com/nicolaepostica/keyboard/issues
AppUpdatesURL=https://github.com/nicolaepostica/keyboard/releases
VersionInfoVersion={#AppVersion}
DefaultDirName={autopf}\keyboard
DisableProgramGroupPage=yes
; Per-user install by default (no admin prompt); the first page offers "for all users".
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir={#OutDir}
OutputBaseFilename=keyboard-v{#AppVersion}-windows-x86_64-setup
SetupIconFile=..\..\assets\keyboard.ico
UninstallDisplayIcon={app}\keyboard.exe
UninstallDisplayName=keyboard
WizardStyle=modern
Compression=lzma2
SolidCompression=yes
; keyboard keeps running in the tray; PrepareToInstall stops it instead of the Restart Manager.
CloseApplications=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "autostart"; Description: "Start keyboard when I sign in"; GroupDescription: "Startup:"
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#BinDir}\keyboard.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE-MIT"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE-APACHE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\keyboard"; Filename: "{app}\keyboard.exe"
Name: "{autodesktop}\keyboard"; Filename: "{app}\keyboard.exe"; Tasks: desktopicon

[Registry]
; The same value the app's own "Start at login" switch writes (autostart.rs, run_value).
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; \
  ValueName: "keyboard"; ValueData: """{app}\keyboard.exe"" --minimized"; Tasks: autostart
; Removed on uninstall, also when the app's switch created it.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: none; \
  ValueName: "keyboard"; Flags: uninsdeletevalue

[Run]
Filename: "{app}\keyboard.exe"; Description: "{cm:LaunchProgram,keyboard}"; \
  Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "{sys}\taskkill.exe"; Parameters: "/F /IM keyboard.exe"; Flags: runhidden; \
  RunOnceId: "StopKeyboard"

[Code]
// An upgrade must replace keyboard.exe, which is locked while the app runs.
function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  ResultCode: Integer;
begin
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM keyboard.exe', '', SW_HIDE,
    ewWaitUntilTerminated, ResultCode);
  Result := '';
end;
