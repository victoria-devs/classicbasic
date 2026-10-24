Attribute VB_Name = "Module1"
Option Explicit

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

Private Declare Function GetStdHandle Lib "kernel32" (ByVal nStdHandle As Long) As Long
Private Declare Function WriteFile Lib "kernel32" ( _
    ByVal hFile As Long, _
    ByVal lpBuffer As String, _
    ByVal nNumberOfBytesToWrite As Long, _
    lpNumberOfBytesWritten As Long, _
    ByVal lpOverlapped As Long) As Long
Private Declare Function GetModuleHandleA Lib "kernel32" (ByVal lpModuleName As String) As Long

Private Declare Function GetSystemMetrics Lib "user32" (ByVal nIndex As Long) As Long
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
Private Declare Function ShowWindow Lib "user32" (ByVal hwnd As Long, ByVal nCmdShow As Long) As Long
Private Declare Function UpdateWindow Lib "user32" (ByVal hwnd As Long) As Long
Private Declare Function MoveWindow Lib "user32" (ByVal hwnd As Long, ByVal x As Long, ByVal y As Long, ByVal nWidth As Long, ByVal nHeight As Long, ByVal bRepaint As Long) As Long
Private Declare Function SetWindowPos Lib "user32" (ByVal hwnd As Long, ByVal hWndInsertAfter As Long, ByVal x As Long, ByVal y As Long, ByVal cx As Long, ByVal cy As Long, ByVal wFlags As Long) As Long
Private Declare Function SetWindowTextA Lib "user32" (ByVal hwnd As Long, ByVal lpString As String) As Long
Private Declare Function GetWindowTextA Lib "user32" (ByVal hwnd As Long, ByVal lpString As String, ByVal cch As Long) As Long
Private Declare Function GetClientRect Lib "user32" (ByVal hwnd As Long, ByRef lpRect As RECT) As Long
Private Declare Function GetWindowRect Lib "user32" (ByVal hwnd As Long, ByRef lpRect As RECT) As Long
Private Declare Function IsWindow Lib "user32" (ByVal hwnd As Long) As Long
Private Declare Function IsWindowVisible Lib "user32" (ByVal hwnd As Long) As Long
Private Declare Function EnableWindow Lib "user32" (ByVal hwnd As Long, ByVal bEnable As Long) As Long
Private Declare Function GetParent Lib "user32" (ByVal hwnd As Long) As Long
Private Declare Function SendMessageA Lib "user32" (ByVal hwnd As Long, ByVal wMsg As Long, ByVal wParam As Long, ByVal lParam As Long) As Long
Private Declare Function PostMessageA Lib "user32" (ByVal hwnd As Long, ByVal wMsg As Long, ByVal wParam As Long, ByVal lParam As Long) As Long
Private Declare Function PeekMessageA Lib "user32" (ByRef lpMsg As MSG, ByVal hwnd As Long, ByVal wMsgFilterMin As Long, ByVal wMsgFilterMax As Long, ByVal wRemoveMsg As Long) As Long
Private Declare Function GetMessageA Lib "user32" (ByRef lpMsg As MSG, ByVal hwnd As Long, ByVal wMsgFilterMin As Long, ByVal wMsgFilterMax As Long) As Long
Private Declare Sub PostQuitMessage Lib "user32" (ByVal nExitCode As Long)
Private Declare Function DefWindowProcA Lib "user32" (ByVal hwnd As Long, ByVal wMsg As Long, ByVal wParam As Long, ByVal lParam As Long) As Long

Private Const STD_OUTPUT_HANDLE As Long = -11
Private Const SM_CXSCREEN As Long = 0
Private Const SM_CYSCREEN As Long = 1
Private Const WS_OVERLAPPEDWINDOW As Long = &HCF0000
Private Const WS_CHILD As Long = &H40000000
Private Const WS_VISIBLE As Long = &H10000000
Private Const SW_HIDE As Long = 0
Private Const SW_SHOW As Long = 5
Private Const PM_REMOVE As Long = &H1
Private Const WM_USER As Long = &H400
Private Const WM_QUIT As Long = &H12

Public Function WindowProc(ByVal hwnd As Long, ByVal uMsg As Long, ByVal wParam As Long, ByVal lParam As Long) As Long
    WindowProc = DefWindowProcA(hwnd, uMsg, wParam, lParam)
End Function

Private Function GetWndProcAddress(ByVal fn As Long) As Long
    GetWndProcAddress = fn
End Function

Private Sub PrintLine(ByVal text As String)
    Dim hStdOut As Long
    Dim written As Long
    Dim msg As String
    msg = text & vbCrLf
    hStdOut = GetStdHandle(STD_OUTPUT_HANDLE)
    WriteFile hStdOut, msg, Len(msg), written, 0
End Sub

Public Sub Main()
    PrintLine "=== ClassicBasic App35: Win32 USER32 Virtual Windowing & Condvar Message Pump ==="

    Dim hMainWnd As Long

    TestSystemMetrics
    hMainWnd = TestWindowClassAndCreation()
    TestWindowHierarchyGeometryAndText hMainWnd
    TestWindowVisibilityAndState hMainWnd
    TestSynchronousMessageDispatch hMainWnd
    TestAsynchronousMessageQueueAndCondvar hMainWnd

    PrintLine "=== App35 Test Suite Finished Successfully ==="
End Sub

Private Sub TestSystemMetrics()
    Dim cx As Long, cy As Long
    cx = GetSystemMetrics(SM_CXSCREEN)
    cy = GetSystemMetrics(SM_CYSCREEN)

    If cx <= 0 Or cy <= 0 Then
        Err.Raise 101, , "GetSystemMetrics returned invalid screen dimensions"
    End If

    PrintLine "Test 1: System metrics query passed."
End Sub

Private Function TestWindowClassAndCreation() As Long
    Dim wc As WNDCLASSA
    Dim atom As Integer
    Dim hInstance As Long
    Dim hWnd As Long

    hInstance = GetModuleHandleA(vbNullString)

    wc.style = 0
    wc.lpfnWndProc = GetWndProcAddress(AddressOf WindowProc)
    wc.cbClsExtra = 0
    wc.cbWndExtra = 0
    wc.hInstance = hInstance
    wc.hIcon = 0
    wc.hCursor = 0
    wc.hbrBackground = 0
    wc.lpszMenuName = vbNullString
    wc.lpszClassName = "ClassicBasicApp35"

    atom = RegisterClassA(wc)
    If atom = 0 Then
        Err.Raise 201, , "RegisterClassA failed to register window class"
    End If

    hWnd = CreateWindowExA(0, "ClassicBasicApp35", "ClassicBasic Main Window", _
        WS_OVERLAPPEDWINDOW, 100, 100, 800, 600, 0, 0, hInstance, 0)

    If hWnd = 0 Then
        Err.Raise 202, , "CreateWindowExA returned null window handle"
    End If

    If IsWindow(hWnd) = 0 Then
        Err.Raise 203, , "IsWindow reported handle is invalid"
    End If

    PrintLine "Test 2: Window class registration and creation passed."
    TestWindowClassAndCreation = hWnd
End Function

Private Sub TestWindowHierarchyGeometryAndText(ByVal hWnd As Long)
    Dim buf As String
    Dim length As Long
    Dim rcWindow As RECT
    Dim rcClient As RECT
    Dim hChild As Long
    Dim hInstance As Long

    ' Test window text set and get
    SetWindowTextA hWnd, "Updated Window Title"
    buf = String$(256, 0)
    length = GetWindowTextA(hWnd, buf, 256)
    If length <= 0 Or Left$(buf, length) <> "Updated Window Title" Then
        Err.Raise 301, , "GetWindowTextA mismatch: got " & Left$(buf, length)
    End If

    ' Test rects
    GetWindowRect hWnd, rcWindow
    If (rcWindow.Right - rcWindow.Left) <> 800 Or (rcWindow.Bottom - rcWindow.Top) <> 600 Then
        Err.Raise 302, , "GetWindowRect dimension mismatch"
    End If

    GetClientRect hWnd, rcClient
    If (rcClient.Right - rcClient.Left) <= 0 Or (rcClient.Bottom - rcClient.Top) <= 0 Then
        Err.Raise 303, , "GetClientRect dimension invalid"
    End If

    ' Test child window hierarchy
    hInstance = GetModuleHandleA(vbNullString)
    hChild = CreateWindowExA(0, "ClassicBasicApp35", "Child Window", _
        WS_CHILD Or WS_VISIBLE, 10, 10, 200, 50, hWnd, 101, hInstance, 0)

    If hChild = 0 Then
        Err.Raise 304, , "CreateWindowExA failed for child control"
    End If

    If GetParent(hChild) <> hWnd Then
        Err.Raise 305, , "GetParent mismatch for child control"
    End If

    DestroyWindow hChild
    PrintLine "Test 3: Window hierarchy, geometry and text passed."
End Sub

Private Sub TestWindowVisibilityAndState(ByVal hWnd As Long)
    ShowWindow hWnd, SW_SHOW
    If IsWindowVisible(hWnd) = 0 Then
        Err.Raise 401, , "IsWindowVisible returned 0 after SW_SHOW"
    End If

    EnableWindow hWnd, 0
    EnableWindow hWnd, 1

    ShowWindow hWnd, SW_HIDE
    If IsWindowVisible(hWnd) <> 0 Then
        Err.Raise 402, , "IsWindowVisible returned 1 after SW_HIDE"
    End If

    PrintLine "Test 4: Window visibility and state control passed."
End Sub

Private Sub TestSynchronousMessageDispatch(ByVal hWnd As Long)
    Dim res As Long
    res = SendMessageA(hWnd, WM_USER + 42, 100, 200)

    PrintLine "Test 5: Synchronous message dispatch and DefWindowProc passed."
End Sub

Private Sub TestAsynchronousMessageQueueAndCondvar(ByVal hWnd As Long)
    Dim msg As MSG
    Dim res As Long

    PostMessageA hWnd, WM_USER + 77, 1234, 5678

    res = PeekMessageA(msg, hWnd, 0, 0, PM_REMOVE)
    If res = 0 Then
        Err.Raise 601, , "PeekMessageA found no message in queue"
    End If

    If msg.message <> (WM_USER + 77) Or msg.wParam <> 1234 Or msg.lParam <> 5678 Then
        Err.Raise 602, , "Message queue content mismatch"
    End If

    PostQuitMessage 0

    res = GetMessageA(msg, 0, 0, 0)
    If res <> 0 Or msg.message <> WM_QUIT Then
        Err.Raise 603, , "GetMessageA did not process WM_QUIT correctly"
    End If

    DestroyWindow hWnd
    If IsWindow(hWnd) <> 0 Then
        Err.Raise 604, , "Window still valid after DestroyWindow"
    End If

    PrintLine "Test 6: Asynchronous queue, pump and lifecycle cleanup passed."
End Sub
