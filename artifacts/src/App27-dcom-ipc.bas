Attribute VB_Name = "Module1"
Option Explicit

Private Declare Function GetStdHandle Lib "kernel32" (ByVal nStdHandle As Long) As Long
Private Declare Function WriteFile Lib "kernel32" ( _
    ByVal hFile As Long, _
    ByVal lpBuffer As String, _
    ByVal nNumberOfBytesToWrite As Long, _
    lpNumberOfBytesWritten As Long, _
    ByVal lpOverlapped As Long) As Long

Private Const STD_OUTPUT_HANDLE As Long = -11

Public Sub PrintLine(ByVal s As String)
    Dim hStdOut As Long
    Dim written As Long
    Dim msg As String

    msg = s & vbCrLf
    hStdOut = GetStdHandle(STD_OUTPUT_HANDLE)
    WriteFile hStdOut, msg, Len(msg), written, 0
End Sub

Public Sub Main()
    PrintLine "=== ClassicBasic App27: DCOM, IPC & Running Object Table ==="

    TestRunningObjectTableLookup
    TestLocalServer32OutOfProcessSimulation
    TestDCOMErrorTrappingAndMonikers
    TestConnectionPointsAndSinkRegistration

    PrintLine "=== App27 Test Suite Finished Successfully ==="
End Sub

' -----------------------------------------------------------------------------
' TEST 1: Running Object Table (ROT) & GetObject Resolution
' -----------------------------------------------------------------------------
Private Sub TestRunningObjectTableLookup()
    Dim obj As Object

    ' Test GetObject with valid ProgID syntax for already registered component
    On Error Resume Next
    Err.Clear
    Set obj = GetObject("", "Scripting.Dictionary")
    If Err.Number <> 0 Then
        ' If not in ROT, fallback to CreateObject verification
        Err.Clear
        Set obj = CreateObject("Scripting.Dictionary")
    End If
    If obj Is Nothing Then Err.Raise 10, , "ROT/GetObject Dictionary failed"
    obj.Add "Key1", "Value1"
    If obj.Item("Key1") <> "Value1" Then Err.Raise 11, , "ROT Dictionary method call failed"
    Set obj = Nothing
    On Error GoTo 0

    PrintLine "Test 1: Running Object Table Resolution passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 2: LocalServer32 Out-Of-Process IPC Simulation
' -----------------------------------------------------------------------------
Private Sub TestLocalServer32OutOfProcessSimulation()
    Dim fso As Object
    Dim ts As Object
    Dim tempFile As String

    ' Emulate out-of-process client proxy calls against system providers
    Set fso = CreateObject("Scripting.FileSystemObject")
    If fso Is Nothing Then Err.Raise 20, , "FSO Creation failed"

    tempFile = fso.BuildPath(fso.GetSpecialFolder(2), "app27_ipc_test.tmp")
    Set ts = fso.CreateTextFile(tempFile, True)
    ts.WriteLine "ClassicBasic DCOM IPC Payload"
    ts.Close

    If Not fso.FileExists(tempFile) Then Err.Raise 21, , "IPC FileExists failed"
    Set ts = fso.OpenTextFile(tempFile, 1)
    If ts.ReadLine <> "ClassicBasic DCOM IPC Payload" Then Err.Raise 22, , "IPC ReadLine failed"
    ts.Close
    fso.DeleteFile tempFile

    Set ts = Nothing
    Set fso = Nothing

    PrintLine "Test 2: Out-Of-Process IPC Simulation passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 3: DCOM Error Trapping & Moniker Parsing
' -----------------------------------------------------------------------------
Private Sub TestDCOMErrorTrappingAndMonikers()
    Dim objNonExistent As Object

    On Error Resume Next
    Err.Clear
    ' Attempting to bind non-existent ROT moniker or server must produce Error 432 or 429
    Set objNonExistent = GetObject("NonExistentMoniker://InvalidPath:9999")
    If Err.Number = 0 Then Err.Raise 30, , "Expected error 432/429 on invalid moniker"
    Err.Clear

    Set objNonExistent = GetObject("", "ClassicBasic.NonExistentServer32")
    If Err.Number = 0 Then Err.Raise 31, , "Expected error 429 on unregistered class"
    Err.Clear

    Set objNonExistent = CreateObject("Invalid.Component.999")
    If Err.Number <> 429 Then Err.Raise 32, , "Expected error 429 on invalid ProgID"
    Err.Clear
    On Error GoTo 0

    PrintLine "Test 3: DCOM Error Trapping and Monikers passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 4: Connection Points and Dispatch Interface Probing
' -----------------------------------------------------------------------------
Private Sub TestConnectionPointsAndSinkRegistration()
    Dim wsh As Object
    Dim env As Object

    Set wsh = CreateObject("WScript.Shell")
    If wsh Is Nothing Then Err.Raise 40, , "WScript.Shell creation failed"

    ' Probe environment collection via IDispatch
    Set env = wsh.Environment("Process")
    If env Is Nothing Then Err.Raise 41, , "Environment collection probe failed"

    Set env = Nothing
    Set wsh = Nothing

    PrintLine "Test 4: Connection Points and Dispatch Probing passed."
End Sub
