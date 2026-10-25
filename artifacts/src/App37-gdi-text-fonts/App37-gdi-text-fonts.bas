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

Private Type SIZE
    cx As Long
    cy As Long
End Type

Private Type LOGFONTA
    lfHeight As Long
    lfWidth As Long
    lfEscapement As Long
    lfOrientation As Long
    lfWeight As Long
    lfItalic As Byte
    lfUnderline As Byte
    lfStrikeOut As Byte
    lfCharSet As Byte
    lfOutPrecision As Byte
    lfClipPrecision As Byte
    lfQuality As Byte
    lfPitchAndFamily As Byte
    lfFaceName(0 To 31) As Byte
End Type

Private Type TEXTMETRICA
    tmHeight As Long
    tmAscent As Long
    tmDescent As Long
    tmInternalLeading As Long
    tmExternalLeading As Long
    tmAveCharWidth As Long
    tmMaxCharWidth As Long
    tmWeight As Long
    tmOverhang As Long
    tmDigitizedAspectX As Long
    tmDigitizedAspectY As Long
    tmFirstChar As Byte
    tmLastChar As Byte
    tmDefaultChar As Byte
    tmBreakChar As Byte
    tmItalic As Byte
    tmUnderlined As Byte
    tmStruckOut As Byte
    tmPitchAndFamily As Byte
    tmCharSet As Byte
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

' Stock Objects
Private Const WHITE_BRUSH As Long = 0
Private Const BLACK_BRUSH As Long = 4
Private Const WHITE_PEN As Long = 6
Private Const BLACK_PEN As Long = 7

Private Const OEM_FIXED_FONT As Long = 10
Private Const ANSI_FIXED_FONT As Long = 11
Private Const ANSI_VAR_FONT As Long = 12
Private Const SYSTEM_FONT As Long = 13
Private Const DEVICE_DEFAULT_FONT As Long = 14
Private Const DEFAULT_PALETTE As Long = 15
Private Const SYSTEM_FIXED_FONT As Long = 16
Private Const DEFAULT_GUI_FONT As Long = 17

' Font Weights
Private Const FW_DONTCARE As Long = 0
Private Const FW_NORMAL As Long = 400
Private Const FW_BOLD As Long = 700

' Font Charset & Precisions
Private Const ANSI_CHARSET As Long = 0
Private Const DEFAULT_CHARSET As Long = 1
Private Const OUT_DEFAULT_PRECIS As Long = 0
Private Const CLIP_DEFAULT_PRECIS As Long = 0
Private Const DEFAULT_QUALITY As Long = 0
Private Const DEFAULT_PITCH As Long = 0

' BkMode
Private Const TRANSPARENT As Long = 1
Private Const OPAQUE As Long = 2

' ExtTextOut flags
Private Const ETO_OPAQUE As Long = &H2
Private Const ETO_CLIPPED As Long = &H4

' DrawText flags
Private Const DT_TOP As Long = &H0
Private Const DT_LEFT As Long = &H0
Private Const DT_CENTER As Long = &H1
Private Const DT_RIGHT As Long = &H2
Private Const DT_VCENTER As Long = &H4
Private Const DT_BOTTOM As Long = &H8
Private Const DT_WORDBREAK As Long = &H10
Private Const DT_SINGLELINE As Long = &H20
Private Const DT_NOCLIP As Long = &H100
Private Const DT_CALCRECT As Long = &H400

' Raster Operations
Private Const BLACKNESS As Long = &H42
Private Const WHITENESS As Long = &HFF0062

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
Private Declare Function GetDC Lib "user32" (ByVal hwnd As Long) As Long
Private Declare Function ReleaseDC Lib "user32" (ByVal hwnd As Long, ByVal hdc As Long) As Long
Private Declare Function DrawTextA Lib "user32" ( _
    ByVal hdc As Long, _
    ByVal lpchText As String, _
    ByVal nCount As Long, _
    ByRef lpRect As RECT, _
    ByVal uFormat As Long) As Long

' GDI32
Private Declare Function CreateCompatibleDC Lib "gdi32" (ByVal hdc As Long) As Long
Private Declare Function DeleteDC Lib "gdi32" (ByVal hdc As Long) As Long
Private Declare Function CreateCompatibleBitmap Lib "gdi32" (ByVal hdc As Long, ByVal nWidth As Long, ByVal nHeight As Long) As Long
Private Declare Function SelectObject Lib "gdi32" (ByVal hdc As Long, ByVal hgdiobj As Long) As Long
Private Declare Function DeleteObject Lib "gdi32" (ByVal hObject As Long) As Long
Private Declare Function GetStockObject Lib "gdi32" (ByVal fnObject As Long) As Long
Private Declare Function PatBlt Lib "gdi32" (ByVal hdc As Long, ByVal nXLeft As Long, ByVal nYLeft As Long, ByVal nWidth As Long, ByVal nHeight As Long, ByVal dwRop As Long) As Long
Private Declare Function SetPixel Lib "gdi32" (ByVal hdc As Long, ByVal x As Long, ByVal y As Long, ByVal crColor As Long) As Long
Private Declare Function GetPixel Lib "gdi32" (ByVal hdc As Long, ByVal x As Long, ByVal y As Long) As Long

Private Declare Function CreateFontA Lib "gdi32" ( _
    ByVal nHeight As Long, _
    ByVal nWidth As Long, _
    ByVal nEscapement As Long, _
    ByVal nOrientation As Long, _
    ByVal fnWeight As Long, _
    ByVal fdwItalic As Long, _
    ByVal fdwUnderline As Long, _
    ByVal fdwStrikeOut As Long, _
    ByVal fdwCharSet As Long, _
    ByVal fdwOutputPrecision As Long, _
    ByVal fdwClipPrecision As Long, _
    ByVal fdwQuality As Long, _
    ByVal fdwPitchAndFamily As Long, _
    ByVal lpszFace As String) As Long

Private Declare Function CreateFontIndirectA Lib "gdi32" (ByRef lpLogFont As LOGFONTA) As Long

Private Declare Function TextOutA Lib "gdi32" ( _
    ByVal hdc As Long, _
    ByVal x As Long, _
    ByVal y As Long, _
    ByVal lpString As String, _
    ByVal nCount As Long) As Long

Private Declare Function ExtTextOutA Lib "gdi32" ( _
    ByVal hdc As Long, _
    ByVal x As Long, _
    ByVal y As Long, _
    ByVal fuOptions As Long, _
    ByRef lprc As RECT, _
    ByVal lpString As String, _
    ByVal cbCount As Long, _
    ByVal lpDx As Long) As Long

Private Declare Function GetTextExtentPoint32A Lib "gdi32" ( _
    ByVal hdc As Long, _
    ByVal lpString As String, _
    ByVal c As Long, _
    ByRef lpSize As SIZE) As Long

Private Declare Function GetTextMetricsA Lib "gdi32" ( _
    ByVal hdc As Long, _
    ByRef lpMetrics As TEXTMETRICA) As Long

Private Declare Function SetTextColor Lib "gdi32" (ByVal hdc As Long, ByVal crColor As Long) As Long
Private Declare Function GetTextColor Lib "gdi32" (ByVal hdc As Long) As Long
Private Declare Function SetBkColor Lib "gdi32" (ByVal hdc As Long, ByVal crColor As Long) As Long
Private Declare Function GetBkColor Lib "gdi32" (ByVal hdc As Long) As Long
Private Declare Function SetBkMode Lib "gdi32" (ByVal hdc As Long, ByVal iBkMode As Long) As Long
Private Declare Function GetBkMode Lib "gdi32" (ByVal hdc As Long) As Long

' -----------------------------------------------------------------------------
' Helper Functions
' -----------------------------------------------------------------------------
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
    CreateTestWindow = CreateWindowExA(0, className, "Font Test Window", WS_OVERLAPPEDWINDOW, 0, 0, 400, 300, 0, 0, hInst, 0)
End Function

' -----------------------------------------------------------------------------
' Main Entry Point
' -----------------------------------------------------------------------------
Public Sub Main()
    PrintLine "=== ClassicBasic App37: GDI32 Text Output, Font Management & Metrics ==="

    TestFontCreationAndLifecycle
    TestTextMetricsAndExtentQuery
    TestTextColorAndBackgroundMode
    TestBasicTextRenderingTextOut
    TestExtTextOutAndClipping
    TestDrawTextFormattingAndCalcRect

    PrintLine "=== App37 Test Suite Finished Successfully ==="
End Sub

' -----------------------------------------------------------------------------
' TEST 1: Font Creation, Selection, Stock Fonts & Lifecycle
' -----------------------------------------------------------------------------
Private Sub TestFontCreationAndLifecycle()
    Dim hwnd As Long
    Dim hdc As Long
    Dim hFont1 As Long
    Dim hFont2 As Long
    Dim hSysFont As Long
    Dim hGuiFont As Long
    Dim oldFont As Long
    Dim lf As LOGFONTA
    Dim face As String
    Dim i As Long
    Dim res As Long

    hwnd = CreateTestWindow("FontTestClass1")
    If hwnd = 0 Then Err.Raise 101, , "Failed to create test window"

    hdc = GetDC(hwnd)
    If hdc = 0 Then Err.Raise 102, , "GetDC failed"

    ' Stock font queries
    hSysFont = GetStockObject(SYSTEM_FONT)
    If hSysFont = 0 Then Err.Raise 103, , "GetStockObject(SYSTEM_FONT) returned 0"

    hGuiFont = GetStockObject(DEFAULT_GUI_FONT)
    If hGuiFont = 0 Then Err.Raise 104, , "GetStockObject(DEFAULT_GUI_FONT) returned 0"

    ' Explicit font creation via CreateFontA
    hFont1 = CreateFontA(16, 0, 0, 0, FW_BOLD, 0, 0, 0, ANSI_CHARSET, OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, DEFAULT_QUALITY, DEFAULT_PITCH, "Arial")
    If hFont1 = 0 Then Err.Raise 105, , "CreateFontA returned 0"

    oldFont = SelectObject(hdc, hFont1)
    If oldFont = 0 Then Err.Raise 106, , "SelectObject(hFont1) failed"

    ' Font creation via CreateFontIndirectA
    lf.lfHeight = 14
    lf.lfWidth = 0
    lf.lfEscapement = 0
    lf.lfOrientation = 0
    lf.lfWeight = FW_NORMAL
    lf.lfItalic = 0
    lf.lfUnderline = 0
    lf.lfStrikeOut = 0
    lf.lfCharSet = ANSI_CHARSET
    face = "MS Sans Serif"
    For i = 1 To Len(face)
        lf.lfFaceName(i - 1) = Asc(Mid$(face, i, 1))
    Next i
    lf.lfFaceName(Len(face)) = 0

    hFont2 = CreateFontIndirectA(lf)
    If hFont2 = 0 Then Err.Raise 107, , "CreateFontIndirectA returned 0"

    ' Restore original font and delete created fonts
    SelectObject hdc, oldFont
    res = DeleteObject(hFont1)
    If res = 0 Then Err.Raise 108, , "DeleteObject(hFont1) failed"

    res = DeleteObject(hFont2)
    If res = 0 Then Err.Raise 109, , "DeleteObject(hFont2) failed"

    ReleaseDC hwnd, hdc
    DestroyWindow hwnd
    PrintLine "Test 1: Font creation, selection and lifecycle management passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 2: Text Metrics (GetTextMetrics) & String Extent Query (GetTextExtentPoint32)
' -----------------------------------------------------------------------------
Private Sub TestTextMetricsAndExtentQuery()
    Dim hwnd As Long
    Dim hdc As Long
    Dim tm As TEXTMETRICA
    Dim sz As SIZE
    Dim szEmpty As SIZE
    Dim res As Long

    hwnd = CreateTestWindow("FontTestClass2")
    hdc = GetDC(hwnd)

    res = GetTextMetricsA(hdc, tm)
    If res = 0 Then Err.Raise 201, , "GetTextMetricsA returned 0"
    If tm.tmHeight <= 0 Then Err.Raise 202, , "tmHeight <= 0"
    If tm.tmAveCharWidth <= 0 Then Err.Raise 203, , "tmAveCharWidth <= 0"

    res = GetTextExtentPoint32A(hdc, "Hello ClassicBasic", 18, sz)
    If res = 0 Then Err.Raise 204, , "GetTextExtentPoint32A returned 0"
    If sz.cx <= 0 Then Err.Raise 205, , "sz.cx <= 0"
    If sz.cy <= 0 Then Err.Raise 206, , "sz.cy <= 0"

    res = GetTextExtentPoint32A(hdc, "", 0, szEmpty)
    If res = 0 Then Err.Raise 207, , "GetTextExtentPoint32A empty string returned 0"
    If szEmpty.cx <> 0 Then Err.Raise 208, , "Empty string extent cx <> 0: " & CStr(szEmpty.cx)

    ReleaseDC hwnd, hdc
    DestroyWindow hwnd
    PrintLine "Test 2: Text metrics (GetTextMetrics) and string extent calculation passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 3: Text Color, Background Color & Background Mode Control
' -----------------------------------------------------------------------------
Private Sub TestTextColorAndBackgroundMode()
    Dim hwnd As Long
    Dim hdc As Long
    Dim curColor As Long
    Dim curBk As Long
    Dim curMode As Long

    hwnd = CreateTestWindow("FontTestClass3")
    hdc = GetDC(hwnd)

    ' Text Color
    SetTextColor hdc, RGB(255, 128, 0)
    curColor = GetTextColor(hdc)
    If curColor <> RGB(255, 128, 0) Then Err.Raise 301, , "GetTextColor mismatch: " & CStr(curColor)

    ' Background Color
    SetBkColor hdc, RGB(0, 100, 200)
    curBk = GetBkColor(hdc)
    If curBk <> RGB(0, 100, 200) Then Err.Raise 302, , "GetBkColor mismatch: " & CStr(curBk)

    ' Background Mode: TRANSPARENT & OPAQUE
    SetBkMode hdc, TRANSPARENT
    curMode = GetBkMode(hdc)
    If curMode <> TRANSPARENT Then Err.Raise 303, , "GetBkMode TRANSPARENT mismatch: " & CStr(curMode)

    SetBkMode hdc, OPAQUE
    curMode = GetBkMode(hdc)
    If curMode <> OPAQUE Then Err.Raise 304, , "GetBkMode OPAQUE mismatch: " & CStr(curMode)

    ReleaseDC hwnd, hdc
    DestroyWindow hwnd
    PrintLine "Test 3: Text color, background color and background mode control passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 4: Basic Text Rendering (TextOut) onto 32-bit Surface
' -----------------------------------------------------------------------------
Private Sub TestBasicTextRenderingTextOut()
    Dim hwnd As Long
    Dim hdc As Long
    Dim memDC As Long
    Dim hBmp As Long
    Dim oldBmp As Long
    Dim res As Long
    Dim foundWhite As Boolean
    Dim px As Long
    Dim py As Long

    hwnd = CreateTestWindow("FontTestClass4")
    hdc = GetDC(hwnd)
    memDC = CreateCompatibleDC(hdc)
    hBmp = CreateCompatibleBitmap(hdc, 64, 32)
    oldBmp = SelectObject(memDC, hBmp)

    ' Fill with blackness
    PatBlt memDC, 0, 0, 64, 32, BLACKNESS

    SetTextColor memDC, RGB(255, 255, 255)
    SetBkColor memDC, RGB(0, 0, 0)
    SetBkMode memDC, OPAQUE

    res = TextOutA(memDC, 0, 0, "TEST", 4)
    If res = 0 Then Err.Raise 401, , "TextOutA returned 0"

    ' Scan bitmap to confirm glyph pixels were rasterized in white
    foundWhite = False
    For py = 0 To 15
        For px = 0 To 31
            If GetPixel(memDC, px, py) = RGB(255, 255, 255) Then
                foundWhite = True
                Exit For
            End If
        Next px
        If foundWhite Then Exit For
    Next py

    If Not foundWhite Then Err.Raise 402, , "TextOutA did not render expected white pixels"

    SelectObject memDC, oldBmp
    DeleteObject hBmp
    DeleteDC memDC
    ReleaseDC hwnd, hdc
    DestroyWindow hwnd
    PrintLine "Test 4: Basic text rasterization (TextOut) onto bitmap surface passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 5: Advanced Text Output with Background & Clipping (ExtTextOut)
' -----------------------------------------------------------------------------
Private Sub TestExtTextOutAndClipping()
    Dim hwnd As Long
    Dim hdc As Long
    Dim memDC As Long
    Dim hBmp As Long
    Dim oldBmp As Long
    Dim rcClip As RECT
    Dim res As Long
    Dim cPix As Long

    hwnd = CreateTestWindow("FontTestClass5")
    hdc = GetDC(hwnd)
    memDC = CreateCompatibleDC(hdc)
    hBmp = CreateCompatibleBitmap(hdc, 64, 32)
    oldBmp = SelectObject(memDC, hBmp)

    PatBlt memDC, 0, 0, 64, 32, BLACKNESS

    SetBkColor memDC, RGB(0, 0, 255)       ' Blue background
    SetTextColor memDC, RGB(255, 255, 0)   ' Yellow glyphs

    rcClip.Left = 0
    rcClip.Top = 0
    rcClip.Right = 20
    rcClip.Bottom = 16

    res = ExtTextOutA(memDC, 0, 0, ETO_OPAQUE Or ETO_CLIPPED, rcClip, "HI", 2, 0)
    If res = 0 Then Err.Raise 501, , "ExtTextOutA returned 0"

    ' Outside clipped rectangle must remain black (0, 0, 0)
    If GetPixel(memDC, 30, 8) <> RGB(0, 0, 0) Then
        Err.Raise 502, , "Pixel outside ETO_CLIPPED rect was modified: " & CStr(GetPixel(memDC, 30, 8))
    End If

    ' Inside opaque bounding box, pixel should be either blue (bk) or yellow (text)
    cPix = GetPixel(memDC, 18, 14)
    If cPix <> RGB(0, 0, 255) And cPix <> RGB(255, 255, 0) Then
        Err.Raise 503, , "Pixel inside ETO_OPAQUE box was not bk or text color: " & CStr(cPix)
    End If

    SelectObject memDC, oldBmp
    DeleteObject hBmp
    DeleteDC memDC
    ReleaseDC hwnd, hdc
    DestroyWindow hwnd
    PrintLine "Test 5: Advanced text output with background filling and clipping (ExtTextOut) passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 6: Formatted Window Text Layout & Rectangle Calculation (DrawText)
' -----------------------------------------------------------------------------
Private Sub TestDrawTextFormattingAndCalcRect()
    Dim hwnd As Long
    Dim hdc As Long
    Dim memDC As Long
    Dim hBmp As Long
    Dim oldBmp As Long
    Dim rcCalc As RECT
    Dim rcDraw As RECT
    Dim res As Long

    hwnd = CreateTestWindow("FontTestClass6")
    hdc = GetDC(hwnd)
    memDC = CreateCompatibleDC(hdc)
    hBmp = CreateCompatibleBitmap(hdc, 100, 50)
    oldBmp = SelectObject(memDC, hBmp)

    ' 1. DT_CALCRECT calculation
    rcCalc.Left = 0
    rcCalc.Top = 0
    rcCalc.Right = 0
    rcCalc.Bottom = 0

    res = DrawTextA(memDC, "DrawText Test", 13, rcCalc, DT_CALCRECT Or DT_SINGLELINE)
    If res <= 0 Then Err.Raise 601, , "DrawTextA DT_CALCRECT returned <= 0: " & CStr(res)
    If rcCalc.Right <= 0 Or rcCalc.Bottom <= 0 Then
        Err.Raise 602, , "DrawTextA DT_CALCRECT failed to compute positive bounds"
    End If

    ' 2. Centered formatted drawing
    rcDraw.Left = 0
    rcDraw.Top = 0
    rcDraw.Right = 100
    rcDraw.Bottom = 50

    res = DrawTextA(memDC, "Centered", 8, rcDraw, DT_CENTER Or DT_VCENTER Or DT_SINGLELINE)
    If res <= 0 Then Err.Raise 603, , "DrawTextA centered returned <= 0: " & CStr(res)

    SelectObject memDC, oldBmp
    DeleteObject hBmp
    DeleteDC memDC
    ReleaseDC hwnd, hdc
    DestroyWindow hwnd
    PrintLine "Test 6: Formatted text layout and rectangle calculation (DrawText) passed."
End Sub
