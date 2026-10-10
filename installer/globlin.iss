#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#ifndef Arch
  #define Arch "x64"
#endif
#ifndef SourceExe
  #define SourceExe "..\target\release\globlin.exe"
#endif

[Setup]
AppId={{380BD341-81C1-4AC3-AA2E-BC70BE5CC8F7}
AppName=Globlin
AppVersion={#AppVersion}
AppVerName=Globlin {#AppVersion}
AppPublisher=TOR968
AppPublisherURL=https://globlin.pages.dev
AppSupportURL=https://github.com/TOR968/Globlin/issues
AppUpdatesURL=https://github.com/TOR968/Globlin/releases
DefaultDirName={localappdata}\Programs\Globlin
DisableDirPage=auto
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
#if Arch == "arm64"
ArchitecturesAllowed=arm64
#else
ArchitecturesAllowed=x64compatible
#endif
OutputBaseFilename=globlin-setup-{#Arch}
#ifdef IconFile
SetupIconFile={#IconFile}
#endif
UninstallDisplayIcon={app}\globlin.exe
UninstallDisplayName=Globlin
VersionInfoVersion={#AppVersion}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
CloseApplications=no

[Tasks]
Name: "autostart"; Description: "Run Globlin when I sign in"
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#SourceExe}"; DestDir: "{app}"; DestName: "globlin.exe"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Globlin"; Filename: "{app}\globlin.exe"
Name: "{autodesktop}\Globlin"; Filename: "{app}\globlin.exe"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "globlin"; ValueData: """{app}\globlin.exe"""; Tasks: autostart
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: none; ValueName: "globlin"; Flags: uninsdeletevalue

[Run]
Filename: "{app}\globlin.exe"; Description: "{cm:LaunchProgram,Globlin}"; Flags: nowait postinstall skipifsilent

[Code]
procedure StopGloblin;
var
  ResultCode: Integer;
begin
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM globlin.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  StopGloblin;
  Result := '';
end;

function InitializeUninstall(): Boolean;
begin
  StopGloblin;
  Result := True;
end;
