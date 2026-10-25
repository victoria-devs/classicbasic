Attribute VB_Name = "Module1"
Option Explicit

' =============================================================================
' ClassicBasic App40: GUI WebSocket Stream - Wire Protocol Source Surfaces
' -----------------------------------------------------------------------------
' Phase 3.6 (v0.3.6): Atomic Wire Protocol Codec (0x02..0x07) & RFC 6455
' WebSocket Server (crates/cb-webserver).
'
' Each test exercises one Win32/VB6 runtime surface whose state is encoded by
' the cb-webserver frame encoder, plus the inbound message path that the JSON
' input frame parser feeds into the VirtualMessageQueue:
'   Test 1 -> 0x02 FRAME_BITMAP_RECT  (GDI pixel writes + dirty/update rects)
'   Test 2 -> 0x03 FRAME_FORM         (ThunderRT6FormDC + intrinsic controls)
'   Test 3 -> 0x05 FRAME_CURSOR       (LoadCursor / SetCursor / CursorPos)
'   Test 4 -> 0x06 FRAME_CLIPBOARD    (VB Clipboard object text round-trip)
'   Test 5 -> 0x07 FRAME_TEXT         (TextOut / DrawText / text attributes)
'   Test 6 -> Input frames            (mouse.*, key.*, char, resize -> WM_*)
' 0x04 FRAME_DIALOG (modal MsgBox/InputBox) is covered by the cb-webserver
' crate's own test suite, since a modal call would block a headless run.
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
Private Const WS_OVERLAPPEDWINDOW As Long = &HCF0000

' Window messages delivered by the WebSocket input frame parser
Private Const WM_SIZE As Long = &H5
Private Const WM_KEYDOWN As Long = &H100
Private Const WM_KEYUP As Long = &H101
Private Const WM_CHAR As Long = &H102
Private Const WM_MOUSEMOVE As Long = &H200
Private Const WM_LBUTTONDOWN As Long = &H201
Private Const WM_LBUTTONUP As Long = &H202
Private Const WM_RBUTTONDOWN As Long = &H204

Private Const MK_LBUTTON As Long = &H1
Private Const SIZE_RESTORED As Long = 0
Private Const VK_RETURN As Long = &HD
Private Const VK_A As Long = &H41

Private Const PM_NOREMOVE As Long = &H0
Private Const PM_REMOVE As Long = &H1

' Standard cursors
Private Const IDC_ARROW As Long = 32512
Private Const IDC_IBEAM As Long = 32513
Private Const IDC_WAIT As Long = 32514
Private Const IDC_HAND As Long = 32649

' Raster operations
Private Const WHITENESS As Long = &HFF0062
Private Const BLACKNESS As Long = &H42

' Text
Private Const TRANSPARENT As Long = 1
Private Const OPAQUE As Long = 2
Private Const DT_LEFT As Long = &H0
Private Const DT_SINGLELINE As Long = &H20
Private Const DT_CALCRECT As Long = &H400

' -----------------------------------------------------------------------------
' Win32 API Declarations
' -----------------------------------------------------------------------------
Private Declare Function GetStdHandle Lib "kernel32" (ByVal nStdHandle As Long) As Long
Private Declare Function WriteFile Lib "kernel32" ( _
    ByVal hFile As Long, _
    ByVal lpBuffer As String, _
    ByVal nNumberOfBytesToWrite As Long, _
    lpNumberOfBytesWritten As Long, _
    ByVal lpOverlapped As Long) As Long
Private Declare Function GetModuleHandleA Lib "kernel32" (ByVal lpModuleName As String) As Long

' USER32 - windowing
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
Private Declare Function GetWindowTextA Lib "user32" (ByVal hwnd As Long, ByVal lpString As String, ByVal cch As Long) As Long
Private Declare Function GetClientRect Lib "user32" (ByVal hwnd As Long, ByRef lpRect As RECT) As Long

' USER32 - painting / damage tracking
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

' USER32 - cursor
Private Declare Function LoadCursorA Lib "user32" (ByVal hInstance As Long, ByVal lpCursorName As Long) As Long
Private Declare Function SetCursor Lib "user32" (ByVal hCursor As Long) As Long
Private Declare Function GetCursor Lib "user32" () As Long
Private Declare Function SetCursorPos Lib "user32" (ByVal x As Long, ByVal y As Long) As Long
Private Declare Function GetCursorPos Lib "user32" (ByRef lpPoint As POINTAPI) As Long

' USER32 - message queue
Private Declare Function PostMessageA Lib "user32" (ByVal hwnd As Long, ByVal wMsg As Long, ByVal wParam As Long, ByVal lParam As Long) As Long
Private Declare Function PeekMessageA Lib "user32" (ByRef lpMsg As MSG, ByVal hwnd As Long, ByVal wMsgFilterMin As Long, ByVal wMsgFilterMax As Long, ByVal wRemoveMsg As Long) As Long

' GDI32
Private Declare Function CreateSolidBrush Lib "gdi32" (ByVal crColor As Long) As Long
Private Declare Function DeleteObject Lib "gdi32" (ByVal hObject As Long) As Long
Private Declare Function PatBlt Lib "gdi32" (ByVal hdc As Long, ByVal nXLeft As Long, ByVal nYLeft As Long, ByVal nWidth As Long, ByVal nHeight As Long, ByVal dwRop As Long) As Long
Private Declare Function SetPixel Lib "gdi32" (ByVal hdc As Long, ByVal x As Long, ByVal y As Long, ByVal crColor As Long) As Long
Private Declare Function GetPixel Lib "gdi32" (ByVal hdc As Long, ByVal x As Long, ByVal y As Long) As Long
Private Declare Function SetTextColor Lib "gdi32" (ByVal hdc As Long, ByVal crColor As Long) As Long
Private Declare Function GetTextColor Lib "gdi32" (ByVal hdc As Long) As Long
Private Declare Function SetBkMode Lib "gdi32" (ByVal hdc As Long, ByVal nBkMode As Long) As Long
Private Declare Function GetBkMode Lib "gdi32" (ByVal hdc As Long) As Long
Private Declare Function TextOutA Lib "gdi32" (ByVal hdc As Long, ByVal x As Long, ByVal y As Long, ByVal lpString As String, ByVal nCount As Long) As Long
Private Declare Function GetTextExtentPoint32A Lib "gdi32" (ByVal hdc As Long, ByVal lpsz As String, ByVal cbString As Long, ByRef lpSize As SIZE) As Long

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

Private Function CreateTestWindow(ByVal className As String) As Long
    Dim wc As WNDCLASSA
    Dim atom As Integer
    Dim hInst As Long

    hInst = GetModuleHandleA(vbNullString)
    wc.style = 0
    wc.lpfnWndProc = 0
    wc.cbClsExtra = 0
    wc.cbWndExtra = 0
    wc.hInstance = hInst
    wc.hIcon = 0
    wc.hCursor = 0
    wc.hbrBackground = 0
    wc.lpszMenuName = vbNullString
    wc.lpszClassName = className

    atom = RegisterClassA(wc)
    CreateTestWindow = CreateWindowExA(0, className, "Stream Test Window", WS_OVERLAPPEDWINDOW, 0, 0, 320, 240, 0, 0, hInst, 0)
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
    PrintLine "=== ClassicBasic App40: GUI WebSocket Stream & Wire Protocol Surfaces ==="

    TestBitmapRectDamageTracking
    TestFormSnapshotSurface
    TestCursorStateSurface
    TestClipboardTextSurface
    TestTextRunSurface
    TestInputFrameMessageInjection

    PrintLine "=== App40 Test Suite Finished Successfully ==="
End Sub

' -----------------------------------------------------------------------------
' TEST 1: 0x02 FRAME_BITMAP_RECT - pixel writes & dirty/update rectangles
' -----------------------------------------------------------------------------
Private Sub TestBitmapRectDamageTracking()
    Dim hwnd As Long
    Dim hdc As Long
    Dim hBrush As Long
    Dim rc As RECT
    Dim dmg As RECT
    Dim upd As RECT
    Dim res As Long
    Dim px As Long

    hwnd = CreateTestWindow("StreamClass1")
    If hwnd = 0 Then Err.Raise 101, , "Failed to create test window"

    hdc = GetDC(hwnd)
    If hdc = 0 Then Err.Raise 102, , "GetDC failed"

    ' Clear the surface to white, then paint a red 40x30 block at (10,20)
    res = PatBlt(hdc, 0, 0, 320, 240, WHITENESS)
    If res = 0 Then Err.Raise 103, , "PatBlt(WHITENESS) failed"

    hBrush = CreateSolidBrush(RGB(255, 0, 0))
    If hBrush = 0 Then Err.Raise 104, , "CreateSolidBrush failed"

    rc.Left = 10: rc.Top = 20: rc.Right = 50: rc.Bottom = 50
    res = FillRect(hdc, rc, hBrush)
    If res = 0 Then Err.Raise 105, , "FillRect failed"

    px = GetPixel(hdc, 10, 20)
    If px <> RGB(255, 0, 0) Then Err.Raise 106, , "GetPixel inside block expected red got " & CStr(px)
    px = GetPixel(hdc, 49, 49)
    If px <> RGB(255, 0, 0) Then Err.Raise 107, , "GetPixel at block corner expected red got " & CStr(px)
    px = GetPixel(hdc, 50, 50)
    If px <> RGB(255, 255, 255) Then Err.Raise 108, , "GetPixel outside block expected white got " & CStr(px)

    ' Single-pixel write
    SetPixel hdc, 100, 100, RGB(0, 0, 255)
    px = GetPixel(hdc, 100, 100)
    If px <> RGB(0, 0, 255) Then Err.Raise 109, , "SetPixel/GetPixel mismatch: " & CStr(px)

    ' Damage region: invalidate the painted block and read back the update rect
    res = ValidateRect(hwnd, 0)
    dmg.Left = 10: dmg.Top = 20: dmg.Right = 50: dmg.Bottom = 50
    res = InvalidateRect(hwnd, dmg, 0)
    If res = 0 Then Err.Raise 110, , "InvalidateRect failed"

    res = GetUpdateRect(hwnd, upd, 0)
    If res = 0 Then Err.Raise 111, , "GetUpdateRect reported no pending damage"
    If upd.Left <> 10 Or upd.Top <> 20 Or upd.Right <> 50 Or upd.Bottom <> 50 Then
        Err.Raise 112, , "Update rect mismatch: " & CStr(upd.Left) & "," & CStr(upd.Top) & "," & CStr(upd.Right) & "," & CStr(upd.Bottom)
    End If

    res = ValidateRect(hwnd, 0)
    If res = 0 Then Err.Raise 113, , "ValidateRect failed"
    res = GetUpdateRect(hwnd, upd, 0)
    If res <> 0 Then Err.Raise 114, , "GetUpdateRect still reports damage after ValidateRect"

    DeleteObject hBrush
    ReleaseDC hwnd, hdc
    DestroyWindow hwnd
    PrintLine "Test 1: FRAME_BITMAP_RECT pixel surface and damage rectangle tracking passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 2: 0x03 FRAME_FORM - form & control snapshot surface
' -----------------------------------------------------------------------------
Private Sub TestFormSnapshotSurface()
    Dim buf As String
    Dim n As Long
    Dim rc As RECT

    Load Form1

    Form1.Caption = "Stream Form"
    If Form1.Caption <> "Stream Form" Then Err.Raise 201, , "Form1.Caption mismatch: " & Form1.Caption

    If Form1.hwnd = 0 Then Err.Raise 202, , "Form1.hWnd is 0"

    buf = String$(64, vbNullChar)
    n = GetWindowTextA(Form1.hwnd, buf, 64)
    If Left$(buf, n) <> "Stream Form" Then Err.Raise 203, , "GetWindowTextA(Form1.hWnd) mismatch: " & Left$(buf, n)

    If GetClientRect(Form1.hwnd, rc) = 0 Then Err.Raise 204, , "GetClientRect(Form1.hWnd) failed"
    If rc.Right <= 0 Or rc.Bottom <= 0 Then Err.Raise 205, , "Form1 client rect is empty"

    Form1.Text1.Text = "Streamed Text"
    If Form1.Text1.Text <> "Streamed Text" Then Err.Raise 206, , "Text1.Text mismatch: " & Form1.Text1.Text

    Form1.Command1.Caption = "Send"
    If Form1.Command1.Caption <> "Send" Then Err.Raise 207, , "Command1.Caption mismatch: " & Form1.Command1.Caption

    Form1.Label1.Caption = "Status: Connected"
    If Form1.Label1.Caption <> "Status: Connected" Then Err.Raise 208, , "Label1.Caption mismatch: " & Form1.Label1.Caption

    Unload Form1
    PrintLine "Test 2: FRAME_FORM form and control snapshot surface passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 3: 0x05 FRAME_CURSOR - cursor shape & position
' -----------------------------------------------------------------------------
Private Sub TestCursorStateSurface()
    Dim hArrow As Long
    Dim hHand As Long
    Dim hIBeam As Long
    Dim hWait As Long
    Dim prev As Long
    Dim pt As POINTAPI

    hArrow = LoadCursorA(0, IDC_ARROW)
    hHand = LoadCursorA(0, IDC_HAND)
    hIBeam = LoadCursorA(0, IDC_IBEAM)
    hWait = LoadCursorA(0, IDC_WAIT)
    If hArrow = 0 Then Err.Raise 301, , "LoadCursorA(IDC_ARROW) returned 0"
    If hHand = 0 Then Err.Raise 302, , "LoadCursorA(IDC_HAND) returned 0"
    If hIBeam = 0 Then Err.Raise 303, , "LoadCursorA(IDC_IBEAM) returned 0"
    If hWait = 0 Then Err.Raise 304, , "LoadCursorA(IDC_WAIT) returned 0"
    If hArrow = hHand Or hArrow = hIBeam Or hHand = hWait Then Err.Raise 305, , "Standard cursor handles are not distinct"

    SetCursor hArrow
    prev = SetCursor(hHand)
    If prev <> hArrow Then Err.Raise 306, , "SetCursor did not return previous cursor"
    If GetCursor() <> hHand Then Err.Raise 307, , "GetCursor mismatch after SetCursor(IDC_HAND)"

    prev = SetCursor(hIBeam)
    If prev <> hHand Then Err.Raise 308, , "SetCursor(IDC_IBEAM) previous mismatch"
    If GetCursor() <> hIBeam Then Err.Raise 309, , "GetCursor mismatch after SetCursor(IDC_IBEAM)"

    If SetCursorPos(123, 77) = 0 Then Err.Raise 310, , "SetCursorPos failed"
    If GetCursorPos(pt) = 0 Then Err.Raise 311, , "GetCursorPos failed"
    If pt.x <> 123 Or pt.y <> 77 Then Err.Raise 312, , "GetCursorPos mismatch: " & CStr(pt.x) & "," & CStr(pt.y)

    SetCursor hArrow
    PrintLine "Test 3: FRAME_CURSOR cursor shape and position surface passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 4: 0x06 FRAME_CLIPBOARD - VB Clipboard object text round-trip
' -----------------------------------------------------------------------------
Private Sub TestClipboardTextSurface()
    Dim s As String

    Clipboard.Clear
    Clipboard.SetText "ClassicBasic Clipboard Payload"

    If Not Clipboard.GetFormat(vbCFText) Then Err.Raise 401, , "Clipboard.GetFormat(vbCFText) is False after SetText"

    s = Clipboard.GetText()
    If s <> "ClassicBasic Clipboard Payload" Then Err.Raise 402, , "Clipboard.GetText mismatch: " & s

    Clipboard.SetText "Second Payload"
    s = Clipboard.GetText(vbCFText)
    If s <> "Second Payload" Then Err.Raise 403, , "Clipboard.GetText overwrite mismatch: " & s

    Clipboard.Clear
    If Clipboard.GetFormat(vbCFText) Then Err.Raise 404, , "Clipboard.GetFormat(vbCFText) still True after Clear"

    PrintLine "Test 4: FRAME_CLIPBOARD clipboard text round-trip passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 5: 0x07 FRAME_TEXT - text runs, attributes & layout extents
' -----------------------------------------------------------------------------
Private Sub TestTextRunSurface()
    Dim hwnd As Long
    Dim hdc As Long
    Dim sz As SIZE
    Dim rc As RECT
    Dim res As Long
    Dim h As Long

    hwnd = CreateTestWindow("StreamClass5")
    hdc = GetDC(hwnd)
    If hdc = 0 Then Err.Raise 501, , "GetDC failed"

    SetTextColor hdc, RGB(0, 128, 255)
    If GetTextColor(hdc) <> RGB(0, 128, 255) Then Err.Raise 502, , "GetTextColor mismatch"

    SetBkMode hdc, TRANSPARENT
    If GetBkMode(hdc) <> TRANSPARENT Then Err.Raise 503, , "GetBkMode mismatch"

    res = TextOutA(hdc, 8, 12, "Streamed Text Run", 17)
    If res = 0 Then Err.Raise 504, , "TextOutA failed"

    res = GetTextExtentPoint32A(hdc, "Streamed Text Run", 17, sz)
    If res = 0 Then Err.Raise 505, , "GetTextExtentPoint32A failed"
    If sz.cx <= 0 Or sz.cy <= 0 Then Err.Raise 506, , "Text extent is empty"

    rc.Left = 0: rc.Top = 0: rc.Right = 0: rc.Bottom = 0
    h = DrawTextA(hdc, "Streamed Text Run", 17, rc, DT_LEFT Or DT_SINGLELINE Or DT_CALCRECT)
    If h <= 0 Then Err.Raise 507, , "DrawTextA(DT_CALCRECT) returned no height"
    If rc.Right - rc.Left <> sz.cx Then Err.Raise 508, , "DT_CALCRECT width " & CStr(rc.Right - rc.Left) & " <> extent " & CStr(sz.cx)

    rc.Left = 8: rc.Top = 40: rc.Right = 300: rc.Bottom = 60
    h = DrawTextA(hdc, "Second Run", 10, rc, DT_LEFT Or DT_SINGLELINE)
    If h <= 0 Then Err.Raise 509, , "DrawTextA render returned no height"

    SetBkMode hdc, OPAQUE
    ReleaseDC hwnd, hdc
    DestroyWindow hwnd
    PrintLine "Test 5: FRAME_TEXT text run attributes and layout extents passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 6: Input frames (mouse.*, key.*, char, resize) -> WM_* message queue
' -----------------------------------------------------------------------------
Private Sub TestInputFrameMessageInjection()
    Dim hwnd As Long
    Dim m As MSG

    hwnd = CreateTestWindow("StreamClass6")
    If hwnd = 0 Then Err.Raise 601, , "Failed to create test window"

    ' Drain anything pending for this window
    Do While PeekMessageA(m, hwnd, 0, 0, PM_REMOVE) <> 0
    Loop

    ' Sequence produced by a browser click, typing "A" + Enter, and a resize
    PostMessageA hwnd, WM_MOUSEMOVE, 0, MakeLParam(40, 25)
    PostMessageA hwnd, WM_LBUTTONDOWN, MK_LBUTTON, MakeLParam(42, 26)
    PostMessageA hwnd, WM_LBUTTONUP, 0, MakeLParam(42, 26)
    PostMessageA hwnd, WM_RBUTTONDOWN, 0, MakeLParam(300, 200)
    PostMessageA hwnd, WM_KEYDOWN, VK_A, 1
    PostMessageA hwnd, WM_CHAR, Asc("A"), 1
    PostMessageA hwnd, WM_KEYUP, VK_A, &HC0000001
    PostMessageA hwnd, WM_KEYDOWN, VK_RETURN, 1
    PostMessageA hwnd, WM_SIZE, SIZE_RESTORED, MakeLParam(640, 480)

    ' Non-destructive peek must see the first message without consuming it
    If PeekMessageA(m, hwnd, 0, 0, PM_NOREMOVE) = 0 Then Err.Raise 602, , "PM_NOREMOVE peek found nothing"
    If m.message <> WM_MOUSEMOVE Then Err.Raise 603, , "PM_NOREMOVE head is not WM_MOUSEMOVE: 0x" & Hex$(m.message)

    ExpectMessage hwnd, WM_MOUSEMOVE, 0, MakeLParam(40, 25), 610
    ExpectMessage hwnd, WM_LBUTTONDOWN, MK_LBUTTON, MakeLParam(42, 26), 620
    ExpectMessage hwnd, WM_LBUTTONUP, 0, MakeLParam(42, 26), 630
    ExpectMessage hwnd, WM_RBUTTONDOWN, 0, MakeLParam(300, 200), 640
    ExpectMessage hwnd, WM_KEYDOWN, VK_A, 1, 650
    ExpectMessage hwnd, WM_CHAR, Asc("A"), 1, 660
    ExpectMessage hwnd, WM_KEYUP, VK_A, &HC0000001, 670
    ExpectMessage hwnd, WM_KEYDOWN, VK_RETURN, 1, 680
    ExpectMessage hwnd, WM_SIZE, SIZE_RESTORED, MakeLParam(640, 480), 690

    If PeekMessageA(m, hwnd, 0, 0, PM_REMOVE) <> 0 Then Err.Raise 699, , "Unexpected extra message 0x" & Hex$(m.message)

    DestroyWindow hwnd
    PrintLine "Test 6: Input frame to WM_* message queue injection and ordering passed."
End Sub
