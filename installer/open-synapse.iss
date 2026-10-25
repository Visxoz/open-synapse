; Windows installer, built by the release workflow with Inno Setup 6:
;   ISCC /DAppVersion=<x.y.z> installer\open-synapse.iss
; Installs per user, without an admin prompt, so the app can update itself
; by running a newer installer silently with /relaunch=1.

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#define AppName "Open Synapse"
#define AppExe "open-synapse.exe"

[Setup]
; Never change: Windows matches updates and the uninstaller by it.
AppId={{65F7EDB3-6554-4162-A5C8-D783929F660A}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher=Visxoz
AppPublisherURL=https://github.com/Visxoz/open-synapse
AppSupportURL=https://github.com/Visxoz/open-synapse/issues
AppUpdatesURL=https://github.com/Visxoz/open-synapse/releases
DefaultDirName={localappdata}\Programs\{#AppName}
DisableDirPage=auto
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputDir=..\dist
OutputBaseFilename=open-synapse_{#AppVersion}_x64-setup
SetupIconFile=..\icons\icon.ico
UninstallDisplayIcon={app}\{#AppExe}
UninstallDisplayName={#AppName}
WizardStyle=modern
Compression=lzma2/max
SolidCompression=yes
; Close a running copy before replacing it, without asking when silent.
CloseApplications=force
RestartApplications=no

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\target\release\{#AppExe}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Registry]
; The app adds this itself for Start with Windows; only uninstalling removes it.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueName: "{#AppName}"; ValueType: none; Flags: dontcreatekey uninsdeletevalue

[Run]
Filename: "{app}\{#AppExe}"; Description: "{cm:LaunchProgram,{#AppName}}"; Flags: nowait postinstall skipifsilent
; After a silent update started from the app, open it again.
Filename: "{app}\{#AppExe}"; Flags: nowait; Check: Relaunch

[UninstallRun]
; It may be running in the tray, which would keep its exe from being removed.
Filename: "{sys}\taskkill.exe"; Parameters: "/f /im {#AppExe}"; Flags: runhidden; RunOnceId: "StopApp"

[Code]
function Relaunch: Boolean;
begin
  Result := ExpandConstant('{param:relaunch|0}') = '1';
end;
