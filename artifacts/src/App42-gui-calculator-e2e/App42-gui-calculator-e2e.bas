Attribute VB_Name = "Module1"
Option Explicit

' =============================================================================
' ClassicBasic App42: Integrated GUI Web Runner & Interactive Form Lifecycle
' -----------------------------------------------------------------------------
' Phase 3.8 (v0.3.8): End-to-End Interactive Form Lifecycle, Controls & Message Loop
'
' This test fixture exercises the full interactive lifecycle of a multi-control
' Win32/VB6 Calculator Form, verifying:
'   Test 1 -> Form Instantiation & Form_Load Initialization
'   Test 2 -> Interactive Calculation Lifecycle (Addition & Subtraction)
'   Test 3 -> Multiplication, Division, Floating Point & Error State Handling
'   Test 4 -> Keyboard Event Routing & Message Pump Dispatch (WM_KEYDOWN, WM_CHAR)
'   Test 5 -> GDI Invalidation, Damage Tracking & ValidateRect Lifecycle
'   Test 6 -> Complete Form Teardown, Unload & Clean Exit Termination
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

Private Const STD_OUTPUT_HANDLE As Long = -11
Private Const WS_VISIBLE As Long = &H10000000
Private Const WS_OVERLAPPEDWINDOW As Long = &HCF0000

Private Const WM_KEYDOWN As Long = &H100
Private Const WM_KEYUP As Long = &H101
Private Const WM_CHAR As Long = &H102
Private Const PM_REMOVE As Long = &H1
Private Const VK_5 As Long = &H35

Private Declare Function GetStdHandle Lib "kernel32" (ByVal nStdHandle As Long) As Long
Private Declare Function WriteFile Lib "kernel32" (ByVal hFile As Long, ByVal lpBuffer As String, ByVal nNumberOfBytesToWrite As Long, lpNumberOfBytesWritten As Long, ByVal lpOverlapped As Long) As Long

Private Declare Function GetClientRect Lib "user32" (ByVal hwnd As Long, lpRect As RECT) As Long
Private Declare Function GetWindowRect Lib "user32" (ByVal hwnd As Long, lpRect As RECT) As Long
Private Declare Function MoveWindow Lib "user32" (ByVal hwnd As Long, ByVal x As Long, ByVal y As Long, ByVal nWidth As Long, ByVal nHeight As Long, ByVal bRepaint As Long) As Long

Private Declare Function InvalidateRect Lib "user32" (ByVal hwnd As Long, ByVal lpRect As Long, ByVal bErase As Long) As Long
Private Declare Function ValidateRect Lib "user32" (ByVal hwnd As Long, ByVal lpRect As Long) As Long
Private Declare Function GetUpdateRect Lib "user32" (ByVal hwnd As Long, lpRect As RECT, ByVal bErase As Long) As Long

Private Declare Function PostMessageA Lib "user32" (ByVal hwnd As Long, ByVal wMsg As Long, ByVal wParam As Long, ByVal lParam As Long) As Long
Private Declare Function PeekMessageA Lib "user32" (lpMsg As MSG, ByVal hwnd As Long, ByVal wMsgFilterMin As Long, ByVal wMsgFilterMax As Long, ByVal wRemoveMsg As Long) As Long
Private Declare Function GetMessageA Lib "user32" (lpMsg As MSG, ByVal hwnd As Long, ByVal wMsgFilterMin As Long, ByVal wMsgFilterMax As Long) As Long
Private Declare Function TranslateMessage Lib "user32" (lpMsg As MSG) As Long
Private Declare Function DispatchMessageA Lib "user32" (lpMsg As MSG) As Long
Private Declare Function DefWindowProcA Lib "user32" (ByVal hwnd As Long, ByVal wMsg As Long, ByVal wParam As Long, ByVal lParam As Long) As Long
Private Declare Function RegisterClassA Lib "user32" (lpWndClass As WNDCLASSA) As Long
Private Declare Function CreateWindowExA Lib "user32" (ByVal dwExStyle As Long, ByVal lpClassName As String, ByVal lpWindowName As String, ByVal dwStyle As Long, ByVal x As Long, ByVal y As Long, ByVal nWidth As Long, ByVal nHeight As Long, ByVal hWndParent As Long, ByVal hMenu As Long, ByVal hInstance As Long, ByVal lpParam As Long) As Long
Private Declare Function DestroyWindow Lib "user32" (ByVal hwnd As Long) As Long

Public Function WindowProc(ByVal hwnd As Long, ByVal uMsg As Long, ByVal wParam As Long, ByVal lParam As Long) As Long
    WindowProc = DefWindowProcA(hwnd, uMsg, wParam, lParam)
End Function

Private Function GetWndProcAddress(ByVal pfn As Long) As Long
    GetWndProcAddress = pfn
End Function

Private Sub PrintLine(ByVal text As String)
    Dim hStdOut As Long
    Dim written As Long
    Dim msg As String
    
    hStdOut = GetStdHandle(STD_OUTPUT_HANDLE)
    msg = text & vbCrLf
    If hStdOut <> 0 And hStdOut <> -1 Then
        WriteFile hStdOut, msg, Len(msg), written, 0
    Else
        Debug.Print text
    End If
End Sub

Private Function CreateCalcWindow(ByVal className As String, ByVal w As Long, ByVal h As Long) As Long
    Dim wc As WNDCLASSA
    Dim atom As Long
    
    wc.style = 3
    wc.lpfnWndProc = GetWndProcAddress(AddressOf WindowProc)
    wc.hInstance = 0
    wc.lpszClassName = className
    
    atom = RegisterClassA(wc)
    CreateCalcWindow = CreateWindowExA(0, className, "CalcWindow", WS_OVERLAPPEDWINDOW Or WS_VISIBLE, 100, 100, w, h, 0, 0, 0, 0)
End Function

Public Sub Main()
    PrintLine "=== ClassicBasic App42: Integrated GUI Web Runner & Form Lifecycle ==="
    
    TestFormInstantiationAndLoad
    TestCalculationAdditionSubtraction
    TestCalculationMultiplicationDivisionAndError
    TestKeyboardRoutingAndMessagePump
    TestGdiSurfaceDamageAndValidation
    TestFormTeardownAndUnload
    
    PrintLine "=== App42 Test Suite Finished Successfully ==="
    
    ' Interactive GUI Mode for Windows XP desktop exploration
    If InStr(Command$, "--gui") > 0 Or (InStr(Command$, "--test") = 0 And InStr(Command$, "--headless") = 0 And GetStdHandle(STD_OUTPUT_HANDLE) = 0) Then
        Form1.Show vbModal
    End If
End Sub

' -----------------------------------------------------------------------------
' TEST 1: Form Instantiation & Form_Load Initialization
' -----------------------------------------------------------------------------
Private Sub TestFormInstantiationAndLoad()
    Dim rc As RECT
    
    Load Form1
    Form1.Show
    
    If Form1.hwnd = 0 Then Err.Raise 101, , "Form1.hWnd is 0"
    If Form1.Caption <> "ClassicBasic Calculator" Then Err.Raise 102, , "Form1.Caption mismatch: " & Form1.Caption
    If Form1.Text1.Text <> "0" Then Err.Raise 103, , "Form1.Text1.Text init mismatch: " & Form1.Text1.Text
    If Form1.Label1.Caption <> "Ready" Then Err.Raise 104, , "Form1.Label1.Caption init mismatch: " & Form1.Label1.Caption
    
    If GetClientRect(Form1.hwnd, rc) = 0 Then Err.Raise 105, , "GetClientRect(Form1.hWnd) failed"
    If rc.Right <= 0 Or rc.Bottom <= 0 Then Err.Raise 106, , "Form1 client rect dimensions are invalid"
    
    PrintLine "Test 1: Form instantiation and Form_Load initialization passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 2: Basic Calculation Lifecycle (Addition & Subtraction)
' -----------------------------------------------------------------------------
Private Sub TestCalculationAdditionSubtraction()
    Form1.ClearAll
    
    ' 12 + 34 = 46
    Form1.cmd1_Click
    Form1.cmd2_Click
    If Form1.Text1.Text <> "12" Then Err.Raise 201, , "Expected 12 got " & Form1.Text1.Text
    
    Form1.cmdAdd_Click
    Form1.cmd3_Click
    Form1.cmd4_Click
    If Form1.Text1.Text <> "34" Then Err.Raise 202, , "Expected 34 got " & Form1.Text1.Text
    
    Form1.cmdEq_Click
    If Form1.Text1.Text <> "46" Then Err.Raise 203, , "Expected 46 got " & Form1.Text1.Text
    If Form1.Label1.Caption <> "12 + 34 = 46" Then Err.Raise 204, , "Status mismatch: " & Form1.Label1.Caption
    
    ' 46 - 6 = 40
    Form1.cmdSub_Click
    Form1.cmd6_Click
    Form1.cmdEq_Click
    If Form1.Text1.Text <> "40" Then Err.Raise 205, , "Expected 40 got " & Form1.Text1.Text
    If Form1.Label1.Caption <> "46 - 6 = 40" Then Err.Raise 206, , "Status mismatch: " & Form1.Label1.Caption
    
    PrintLine "Test 2: Basic calculation lifecycle (addition and subtraction) passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 3: Multiplication, Division, Floating Point & Error Handling
' -----------------------------------------------------------------------------
Private Sub TestCalculationMultiplicationDivisionAndError()
    Form1.ClearAll
    
    ' 7 / 2 = 3.5
    Form1.cmd7_Click
    Form1.cmdDiv_Click
    Form1.cmd2_Click
    Form1.cmdEq_Click
    If Abs(Val(Form1.Text1.Text) - 3.5) > 0.0001 Then Err.Raise 301, , "Expected 3.5 got " & Form1.Text1.Text
    
    ' 3.5 * 4 = 14
    Form1.cmdMul_Click
    Form1.cmd4_Click
    Form1.cmdEq_Click
    If Abs(Val(Form1.Text1.Text) - 14) > 0.0001 Then Err.Raise 302, , "Expected 14 got " & Form1.Text1.Text
    
    ' 14 / 0 = Error
    Form1.cmdDiv_Click
    Form1.cmd0_Click
    Form1.cmdEq_Click
    If Form1.Text1.Text <> "Error" Then Err.Raise 303, , "Expected Error got " & Form1.Text1.Text
    If Form1.Label1.Caption <> "Division by zero" Then Err.Raise 304, , "Expected Division by zero got " & Form1.Label1.Caption
    
    PrintLine "Test 3: Multiplication, division, floating point, and error handling passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 4: Keyboard Event Routing & Message Pump Dispatch
' -----------------------------------------------------------------------------
Private Sub TestKeyboardRoutingAndMessagePump()
    Dim hwnd As Long
    Dim m As MSG
    Dim keyCount As Long
    Dim charCount As Long
    
    hwnd = CreateCalcWindow("CalcKeyClass", 320, 240)
    If hwnd = 0 Then Err.Raise 401, , "CreateCalcWindow failed"
    
    ' Drain pending
    Do While PeekMessageA(m, hwnd, 0, 0, PM_REMOVE) <> 0
    Loop
    
    ' Post simulated keyboard events
    PostMessageA hwnd, WM_KEYDOWN, VK_5, 0
    PostMessageA hwnd, WM_CHAR, Asc("5"), 0
    PostMessageA hwnd, WM_KEYUP, VK_5, &HC0000001
    
    Do While PeekMessageA(m, hwnd, 0, 0, PM_REMOVE) <> 0
        If m.message = WM_KEYDOWN Then keyCount = keyCount + 1
        If m.message = WM_CHAR Then charCount = charCount + 1
        DispatchMessageA m
    Loop
    
    If keyCount <> 1 Then Err.Raise 402, , "WM_KEYDOWN count mismatch: " & keyCount
    If charCount <> 1 Then Err.Raise 403, , "WM_CHAR count mismatch: " & charCount
    
    DestroyWindow hwnd
    PrintLine "Test 4: Keyboard event routing and message pump dispatch passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 5: GDI Surface Damage Tracking & Validation Lifecycle
' -----------------------------------------------------------------------------
Private Sub TestGdiSurfaceDamageAndValidation()
    Dim hwnd As Long
    Dim rc As RECT
    Dim res As Long
    
    hwnd = CreateCalcWindow("CalcGdiClass", 400, 300)
    If hwnd = 0 Then Err.Raise 501, , "CreateCalcWindow failed"
    
    ' Clear initial damage
    ValidateRect hwnd, 0
    
    ' Invalidate whole client
    InvalidateRect hwnd, 0, 1
    
    ' Verify damage reported
    res = GetUpdateRect(hwnd, rc, 0)
    If res = 0 Then Err.Raise 502, , "GetUpdateRect reported no damage after InvalidateRect"
    If rc.Right <= 0 Or rc.Bottom <= 0 Then Err.Raise 503, , "Invalid damage rect extents: " & rc.Right & "x" & rc.Bottom
    
    ' Validate (clear damage)
    ValidateRect hwnd, 0
    
    ' Verify surface is now clean
    res = GetUpdateRect(hwnd, rc, 0)
    If res <> 0 Then Err.Raise 504, , "GetUpdateRect reported damage after ValidateRect"
    
    DestroyWindow hwnd
    PrintLine "Test 5: GDI surface damage tracking and validation lifecycle passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 6: Form Teardown and Unload Lifecycle
' -----------------------------------------------------------------------------
Private Sub TestFormTeardownAndUnload()
    Unload Form1
    PrintLine "Test 6: Form teardown and unload lifecycle passed."
End Sub
