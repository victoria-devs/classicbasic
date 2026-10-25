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

Private Type PAINTSTRUCT
    hdc As Long
    fErase As Long
    rcPaint As RECT
    fRestore As Long
    fIncUpdate As Long
    rgbReserved(0 To 31) As Byte
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
' Win32 API Constants
' -----------------------------------------------------------------------------
Private Const STD_OUTPUT_HANDLE As Long = -11
Private Const WS_OVERLAPPEDWINDOW As Long = &HCF0000

' Stock Objects
Private Const WHITE_BRUSH As Long = 0
Private Const LTGRAY_BRUSH As Long = 1
Private Const GRAY_BRUSH As Long = 2
Private Const DKGRAY_BRUSH As Long = 3
Private Const BLACK_BRUSH As Long = 4
Private Const NULL_BRUSH As Long = 5
Private Const WHITE_PEN As Long = 6
Private Const BLACK_PEN As Long = 7
Private Const NULL_PEN As Long = 8

' Pen Styles
Private Const PS_SOLID As Long = 0
Private Const PS_DASH As Long = 1
Private Const PS_DOT As Long = 2
Private Const PS_NULL As Long = 5

' Raster Operations (ROP)
Private Const SRCCOPY As Long = &HCC0020
Private Const SRCPAINT As Long = &HEE0086
Private Const SRCAND As Long = &H8800C6
Private Const SRCINVERT As Long = &H660046
Private Const WHITENESS As Long = &HFF0062
Private Const BLACKNESS As Long = &H42
Private Const PATCOPY As Long = &HF00021

' -----------------------------------------------------------------------------
' Kernel32 / User32 / GDI32 API Declarations
' -----------------------------------------------------------------------------
Private Declare Function GetStdHandle Lib "kernel32" (ByVal nStdHandle As Long) As Long
Private Declare Function WriteFile Lib "kernel32" ( _
    ByVal hFile As Long, _
    ByVal lpBuffer As String, _
    ByVal nNumberOfBytesToWrite As Long, _
    lpNumberOfBytesWritten As Long, _
    ByVal lpOverlapped As Long) As Long
Private Declare Function GetModuleHandleA Lib "kernel32" (ByVal lpModuleName As String) As Long

' USER32
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
Private Declare Function GetDC Lib "user32" (ByVal hwnd As Long) As Long
Private Declare Function ReleaseDC Lib "user32" (ByVal hwnd As Long, ByVal hdc As Long) As Long
Private Declare Function BeginPaint Lib "user32" (ByVal hwnd As Long, ByRef lpPaint As PAINTSTRUCT) As Long
Private Declare Function EndPaint Lib "user32" (ByVal hwnd As Long, ByRef lpPaint As PAINTSTRUCT) As Long
Private Declare Function InvalidateRect Lib "user32" (ByVal hwnd As Long, ByRef lpRect As RECT, ByVal bErase As Long) As Long
Private Declare Function FillRect Lib "user32" (ByVal hdc As Long, ByRef lpRect As RECT, ByVal hBrush As Long) As Long
Private Declare Function FrameRect Lib "user32" (ByVal hdc As Long, ByRef lpRect As RECT, ByVal hBrush As Long) As Long

' GDI32
Private Declare Function CreateCompatibleDC Lib "gdi32" (ByVal hdc As Long) As Long
Private Declare Function DeleteDC Lib "gdi32" (ByVal hdc As Long) As Long
Private Declare Function CreateCompatibleBitmap Lib "gdi32" (ByVal hdc As Long, ByVal nWidth As Long, ByVal nHeight As Long) As Long
Private Declare Function SelectObject Lib "gdi32" (ByVal hdc As Long, ByVal hgdiobj As Long) As Long
Private Declare Function DeleteObject Lib "gdi32" (ByVal hObject As Long) As Long
Private Declare Function GetStockObject Lib "gdi32" (ByVal fnObject As Long) As Long
Private Declare Function CreatePen Lib "gdi32" (ByVal fnPenStyle As Long, ByVal nWidth As Long, ByVal crColor As Long) As Long
Private Declare Function CreateSolidBrush Lib "gdi32" (ByVal crColor As Long) As Long
Private Declare Function SetPixel Lib "gdi32" (ByVal hdc As Long, ByVal x As Long, ByVal y As Long, ByVal crColor As Long) As Long
Private Declare Function GetPixel Lib "gdi32" (ByVal hdc As Long, ByVal x As Long, ByVal y As Long) As Long
Private Declare Function MoveToEx Lib "gdi32" (ByVal hdc As Long, ByVal x As Long, ByVal y As Long, ByRef lpPoint As POINTAPI) As Long
Private Declare Function LineTo Lib "gdi32" (ByVal hdc As Long, ByVal x As Long, ByVal y As Long) As Long
Private Declare Function Rectangle Lib "gdi32" (ByVal hdc As Long, ByVal nLeft As Long, ByVal nTop As Long, ByVal nRight As Long, ByVal nBottom As Long) As Long
Private Declare Function Ellipse Lib "gdi32" (ByVal hdc As Long, ByVal nLeft As Long, ByVal nTop As Long, ByVal nRight As Long, ByVal nBottom As Long) As Long
Private Declare Function RoundRect Lib "gdi32" (ByVal hdc As Long, ByVal nLeft As Long, ByVal nTop As Long, ByVal nRight As Long, ByVal nBottom As Long, ByVal nWidth As Long, ByVal nHeight As Long) As Long
Private Declare Function PatBlt Lib "gdi32" (ByVal hdc As Long, ByVal nXLeft As Long, ByVal nYLeft As Long, ByVal nWidth As Long, ByVal nHeight As Long, ByVal dwRop As Long) As Long
Private Declare Function BitBlt Lib "gdi32" (ByVal hdcDest As Long, ByVal nXDest As Long, ByVal nYDest As Long, ByVal nWidth As Long, ByVal nHeight As Long, ByVal hdcSrc As Long, ByVal nXSrc As Long, ByVal nYSrc As Long, ByVal dwRop As Long) As Long

Public Sub PrintLine(ByVal s As String)
    Dim hStdOut As Long
    Dim written As Long
    Dim msg As String

    msg = s & vbCrLf
    hStdOut = GetStdHandle(STD_OUTPUT_HANDLE)
    WriteFile hStdOut, msg, Len(msg), written, 0
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
    CreateTestWindow = CreateWindowExA(0, className, "GDI Test Window", WS_OVERLAPPEDWINDOW, 0, 0, 400, 300, 0, 0, hInst, 0)
End Function

Public Sub Main()
    PrintLine "=== ClassicBasic App36: GDI32 32-Bit Surface, Vector Primitives & Dirty Rects ==="

    TestDeviceContextLifecycleAndStockObjects
    TestSolidPensBrushesAndPixelDrawing
    TestVectorDrawingPrimitives
    TestRegionFillsAndFraming
    TestBitmapsMemoryDCsAndRasterOps
    TestInvalidationAndPaintLifecycle

    PrintLine "=== App36 Test Suite Finished Successfully ==="
End Sub

' -----------------------------------------------------------------------------
' TEST 1: Device Context Lifecycle & Stock Objects
' -----------------------------------------------------------------------------
Private Sub TestDeviceContextLifecycleAndStockObjects()
    Dim hwnd As Long
    Dim hdc As Long
    Dim memDC As Long
    Dim hWhite As Long
    Dim hBlackPen As Long
    Dim hNullBrush As Long
    Dim oldObj As Long
    Dim res As Long

    hwnd = CreateTestWindow("GdiTestClass1")
    If hwnd = 0 Then Err.Raise 101, , "Failed to create test window"

    hdc = GetDC(hwnd)
    If hdc = 0 Then Err.Raise 102, , "GetDC failed"

    hWhite = GetStockObject(WHITE_BRUSH)
    If hWhite = 0 Then Err.Raise 103, , "GetStockObject(WHITE_BRUSH) returned 0"

    hBlackPen = GetStockObject(BLACK_PEN)
    If hBlackPen = 0 Then Err.Raise 104, , "GetStockObject(BLACK_PEN) returned 0"

    hNullBrush = GetStockObject(NULL_BRUSH)
    If hNullBrush = 0 Then Err.Raise 105, , "GetStockObject(NULL_BRUSH) returned 0"

    oldObj = SelectObject(hdc, hWhite)
    If oldObj = 0 Then Err.Raise 106, , "SelectObject(hWhite) failed"

    oldObj = SelectObject(hdc, hBlackPen)
    If oldObj = 0 Then Err.Raise 107, , "SelectObject(hBlackPen) failed"

    res = ReleaseDC(hwnd, hdc)
    If res <> 1 Then Err.Raise 108, , "ReleaseDC failed"

    memDC = CreateCompatibleDC(0)
    If memDC = 0 Then Err.Raise 109, , "CreateCompatibleDC(0) failed"

    res = DeleteDC(memDC)
    If res <> 1 Then Err.Raise 110, , "DeleteDC failed"

    DestroyWindow hwnd
    PrintLine "Test 1: Device context lifecycle and stock objects passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 2: Solid Pens, Brushes & Pixel Drawing
' -----------------------------------------------------------------------------
Private Sub TestSolidPensBrushesAndPixelDrawing()
    Dim hwnd As Long
    Dim hdc As Long
    Dim hRedPen As Long
    Dim hGreenBrush As Long
    Dim oldPen As Long
    Dim oldBrush As Long
    Dim readColor As Long
    Dim res As Long

    hwnd = CreateTestWindow("GdiTestClass2")
    hdc = GetDC(hwnd)

    hRedPen = CreatePen(PS_SOLID, 1, RGB(255, 0, 0))
    If hRedPen = 0 Then Err.Raise 201, , "CreatePen failed"

    hGreenBrush = CreateSolidBrush(RGB(0, 255, 0))
    If hGreenBrush = 0 Then Err.Raise 202, , "CreateSolidBrush failed"

    oldPen = SelectObject(hdc, hRedPen)
    oldBrush = SelectObject(hdc, hGreenBrush)

    SetPixel hdc, 25, 25, RGB(0, 0, 255)
    readColor = GetPixel(hdc, 25, 25)
    If readColor <> RGB(0, 0, 255) Then Err.Raise 203, , "GetPixel mismatch at (25, 25): " & CStr(readColor)

    SelectObject hdc, oldPen
    SelectObject hdc, oldBrush
    DeleteObject hRedPen
    DeleteObject hGreenBrush

    ReleaseDC hwnd, hdc
    DestroyWindow hwnd
    PrintLine "Test 2: Solid pens, brushes and pixel operations passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 3: Vector Drawing Primitives (LineTo, Rectangle, Ellipse)
' -----------------------------------------------------------------------------
Private Sub TestVectorDrawingPrimitives()
    Dim hwnd As Long
    Dim hdc As Long
    Dim hPen As Long
    Dim hBrush As Long
    Dim pt As POINTAPI
    Dim cInterior As Long
    Dim cBorder As Long

    hwnd = CreateTestWindow("GdiTestClass3")
    hdc = GetDC(hwnd)

    hPen = CreatePen(PS_SOLID, 1, RGB(255, 0, 0))
    hBrush = CreateSolidBrush(RGB(0, 255, 0))
    SelectObject hdc, hPen
    SelectObject hdc, hBrush

    ' Draw Line from (10, 10) to (30, 10)
    MoveToEx hdc, 10, 10, pt
    LineTo hdc, 30, 10
    cBorder = GetPixel(hdc, 20, 10)
    If cBorder <> RGB(255, 0, 0) Then Err.Raise 301, , "LineTo pixel mismatch at (20, 10): " & CStr(cBorder)

    ' Draw Rectangle [40, 40] to [80, 80]
    Rectangle hdc, 40, 40, 80, 80
    cInterior = GetPixel(hdc, 60, 60)
    If cInterior <> RGB(0, 255, 0) Then Err.Raise 302, , "Rectangle interior pixel mismatch: " & CStr(cInterior)
    cBorder = GetPixel(hdc, 40, 40)
    If cBorder <> RGB(255, 0, 0) Then Err.Raise 303, , "Rectangle border pixel mismatch: " & CStr(cBorder)

    ' Draw Ellipse [100, 100] to [140, 140]
    Ellipse hdc, 100, 100, 140, 140
    cInterior = GetPixel(hdc, 120, 120)
    If cInterior <> RGB(0, 255, 0) Then Err.Raise 304, , "Ellipse center pixel mismatch: " & CStr(cInterior)

    DeleteObject hPen
    DeleteObject hBrush
    ReleaseDC hwnd, hdc
    DestroyWindow hwnd
    PrintLine "Test 3: Vector drawing primitives (LineTo, Rectangle, Ellipse) passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 4: Region Fills & Framing (FillRect, FrameRect, RoundRect)
' -----------------------------------------------------------------------------
Private Sub TestRegionFillsAndFraming()
    Dim hwnd As Long
    Dim hdc As Long
    Dim hBlueBrush As Long
    Dim hYellowBrush As Long
    Dim rcFill As RECT
    Dim rcFrame As RECT
    Dim cFill As Long
    Dim cFrame As Long

    hwnd = CreateTestWindow("GdiTestClass4")
    hdc = GetDC(hwnd)

    hBlueBrush = CreateSolidBrush(RGB(0, 0, 255))
    rcFill.Left = 10
    rcFill.Top = 10
    rcFill.Right = 50
    rcFill.Bottom = 50
    FillRect hdc, rcFill, hBlueBrush
    cFill = GetPixel(hdc, 25, 25)
    If cFill <> RGB(0, 0, 255) Then Err.Raise 401, , "FillRect pixel mismatch: " & CStr(cFill)

    hYellowBrush = CreateSolidBrush(RGB(255, 255, 0))
    rcFrame.Left = 60
    rcFrame.Top = 60
    rcFrame.Right = 100
    rcFrame.Bottom = 100
    FrameRect hdc, rcFrame, hYellowBrush
    cFrame = GetPixel(hdc, 60, 60)
    If cFrame <> RGB(255, 255, 0) Then Err.Raise 402, , "FrameRect border mismatch: " & CStr(cFrame)

    ' RoundRect drawing
    RoundRect hdc, 120, 10, 180, 70, 8, 8
    cFill = GetPixel(hdc, 150, 40)
    If cFill < 0 Then Err.Raise 403, , "RoundRect pixel query failed"

    DeleteObject hBlueBrush
    DeleteObject hYellowBrush
    ReleaseDC hwnd, hdc
    DestroyWindow hwnd
    PrintLine "Test 4: Region fills and framing (FillRect, FrameRect, RoundRect) passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 5: Bitmaps, Memory DCs & Raster Operations (BitBlt, PatBlt)
' -----------------------------------------------------------------------------
Private Sub TestBitmapsMemoryDCsAndRasterOps()
    Dim hwnd As Long
    Dim hdc As Long
    Dim memDC As Long
    Dim hBmp As Long
    Dim oldBmp As Long
    Dim cPixel As Long

    hwnd = CreateTestWindow("GdiTestClass5")
    hdc = GetDC(hwnd)

    memDC = CreateCompatibleDC(hdc)
    hBmp = CreateCompatibleBitmap(hdc, 32, 32)
    If hBmp = 0 Then Err.Raise 501, , "CreateCompatibleBitmap failed"

    oldBmp = SelectObject(memDC, hBmp)
    PatBlt memDC, 0, 0, 32, 32, WHITENESS
    cPixel = GetPixel(memDC, 0, 0)
    If cPixel <> RGB(255, 255, 255) Then Err.Raise 502, , "PatBlt WHITENESS mismatch: " & CStr(cPixel)

    SetPixel memDC, 5, 5, RGB(255, 0, 128)
    BitBlt hdc, 0, 0, 32, 32, memDC, 0, 0, SRCCOPY
    cPixel = GetPixel(hdc, 5, 5)
    If cPixel <> RGB(255, 0, 128) Then Err.Raise 503, , "BitBlt SRCCOPY pixel mismatch: " & CStr(cPixel)

    SelectObject memDC, oldBmp
    DeleteObject hBmp
    DeleteDC memDC
    ReleaseDC hwnd, hdc
    DestroyWindow hwnd
    PrintLine "Test 5: Bitmaps, memory DCs and raster operations (BitBlt, PatBlt) passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 6: Invalidation, Dirty-Rect Tracking & Paint Lifecycle (BeginPaint, EndPaint)
' -----------------------------------------------------------------------------
Private Sub TestInvalidationAndPaintLifecycle()
    Dim hwnd As Long
    Dim rcInv As RECT
    Dim ps As PAINTSTRUCT
    Dim hPaintDC As Long

    hwnd = CreateTestWindow("GdiTestClass6")

    rcInv.Left = 0
    rcInv.Top = 0
    rcInv.Right = 100
    rcInv.Bottom = 100
    InvalidateRect hwnd, rcInv, 1

    hPaintDC = BeginPaint(hwnd, ps)
    If hPaintDC = 0 Then Err.Raise 601, , "BeginPaint returned 0"
    If ps.rcPaint.Right < 100 Or ps.rcPaint.Bottom < 100 Then Err.Raise 602, , "rcPaint invalid bounds"

    EndPaint hwnd, ps
    DestroyWindow hwnd
    PrintLine "Test 6: Window invalidation and paint lifecycle (BeginPaint, EndPaint) passed."
End Sub
