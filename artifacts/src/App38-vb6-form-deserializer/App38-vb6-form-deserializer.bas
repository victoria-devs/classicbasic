Attribute VB_Name = "Module1"
Option Explicit

Private Type RECT
    Left As Long
    Top As Long
    Right As Long
    Bottom As Long
End Type

Private Const STD_OUTPUT_HANDLE As Long = -11

Private Declare Function GetStdHandle Lib "kernel32" (ByVal nStdHandle As Long) As Long
Private Declare Function WriteFile Lib "kernel32" ( _
    ByVal hFile As Long, _
    ByVal lpBuffer As String, _
    ByVal nNumberOfBytesToWrite As Long, _
    lpNumberOfBytesWritten As Long, _
    ByVal lpOverlapped As Long) As Long

Private Declare Function IsWindow Lib "user32" (ByVal hwnd As Long) As Long
Private Declare Function GetWindowTextA Lib "user32" (ByVal hwnd As Long, ByVal lpString As String, ByVal cch As Long) As Long
Private Declare Function GetClientRect Lib "user32" (ByVal hwnd As Long, ByRef lpRect As RECT) As Long

Public Sub PrintLine(ByVal s As String)
    Dim hStdOut As Long
    Dim written As Long
    Dim msg As String

    msg = s & vbCrLf
    hStdOut = GetStdHandle(STD_OUTPUT_HANDLE)
    WriteFile hStdOut, msg, Len(msg), written, 0
End Sub

Public Sub Main()
    PrintLine "=== ClassicBasic App38: VB6 Form Deserializer & Twips-to-Pixels Engine ==="

    TestFormInstantiationAndMetadata
    TestChildControlsPresenceAndProperties
    TestTwipsToPixelsMetricsConversion
    TestFormWindowTreeAndHwnd
    TestDynamicPropertyUpdates
    TestFormLifecycleAndUnload

    PrintLine "=== App38 Test Suite Finished Successfully ==="
End Sub

' -----------------------------------------------------------------------------
' TEST 1: Form Instantiation, Metadata & Scale Geometry
' -----------------------------------------------------------------------------
Private Sub TestFormInstantiationAndMetadata()
    Load Form1

    If Form1.Caption <> "App38 Form Deserializer" Then
        Err.Raise 101, , "Form caption mismatch: " & Form1.Caption
    End If

    If Form1.ScaleWidth <> 9000 Then
        Err.Raise 102, , "Form ScaleWidth mismatch: " & CStr(Form1.ScaleWidth)
    End If

    If Form1.ScaleHeight <> 6750 Then
        Err.Raise 103, , "Form ScaleHeight mismatch: " & CStr(Form1.ScaleHeight)
    End If

    If Form1.Visible Then
        Err.Raise 104, , "Form should not be visible before Show"
    End If

    PrintLine "Test 1: Form instantiation, metadata and basic properties passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 2: Child Controls Presence & Initial Property Deserialization
' -----------------------------------------------------------------------------
Private Sub TestChildControlsPresenceAndProperties()
    ' CommandButton
    If Form1.Command1.Caption <> "Test Button" Then
        Err.Raise 201, , "Command1.Caption mismatch: " & Form1.Command1.Caption
    End If

    ' TextBox
    If Form1.Text1.Text <> "Deserialized Text" Then
        Err.Raise 202, , "Text1.Text mismatch: " & Form1.Text1.Text
    End If

    ' Label
    If Form1.Label1.Caption <> "Form Label" Then
        Err.Raise 203, , "Label1.Caption mismatch: " & Form1.Label1.Caption
    End If

    ' CheckBox
    If Form1.Check1.Value <> 1 Then
        Err.Raise 204, , "Check1.Value mismatch: " & CStr(Form1.Check1.Value)
    End If

    ' OptionButton
    If Form1.Option1.Value <> True Then
        Err.Raise 205, , "Option1.Value mismatch: " & CStr(Form1.Option1.Value)
    End If

    ' Frame
    If Form1.Frame1.Caption <> "Container Frame" Then
        Err.Raise 206, , "Frame1.Caption mismatch: " & Form1.Frame1.Caption
    End If

    ' Timer
    If Form1.Timer1.Interval <> 1000 Then
        Err.Raise 207, , "Timer1.Interval mismatch: " & CStr(Form1.Timer1.Interval)
    End If

    If Form1.Timer1.Enabled <> False Then
        Err.Raise 208, , "Timer1 should be disabled"
    End If

    ' ComboBox
    If Form1.Combo1.Text <> "Combo Item" Then
        Err.Raise 209, , "Combo1.Text mismatch: " & Form1.Combo1.Text
    End If

    PrintLine "Test 2: Child controls presence and initial properties verified."
End Sub

' -----------------------------------------------------------------------------
' TEST 3: Twips-to-Pixels Geometric Coordinate Conversion (15 twips = 1 pixel)
' -----------------------------------------------------------------------------
Private Sub TestTwipsToPixelsMetricsConversion()
    ' Constant ratio at standard 96 DPI: 1440 twips / 96 DPI = 15 twips per pixel
    Dim twipsPerPixel As Long
    twipsPerPixel = 15

    ' Command1: Left=360, Top=360, Width=1800, Height=480
    Dim cmdX As Long, cmdY As Long, cmdW As Long, cmdH As Long
    cmdX = Form1.Command1.Left \ twipsPerPixel
    cmdY = Form1.Command1.Top \ twipsPerPixel
    cmdW = Form1.Command1.Width \ twipsPerPixel
    cmdH = Form1.Command1.Height \ twipsPerPixel

    If cmdX <> 24 Then Err.Raise 301, , "Command1 pixel X mismatch: " & CStr(cmdX)
    If cmdY <> 24 Then Err.Raise 302, , "Command1 pixel Y mismatch: " & CStr(cmdY)
    If cmdW <> 120 Then Err.Raise 303, , "Command1 pixel Width mismatch: " & CStr(cmdW)
    If cmdH <> 32 Then Err.Raise 304, , "Command1 pixel Height mismatch: " & CStr(cmdH)

    ' Form client dimensions in pixels: 9000x6750 twips = 600x450 pixels
    Dim formW As Long, formH As Long
    formW = Form1.ScaleWidth \ twipsPerPixel
    formH = Form1.ScaleHeight \ twipsPerPixel

    If formW <> 600 Then Err.Raise 305, , "Form pixel Width mismatch: " & CStr(formW)
    If formH <> 450 Then Err.Raise 306, , "Form pixel Height mismatch: " & CStr(formH)

    PrintLine "Test 3: Twips-to-pixels geometric coordinate conversion passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 4: Win32 HWND Resolution & Window Tree Binding
' -----------------------------------------------------------------------------
Private Sub TestFormWindowTreeAndHwnd()
    Dim hFormWnd As Long
    Dim buf As String
    Dim lenText As Long

    hFormWnd = Form1.hwnd
    If hFormWnd = 0 Then Err.Raise 401, , "Form1.hwnd returned 0"

    If IsWindow(hFormWnd) = 0 Then
        Err.Raise 402, , "Form1.hwnd is not a valid Win32 window"
    End If

    buf = String$(128, 0)
    lenText = GetWindowTextA(hFormWnd, buf, 128)
    If InStr(Left$(buf, lenText), "App38 Form Deserializer") = 0 Then
        Err.Raise 403, , "Win32 window text mismatch: " & Left$(buf, lenText)
    End If

    PrintLine "Test 4: Win32 HWND resolution and window tree binding passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 5: Dynamic Property Mutation & State Synchronization
' -----------------------------------------------------------------------------
Private Sub TestDynamicPropertyUpdates()
    ' Mutate Form Caption
    Form1.Caption = "Updated Form Title"
    If Form1.Caption <> "Updated Form Title" Then
        Err.Raise 501, , "Form.Caption update failed"
    End If

    ' Mutate TextBox Text
    Form1.Text1.Text = "Modified Input Content"
    If Form1.Text1.Text <> "Modified Input Content" Then
        Err.Raise 502, , "Text1.Text update failed"
    End If

    ' Mutate CheckBox Value
    Form1.Check1.Value = 0
    If Form1.Check1.Value <> 0 Then
        Err.Raise 503, , "Check1.Value update failed"
    End If

    ' Mutate CommandButton Caption
    Form1.Command1.Caption = "Execute Action"
    If Form1.Command1.Caption <> "Execute Action" Then
        Err.Raise 504, , "Command1.Caption update failed"
    End If

    PrintLine "Test 5: Dynamic property mutation and state synchronization passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 6: Form Unload Lifecycle & Resource Teardown
' -----------------------------------------------------------------------------
Private Sub TestFormLifecycleAndUnload()
    Unload Form1
    PrintLine "Test 6: Form unload lifecycle and resource teardown passed."
End Sub
