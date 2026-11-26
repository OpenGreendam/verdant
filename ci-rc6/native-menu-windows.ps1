param(
    [Parameter(Mandatory=$true)][string]$Exe,
    [Parameter(Mandatory=$true)][string]$OutputDir,
    [int]$ProcessId = 0,
    [switch]$CaptureOnly,
    [switch]$RootOnly,
    [string]$Menu,
    [string]$Row,
    [string]$Outside,
    [ValidateRange(200,5000)][int]$SettleMs = 650
)
# Run in the interactive guest desktop. Coordinates are comma-separated client
# pixels from calibration PNGs. Row is the File > Import parent rectangle.
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;

public static class VerdantMenuInput {
    [StructLayout(LayoutKind.Sequential)]
    public struct Point { public int X, Y; }
    [StructLayout(LayoutKind.Sequential)]
    public struct Rect { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
    [DllImport("user32.dll", SetLastError=true)] static extern bool GetClientRect(IntPtr hwnd, out Rect rect);
    [DllImport("user32.dll", SetLastError=true)] static extern bool ClientToScreen(IntPtr hwnd, ref Point point);
    [DllImport("user32.dll", SetLastError=true)] static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll", SetLastError=true)] static extern bool GetCursorPos(out Point point);
    [DllImport("user32.dll")] static extern IntPtr WindowFromPoint(Point point);
    [DllImport("user32.dll")] static extern IntPtr GetAncestor(IntPtr hwnd, uint flags);
    [DllImport("user32.dll")] static extern short GetAsyncKeyState(int key);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hwnd, int command);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr hwnd);
    [DllImport("user32.dll")] static extern void mouse_event(uint flags, uint x, uint y, uint data, UIntPtr extra);
    [DllImport("user32.dll")] static extern void keybd_event(byte key, byte scan, uint flags, UIntPtr extra);

    public static Rect Client(IntPtr hwnd) {
        IntPtr previous = SetThreadDpiAwarenessContext(new IntPtr(-4));
        if (previous == IntPtr.Zero) throw new Win32Exception();
        try {
            Rect size;
            Point origin = new Point();
            if (!GetClientRect(hwnd, out size) || !ClientToScreen(hwnd, ref origin)) throw new Win32Exception();
            return new Rect { Left = origin.X, Top = origin.Y,
                              Right = origin.X + size.Right, Bottom = origin.Y + size.Bottom };
        } finally { SetThreadDpiAwarenessContext(previous); }
    }

    public static void Move(IntPtr hwnd, int x, int y) {
        Point destination = new Point { X = x, Y = y };
        CheckDestination(hwnd, destination);
        if (!SetCursorPos(x, y)) throw new Win32Exception();
    }

    static void CheckDestination(IntPtr hwnd, Point destination) {
        if (GetAncestor(WindowFromPoint(destination), 2) != hwnd) {
            throw new InvalidOperationException("The intended click is covered or outside the staged GUI.");
        }
    }

    public static void Click(IntPtr hwnd, int x, int y) {
        Point actual;
        if (!GetCursorPos(out actual)) throw new Win32Exception();
        if (actual.X != x || actual.Y != y) throw new InvalidOperationException("The pointer moved before the click.");
        CheckDestination(hwnd, actual);
        mouse_event(2, 0, 0, 0, UIntPtr.Zero);
        mouse_event(4, 0, 0, 0, UIntPtr.Zero);
    }

    public static void Key(byte key) {
        foreach (int modifier in new int[] { 0x10, 0x11, 0x12, 0x5B, 0x5C }) {
            if (GetAsyncKeyState(modifier) < 0) throw new InvalidOperationException("A modifier is held down.");
        }
        uint flags = (key >= 0x23 && key <= 0x28) ? 1u : 0u;
        keybd_event(key, 0, flags, UIntPtr.Zero);
        keybd_event(key, 0, flags | 2u, UIntPtr.Zero);
    }
}
'@

function Read-Coordinates([string]$Text, [int]$Count, [string]$Label) {
    if ($Text -notmatch ('^\d+(,\d+){' + ($Count - 1) + '}$')) {
        throw "$Label requires $Count comma-separated nonnegative integers"
    }
    return @($Text.Split(',') | ForEach-Object { [int]$_ })
}

if (!$CaptureOnly) { $menuPoint = Read-Coordinates $Menu 2 'Menu' }
if (!$CaptureOnly -and !$RootOnly) {
    $parentRow = Read-Coordinates $Row 4 'Row'
    $outsidePoint = Read-Coordinates $Outside 2 'Outside'
    if ($parentRow[2] -lt 40 -or $parentRow[3] -lt 12) { throw 'Row is too small for distinct pointer samples' }
}
$actualExe = (Resolve-Path -LiteralPath $Exe).ProviderPath
if (Test-Path -LiteralPath $OutputDir) { throw 'Output directory must be fresh' }
$output = (New-Item -ItemType Directory -Path $OutputDir).FullName
$receipt = [ordered]@{
    status = 'incomplete'
    exe = $actualExe
    exe_sha256 = (Get-FileHash -LiteralPath $actualExe).Hash.ToLowerInvariant()
    steps = [System.Collections.Generic.List[object]]::new()
    acceptance = @(
        'Root and sibling rows remain visible with the Import child open',
        'Child A and B attach to the same Import row with stable child bounds',
        'Home, End, Up and Down move selection; Right opens and Left closes only child',
        'Right reopens child; Escape, outside click and Tab dismiss menu'
    )
}

function Guard-Window {
    if ([VerdantMenuInput]::GetForegroundWindow() -ne $hwnd) { throw 'Staged GUI lost foreground focus' }
    $current = [VerdantMenuInput]::Client($hwnd)
    if ($current.Left -ne $initial.Left -or $current.Top -ne $initial.Top -or
        $current.Right -ne $initial.Right -or $current.Bottom -ne $initial.Bottom -or
        [VerdantMenuInput]::GetDpiForWindow($hwnd) -ne $receipt.dpi) { throw 'Client moved, resized or changed scale; recalibrate' }
}

function Wait-Frame { Start-Sleep -Milliseconds $SettleMs }

function Save-Frame([string]$Name, $Action) {
    Wait-Frame
    Guard-Window
    $image = Join-Path $output "$Name.png"
    $size = [System.Drawing.Size]::new(($initial.Right - $initial.Left), ($initial.Bottom - $initial.Top))
    $bitmap = [System.Drawing.Bitmap]::new($size.Width, $size.Height)
    try {
        $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
        try { $graphics.CopyFromScreen($initial.Left, $initial.Top, 0, 0, $size) }
        finally { $graphics.Dispose() }
        $bitmap.Save($image, [System.Drawing.Imaging.ImageFormat]::Png)
    } finally { $bitmap.Dispose() }
    $receipt.steps.Add([ordered]@{ name=$Name; action=$Action; image="$Name.png";
        sha256=(Get-FileHash -LiteralPath $image).Hash.ToLowerInvariant() })
}

function Click-Point([int[]]$At) {
    Guard-Window
    if ($At[0] -lt 0 -or $At[1] -lt 0 -or
        $At[0] -ge ($initial.Right - $initial.Left) -or $At[1] -ge ($initial.Bottom - $initial.Top)) { throw 'Point is outside client' }
    $x = $initial.Left + $At[0]
    $y = $initial.Top + $At[1]
    [VerdantMenuInput]::Move($hwnd, $x, $y)
    Start-Sleep -Milliseconds 90
    Guard-Window
    [VerdantMenuInput]::Click($hwnd, $x, $y)
}

function Open-Root([string]$Name) {
    Click-Point $menuPoint
    Save-Frame $Name @{click_client=$menuPoint}
}

function Send-MenuKey([string]$Key, [string]$Name) {
    $codes = @{ Tab=0x09; Escape=0x1B; End=0x23; Home=0x24; Left=0x25; Up=0x26; Right=0x27; Down=0x28 }
    Guard-Window
    [VerdantMenuInput]::Key($codes[$Key])
    Save-Frame $Name @{key=$Key}
}

try {
    if ($ProcessId) { $gui = Get-Process -Id $ProcessId }
    else { $gui = Start-Process -FilePath $actualExe -WorkingDirectory (Split-Path $actualExe) -PassThru }
    if ($gui.SessionId -ne (Get-Process -Id $PID).SessionId) { throw 'GUI and observer must share the interactive session' }
    $deadline = [DateTime]::UtcNow.AddSeconds(30)
    do {
        $gui.Refresh()
        if ($gui.HasExited) { throw "GUI exited with code $($gui.ExitCode)" }
        $hwnd = $gui.MainWindowHandle
        if ($hwnd -eq [IntPtr]::Zero) { Start-Sleep -Milliseconds 250 }
    } until ($hwnd -ne [IntPtr]::Zero -or [DateTime]::UtcNow -ge $deadline)
    if ($hwnd -eq [IntPtr]::Zero) { throw 'No visible main window within 30 seconds' }
    if (![string]::Equals($gui.Path, $actualExe, [StringComparison]::OrdinalIgnoreCase)) { throw 'Window executable differs from Exe' }
    if ([VerdantMenuInput]::IsIconic($hwnd)) { [void][VerdantMenuInput]::ShowWindow($hwnd, 9) }
    $receipt.session = $gui.SessionId
    $receipt.interactive = [Environment]::UserInteractive
    $receipt.explorer_sessions = @(Get-Process explorer -ErrorAction SilentlyContinue | Select-Object -ExpandProperty SessionId)
    $shell = New-Object -ComObject WScript.Shell
    for ($attempt = 0; $attempt -lt 8; $attempt++) {
        if ([VerdantMenuInput]::GetForegroundWindow() -eq $hwnd) { break }
        $shell.SendKeys('%')
        [void]$shell.AppActivate($gui.Id)
        [void][VerdantMenuInput]::ShowWindow($hwnd, 3)
        [void][VerdantMenuInput]::SetForegroundWindow($hwnd)
        Start-Sleep -Milliseconds 500
    }
    $receipt.foreground_hwnd = [VerdantMenuInput]::GetForegroundWindow().ToInt64()
    Wait-Frame
    $initial = [VerdantMenuInput]::Client($hwnd)
    $receipt.pid = $gui.Id
    $receipt.window = $hwnd.ToInt64()
    $receipt.dpi = [VerdantMenuInput]::GetDpiForWindow($hwnd)
    $receipt.client_geometry = $initial
    Save-Frame '00-initial' $null
    if ($CaptureOnly) { $receipt.status = 'calibration-captured'; return }
    Open-Root '01-root'
    if ($RootOnly) {
        Send-MenuKey Escape '02-root-calibration-dismissed'
        $receipt.status = 'root-calibration-captured'
        return
    }
    $x, $y, $width, $height = $parentRow
    $sampleA = @(($x + [int][Math]::Floor($width / 4)), ($y + [int][Math]::Floor($height / 3)))
    $sampleB = @(($x + [int][Math]::Floor(3 * $width / 4)), ($y + [int][Math]::Floor(2 * $height / 3)))
    Click-Point $sampleA
    Save-Frame 'child-a' @{parent_row=$parentRow; click_client=$sampleA}
    Send-MenuKey Escape '04-dismiss-a'
    Open-Root '05-root-b'
    Click-Point $sampleB
    Save-Frame 'child-b' @{parent_row=$parentRow; click_client=$sampleB}
    Send-MenuKey Left '07-key-left'
    Send-MenuKey Right '08-key-right'
    Send-MenuKey Down '09-key-down'
    Send-MenuKey Up '10-key-up'
    Send-MenuKey Escape '11-escape-dismiss'
    Open-Root '12-root-keyboard'
    Send-MenuKey Home '13-key-home'
    Send-MenuKey End '14-key-end'
    Send-MenuKey Escape '15-escape-dismiss'
    Open-Root '16-root-outside'
    Click-Point $outsidePoint
    Save-Frame '17-outside-dismiss' @{click_client=$outsidePoint}
    Open-Root '18-root-tab'
    Send-MenuKey Tab '19-tab-dismiss'
    $receipt.status = 'captured-requires-visual-review'
} catch {
    $receipt.error = $_.Exception.Message
    throw
} finally {
    $receipt | ConvertTo-Json -Depth 8 | Set-Content -Encoding UTF8 -LiteralPath (Join-Path $output 'observations.json')
}
