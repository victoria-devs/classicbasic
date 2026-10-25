Attribute VB_Name = "Module1"
Option Explicit

' =============================================================================
' ClassicBasic App41: WebAssembly Canvas Client & Streaming Protocol
' -----------------------------------------------------------------------------
' Phase 3.7 (v0.3.7): WebAssembly HTML5 Canvas 2D Client Engine (cb-client)
' and Minimal JS Shim Bridge.
'
' This test fixture exercises the full spectrum of Win32/GDI runtime
' surfaces consumed and driven by the WebAssembly client:
'   Test 1 -> Canvas Viewport Geometry & Dynamic Resize (GetClientRect, MoveWindow)
'   Test 2 -> Bitmap Surface Memory & Pixel Blitting (DIB/DDB, SetPixel, InvalidateRect)
'   Test 3 -> Delegated Text Run Layout & Metrics (TextOut, DrawText, GetTextExtent)
'   Test 4 -> Browser Pointer Input Injection (WM_MOUSEMOVE, WM_LBUTTONDOWN/UP)
'   Test 5 -> Browser Keyboard Event Stream Injection (WM_KEYDOWN, WM_CHAR, WM_KEYUP)
'   Test 6 -> Form & Intrinsic Controls Remote Canvas Sync (Caption, Text, Label)
' =============================================================================

Private Type POINTAPI
    x As Long
    y As Long
End Type

Private Type RECT
    Left As Long
    Top As Long
    Right As Long
    Bottom As Long
End Type

Private Type SIZE
    cx As Long
    cy As Long
End Type

Private Type MSG
    hwnd As Long
    message As Long
    wParam As Long
    lParam As Long
    time As Long
    pt As POINTAPI
End Type

Private Type WNDCLASSA
    style As Long
    lpfnWndProc As Long
    cbClsExtra As Long
    cbWndExtra As Long
    hInstance As Long
    hIcon As Long
    hCursor As Long
    hbrBackground As Long
    lpszMenuName As String
    lpszClassName As String
End Type

' -----------------------------------------------------------------------------
' Win32 Constants
' -----------------------------------------------------------------------------
Private Const STD_OUTPUT_HANDLE As Long = -11
Private Const WS_VISIBLE As Long = &H10000000

Private Const WM_SIZE As Long = &H5
Private Const WM_KEYDOWN As Long = &H100
Private Const WM_KEYUP As Long = &H101
Private Const WM_CHAR As Long = &H102
Private Const WM_MOUSEMOVE As Long = &H200
Private Const WM_LBUTTONDOWN As Long = &H201
Private Const WM_LBUTTONUP As Long = &H202
Private Const WM_RBUTTONDOWN As Long = &H204

Private Const MK_LBUTTON As Long = &H1
Private Const VK_RETURN As Long = &HD
Private Const VK_W As Long = &H57

Private Const PM_REMOVE As Long = &H1

Private Const TRANSPARENT As Long = 1
Private Const OPAQUE As Long = 2
Private Const DT_LEFT As Long = &H0
Private Const DT_SINGLELINE As Long = &H20

' -----------------------------------------------------------------------------
' Win32 Declarations
' -----------------------------------------------------------------------------
Private Declare Function GetStdHandle Lib "kernel32" (ByVal nStdHandle As Long) As Long
Private Declare Function WriteFile Lib "kernel32" ( _
    ByVal hFile As Long, _
    ByVal lpBuffer As String, _
    ByVal nNumberOfBytesToWrite As Long, _
    lpNumberOfBytesWritten As Long, _
    ByVal lpOverlapped As Long) As Long
Private Declare Function GetModuleHandleA Lib "kernel32" (ByVal lpModuleName As String) As Long

Private Declare Function RegisterClassA Lib "user32" (ByRef lpWndClass As WNDCLASSA) As Integer
Private Declare Function CreateWindowExA Lib "user32" ( _
    ByVal dwExStyle As Long, _
    ByVal lpClassName As String, _
    ByVal lpWindowName As String, _
    ByVal dwStyle As Long, _
    ByVal x As Long, _
    ByVal y As Long, _
    ByVal nWidth As Long, _
    ByVal nHeight As Long, _
    ByVal hWndParent As Long, _
    ByVal hMenu As Long, _
    ByVal hInstance As Long, _
    ByVal lpParam As Long) As Long
Private Declare Function DestroyWindow Lib "user32" (ByVal hwnd As Long) As Long
Private Declare Function DefWindowProcA Lib "user32" (ByVal hwnd As Long, ByVal wMsg As Long, ByVal wParam As Long, ByVal lParam As Long) As Long
Private Declare Function MoveWindow Lib "user32" (ByVal hwnd As Long, ByVal x As Long, ByVal y As Long, ByVal nWidth As Long, ByVal nHeight As Long, ByVal bRepaint As Long) As Long
Private Declare Function GetClientRect Lib "user32" (ByVal hwnd As Long, ByRef lpRect As RECT) As Long
Private Declare Function GetWindowRect Lib "user32" (ByVal hwnd As Long, ByRef lpRect As RECT) As Long

Private Declare Function GetDC Lib "user32" (ByVal hwnd As Long) As Long
Private Declare Function ReleaseDC Lib "user32" (ByVal hwnd As Long, ByVal hdc As Long) As Long
Private Declare Function InvalidateRect Lib "user32" (ByVal hwnd As Long, ByRef lpRect As RECT, ByVal bErase As Long) As Long
Private Declare Function ValidateRect Lib "user32" (ByVal hwnd As Long, ByVal lpRect As Long) As Long
Private Declare Function GetUpdateRect Lib "user32" (ByVal hwnd As Long, ByRef lpRect As RECT, ByVal bErase As Long) As Long
Private Declare Function FillRect Lib "user32" (ByVal hdc As Long, ByRef lpRect As RECT, ByVal hBrush As Long) As Long
Private Declare Function DrawTextA Lib "user32" ( _
    ByVal hdc As Long, _
    ByVal lpchText As String, _
    ByVal nCount As Long, _
    ByRef lpRect As RECT, _
    ByVal uFormat As Long) As Long

Private Declare Function PostMessageA Lib "user32" (ByVal hwnd As Long, ByVal wMsg As Long, ByVal wParam As Long, ByVal lParam As Long) As Long
Private Declare Function PeekMessageA Lib "user32" (ByRef lpMsg As MSG, ByVal hwnd As Long, ByVal wMsgFilterMin As Long, ByVal wMsgFilterMax As Long, ByVal wRemoveMsg As Long) As Long

Private Declare Function CreateCompatibleDC Lib "gdi32" (ByVal hdc As Long) As Long
Private Declare Function DeleteDC Lib "gdi32" (ByVal hdc As Long) As Long
Private Declare Function CreateCompatibleBitmap Lib "gdi32" (ByVal hdc As Long, ByVal nWidth As Long, ByVal nHeight As Long) As Long
Private Declare Function SelectObject Lib "gdi32" (ByVal hdc As Long, ByVal hObject As Long) As Long
Private Declare Function DeleteObject Lib "gdi32" (ByVal hObject As Long) As Long
Private Declare Function CreateSolidBrush Lib "gdi32" (ByVal crColor As Long) As Long
Private Declare Function SetPixel Lib "gdi32" (ByVal hdc As Long, ByVal x As Long, ByVal y As Long, ByVal crColor As Long) As Long
Private Declare Function GetPixel Lib "gdi32" (ByVal hdc As Long, ByVal x As Long, ByVal y As Long) As Long
Private Declare Function SetTextColor Lib "gdi32" (ByVal hdc As Long, ByVal crColor As Long) As Long
Private Declare Function GetTextColor Lib "gdi32" (ByVal hdc As Long) As Long
Private Declare Function SetBkMode Lib "gdi32" (ByVal hdc As Long, ByVal nBkMode As Long) As Long
Private Declare Function GetBkMode Lib "gdi32" (ByVal hdc As Long) As Long
Private Declare Function TextOutA Lib "gdi32" (ByVal hdc As Long, ByVal x As Long, ByVal y As Long, ByVal lpString As String, ByVal nCount As Long) As Long
Private Declare Function GetTextExtentPoint32A Lib "gdi32" (ByVal hdc As Long, ByVal lpsz As String, ByVal cbString As Long, ByRef lpSize As SIZE) As Long

' -----------------------------------------------------------------------------
' Window Procedure & Function Pointer Helper
' -----------------------------------------------------------------------------
Public Function WindowProc(ByVal hwnd As Long, ByVal uMsg As Long, ByVal wParam As Long, ByVal lParam As Long) As Long
    WindowProc = DefWindowProcA(hwnd, uMsg, wParam, lParam)
End Function

Private Function GetWndProcAddress(ByVal fn As Long) As Long
    GetWndProcAddress = fn
End Function

' -----------------------------------------------------------------------------
' Helpers
' -----------------------------------------------------------------------------
Public Sub PrintLine(ByVal s As String)
    Dim hStdOut As Long
    Dim written As Long
    Dim msgText As String

    msgText = s & vbCrLf
    hStdOut = GetStdHandle(STD_OUTPUT_HANDLE)
    WriteFile hStdOut, msgText, Len(msgText), written, 0
End Sub

Private Function CreateCanvasWindow(ByVal className As String, ByVal w As Long, ByVal h As Long) As Long
    Dim wc As WNDCLASSA
    Dim atom As Integer
    Dim hInst As Long

    hInst = GetModuleHandleA(vbNullString)
    wc.style = 0
    wc.lpfnWndProc = GetWndProcAddress(AddressOf WindowProc)
    wc.cbClsExtra = 0
    wc.cbWndExtra = 0
    wc.hInstance = hInst
    wc.hIcon = 0
    wc.hCursor = 0
    wc.hbrBackground = 0
    wc.lpszMenuName = vbNullString
    wc.lpszClassName = className

    atom = RegisterClassA(wc)
    CreateCanvasWindow = CreateWindowExA(0, className, "WASM Canvas Host", WS_VISIBLE, 0, 0, w, h, 0, 0, hInst, 0)
End Function

Private Function MakeLParam(ByVal loWord As Long, ByVal hiWord As Long) As Long
    MakeLParam = (hiWord * &H10000) Or (loWord And &HFFFF&)
End Function

Private Sub ExpectMessage(ByVal hwnd As Long, ByVal expectedMsg As Long, ByVal expectedW As Long, ByVal expectedL As Long, ByVal errBase As Long)
    Dim m As MSG
    Dim res As Long

    res = PeekMessageA(m, hwnd, 0, 0, PM_REMOVE)
    If res = 0 Then Err.Raise errBase, , "PeekMessageA found no message, expected 0x" & Hex$(expectedMsg)
    If m.hwnd <> hwnd Then Err.Raise errBase + 1, , "MSG.hwnd mismatch: " & CStr(m.hwnd)
    If m.message <> expectedMsg Then Err.Raise errBase + 2, , "MSG.message expected 0x" & Hex$(expectedMsg) & " got 0x" & Hex$(m.message)
    If m.wParam <> expectedW Then Err.Raise errBase + 3, , "MSG.wParam expected " & CStr(expectedW) & " got " & CStr(m.wParam)
    If m.lParam <> expectedL Then Err.Raise errBase + 4, , "MSG.lParam expected 0x" & Hex$(expectedL) & " got 0x" & Hex$(m.lParam)
End Sub

' -----------------------------------------------------------------------------
' Main Entry Point
' -----------------------------------------------------------------------------
Public Sub Main()
    PrintLine "=== ClassicBasic App41: WebAssembly Canvas Client & Streaming Protocol ==="

    TestCanvasViewportGeometry
    TestBitmapSurfaceAndColorBlit
    TestDelegatedTextExtents
    TestPointerInputMessagePipeline
    TestKeyboardInputMessagePipeline
    TestFormAndControlCanvasSync

    PrintLine "=== App41 Test Suite Finished Successfully ==="
End Sub

' -----------------------------------------------------------------------------
' TEST 1: Canvas Viewport Geometry & Dynamic Resize
' -----------------------------------------------------------------------------
Private Sub TestCanvasViewportGeometry()
    Dim hwnd As Long
    Dim rc As RECT
    Dim res As Long

    hwnd = CreateCanvasWindow("WasmViewportClass", 640, 480)
    If hwnd = 0 Then Err.Raise 101, , "CreateCanvasWindow(640, 480) failed"

    res = GetClientRect(hwnd, rc)
    If res = 0 Then Err.Raise 102, , "GetClientRect failed"
    If rc.Right <= 0 Or rc.Bottom <= 0 Then Err.Raise 103, , "Initial client rect is empty: " & CStr(rc.Right) & "x" & CStr(rc.Bottom)

    res = MoveWindow(hwnd, 0, 0, 800, 600, 1)
    If res = 0 Then Err.Raise 104, , "MoveWindow(800, 600) failed"

    res = GetClientRect(hwnd, rc)
    If res = 0 Then Err.Raise 105, , "GetClientRect after resize failed"
    If rc.Right <= 0 Or rc.Bottom <= 0 Then Err.Raise 106, , "Resized client rect is empty: " & CStr(rc.Right) & "x" & CStr(rc.Bottom)

    DestroyWindow hwnd
    PrintLine "Test 1: Canvas viewport geometry and dynamic resize passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 2: Bitmap Surface Memory & Pixel Blitting
' -----------------------------------------------------------------------------
Private Sub TestBitmapSurfaceAndColorBlit()
    Dim hwnd As Long
    Dim hdc As Long
    Dim memDC As Long
    Dim hBmp As Long
    Dim oldBmp As Long
    Dim dmg As RECT
    Dim upd As RECT
    Dim px As Long
    Dim res As Long

    hwnd = CreateCanvasWindow("WasmBlitClass", 320, 240)
    If hwnd = 0 Then Err.Raise 200, , "CreateCanvasWindow failed"
    hdc = GetDC(hwnd)
    If hdc = 0 Then Err.Raise 201, , "GetDC failed"

    memDC = CreateCompatibleDC(hdc)
    If memDC = 0 Then Err.Raise 202, , "CreateCompatibleDC failed"

    hBmp = CreateCompatibleBitmap(hdc, 64, 64)
    If hBmp = 0 Then Err.Raise 203, , "CreateCompatibleBitmap failed"

    oldBmp = SelectObject(memDC, hBmp)

    ' Set reference color pixels using standard VB6 RGB helper
    SetPixel memDC, 10, 10, RGB(255, 0, 0)
    SetPixel memDC, 20, 20, RGB(0, 255, 0)
    SetPixel memDC, 30, 30, RGB(0, 0, 255)

    px = GetPixel(memDC, 10, 10)
    If px <> RGB(255, 0, 0) Then Err.Raise 204, , "GetPixel(10,10) Red mismatch: " & CStr(px)
    px = GetPixel(memDC, 20, 20)
    If px <> RGB(0, 255, 0) Then Err.Raise 205, , "GetPixel(20,20) Green mismatch: " & CStr(px)
    px = GetPixel(memDC, 30, 30)
    If px <> RGB(0, 0, 255) Then Err.Raise 206, , "GetPixel(30,30) Blue mismatch: " & CStr(px)

    ' Clear any pending window damage before testing damage tracking
    ValidateRect hwnd, 0

    ' Mark dirty region for atomic WebSocket FRAME_BITMAP_RECT transmission
    dmg.Left = 5
    dmg.Top = 5
    dmg.Right = 35
    dmg.Bottom = 35
    res = InvalidateRect(hwnd, dmg, 0)
    If res = 0 Then Err.Raise 207, , "InvalidateRect failed"

    res = GetUpdateRect(hwnd, upd, 0)
    If res = 0 Then Err.Raise 208, , "GetUpdateRect failed"
    If upd.Left > dmg.Left Or upd.Top > dmg.Top Or upd.Right < dmg.Right Or upd.Bottom < dmg.Bottom Then _
        Err.Raise 209, , "GetUpdateRect does not encompass invalidated rectangle"

    ValidateRect hwnd, 0

    SelectObject memDC, oldBmp
    DeleteObject hBmp
    DeleteDC memDC
    ReleaseDC hwnd, hdc
    DestroyWindow hwnd

    PrintLine "Test 2: Bitmap surface memory and color channel blit passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 3: Delegated Text Run Layout & Metrics
' -----------------------------------------------------------------------------
Private Sub TestDelegatedTextExtents()
    Dim hwnd As Long
    Dim hdc As Long
    Dim sz As SIZE
    Dim rc As RECT
    Dim res As Long
    Dim txt As String

    hwnd = CreateCanvasWindow("WasmTextClass", 320, 240)
    If hwnd = 0 Then Err.Raise 300, , "CreateCanvasWindow failed"
    hdc = GetDC(hwnd)
    If hdc = 0 Then Err.Raise 301, , "GetDC failed"

    SetTextColor hdc, RGB(136, 0, 255)
    If GetTextColor(hdc) <> RGB(136, 0, 255) Then Err.Raise 302, , "SetTextColor mismatch"

    SetBkMode hdc, TRANSPARENT
    If GetBkMode(hdc) <> TRANSPARENT Then Err.Raise 303, , "SetBkMode mismatch"

    txt = "ClassicBasic WASM Client"
    res = GetTextExtentPoint32A(hdc, txt, Len(txt), sz)
    If res = 0 Then Err.Raise 304, , "GetTextExtentPoint32A failed"
    If sz.cx <= 0 Or sz.cy <= 0 Then Err.Raise 305, , "Text extents empty: " & CStr(sz.cx) & "x" & CStr(sz.cy)

    res = TextOutA(hdc, 20, 30, txt, Len(txt))
    If res = 0 Then Err.Raise 306, , "TextOutA failed"

    rc.Left = 20
    rc.Top = 60
    rc.Right = 20 + sz.cx
    rc.Bottom = 60 + sz.cy
    res = DrawTextA(hdc, txt, Len(txt), rc, DT_LEFT Or DT_SINGLELINE)
    If res = 0 Then Err.Raise 307, , "DrawTextA failed"

    ReleaseDC hwnd, hdc
    DestroyWindow hwnd

    PrintLine "Test 3: Delegated text metrics and canvas draw commands passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 4: Browser Pointer Input Injection
' -----------------------------------------------------------------------------
Private Sub TestPointerInputMessagePipeline()
    Dim hwnd As Long
    Dim targetX As Long
    Dim targetY As Long
    Dim lp As Long
    Dim m As MSG

    hwnd = CreateCanvasWindow("WasmMouseClass", 640, 480)
    If hwnd = 0 Then Err.Raise 400, , "CreateCanvasWindow failed"

    ' Drain pending system messages
    Do While PeekMessageA(m, hwnd, 0, 0, PM_REMOVE) <> 0
    Loop

    targetX = 150
    targetY = 75
    lp = MakeLParam(targetX, targetY)

    ' Simulate WebSocket browser mouse stream
    PostMessageA hwnd, WM_MOUSEMOVE, 0, lp
    PostMessageA hwnd, WM_LBUTTONDOWN, MK_LBUTTON, lp
    PostMessageA hwnd, WM_LBUTTONUP, 0, lp

    ExpectMessage hwnd, WM_MOUSEMOVE, 0, lp, 401
    ExpectMessage hwnd, WM_LBUTTONDOWN, MK_LBUTTON, lp, 406
    ExpectMessage hwnd, WM_LBUTTONUP, 0, lp, 411

    DestroyWindow hwnd
    PrintLine "Test 4: Browser pointer input to Win32 message pipeline passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 5: Browser Keyboard Event Stream Injection
' -----------------------------------------------------------------------------
Private Sub TestKeyboardInputMessagePipeline()
    Dim hwnd As Long
    Dim m As MSG

    hwnd = CreateCanvasWindow("WasmKeyClass", 640, 480)
    If hwnd = 0 Then Err.Raise 500, , "CreateCanvasWindow failed"

    ' Drain pending system messages
    Do While PeekMessageA(m, hwnd, 0, 0, PM_REMOVE) <> 0
    Loop

    ' Simulate WebSocket browser keyboard input stream: KeyDown(W) -> Char(W) -> KeyUp(W)
    PostMessageA hwnd, WM_KEYDOWN, VK_W, 0
    PostMessageA hwnd, WM_CHAR, Asc("W"), 0
    PostMessageA hwnd, WM_KEYUP, VK_W, &HC0000001

    ExpectMessage hwnd, WM_KEYDOWN, VK_W, 0, 501
    ExpectMessage hwnd, WM_CHAR, Asc("W"), 0, 506
    ExpectMessage hwnd, WM_KEYUP, VK_W, &HC0000001, 511

    DestroyWindow hwnd
    PrintLine "Test 5: Browser keyboard events and char stream injection passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 6: Form & Intrinsic Controls Remote Canvas Sync
' -----------------------------------------------------------------------------
Private Sub TestFormAndControlCanvasSync()
    Dim rc As RECT

    Load Form1

    Form1.Caption = "Active Web Canvas"
    If Form1.Caption <> "Active Web Canvas" Then Err.Raise 601, , "Form1.Caption mismatch: " & Form1.Caption
    If Form1.hwnd = 0 Then Err.Raise 602, , "Form1.hWnd is 0"

    If GetClientRect(Form1.hwnd, rc) = 0 Then Err.Raise 603, , "GetClientRect(Form1.hWnd) failed"
    If rc.Right <= 0 Or rc.Bottom <= 0 Then Err.Raise 604, , "Form1 client rect is empty"

    Form1.Text1.Text = "Synchronized State"
    If Form1.Text1.Text <> "Synchronized State" Then Err.Raise 605, , "Text1.Text mismatch: " & Form1.Text1.Text

    Form1.Command1.Caption = "Apply"
    If Form1.Command1.Caption <> "Apply" Then Err.Raise 606, , "Command1.Caption mismatch: " & Form1.Command1.Caption

    Form1.Label1.Caption = "State: Ready"
    If Form1.Label1.Caption <> "State: Ready" Then Err.Raise 607, , "Label1.Caption mismatch: " & Form1.Label1.Caption

    Unload Form1
    PrintLine "Test 6: Form and intrinsic control canvas state synchronization passed."
End Sub
