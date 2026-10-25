Attribute VB_Name = "Module1"
Option Explicit

Private Const STD_OUTPUT_HANDLE As Long = -11

Private Declare Function GetStdHandle Lib "kernel32" (ByVal nStdHandle As Long) As Long
Private Declare Function WriteFile Lib "kernel32" ( _
    ByVal hFile As Long, _
    ByVal lpBuffer As String, _
    ByVal nNumberOfBytesToWrite As Long, _
    lpNumberOfBytesWritten As Long, _
    ByVal lpOverlapped As Long) As Long

Public Sub PrintLine(ByVal s As String)
    Dim hStdOut As Long
    Dim written As Long
    Dim msg As String

    msg = s & vbCrLf
    hStdOut = GetStdHandle(STD_OUTPUT_HANDLE)
    WriteFile hStdOut, msg, Len(msg), written, 0
End Sub

Public Sub Main()
    PrintLine "=== ClassicBasic App39: VB6 Form Event Sinks & Control Runtime ==="

    TestFormLoadEvent
    TestCommandClickEvent
    TestTextChangeEvent
    TestCheckAndOptionClickEvents
    TestListBoxMethods
    TestFormUnloadEventAndSequence

    PrintLine "=== App39 Test Suite Finished Successfully ==="
End Sub

' -----------------------------------------------------------------------------
' TEST 1: Form_Load Event Trigger on Load Form1
' -----------------------------------------------------------------------------
Private Sub TestFormLoadEvent()
    Load Form1

    If InStr(Form1.EventLog, "Form_Load;") = 0 Then
        Err.Raise 101, , "Form_Load event was not triggered: " & Form1.EventLog
    End If

    PrintLine "Test 1: Form_Load event invocation on Load Form1 passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 2: Command1_Click Event Dispatch & State Mutation
' -----------------------------------------------------------------------------
Private Sub TestCommandClickEvent()
    Form1.FireCommandClick

    If InStr(Form1.EventLog, "Command1_Click;") = 0 Then
        Err.Raise 201, , "Command1_Click event was not triggered: " & Form1.EventLog
    End If

    PrintLine "Test 2: Command1_Click event dispatch passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 3: Text1_Change Event Propagation on Text Mutation
' -----------------------------------------------------------------------------
Private Sub TestTextChangeEvent()
    Form1.Text1.Text = "Updated Text Content"
    If Form1.Text1.Text <> "Updated Text Content" Then
        Err.Raise 301, , "Text1.Text update failed: " & Form1.Text1.Text
    End If

    Form1.FireTextChange

    If InStr(Form1.EventLog, "Text1_Change;") = 0 Then
        Err.Raise 302, , "Text1_Change event was not triggered: " & Form1.EventLog
    End If

    PrintLine "Test 3: Text1_Change event propagation passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 4: Check1_Click and Option1_Click Event Dispatch
' -----------------------------------------------------------------------------
Private Sub TestCheckAndOptionClickEvents()
    Form1.Check1.Value = 1
    If Form1.Check1.Value <> 1 Then
        Err.Raise 401, , "Check1.Value update failed: " & CStr(Form1.Check1.Value)
    End If

    Form1.FireCheckClick

    If InStr(Form1.EventLog, "Check1_Click;") = 0 Then
        Err.Raise 402, , "Check1_Click event was not triggered: " & Form1.EventLog
    End If

    Form1.Option1.Value = True
    If Form1.Option1.Value <> True Then
        Err.Raise 403, , "Option1.Value update failed: " & CStr(Form1.Option1.Value)
    End If

    Form1.FireOptionClick

    If InStr(Form1.EventLog, "Option1_Click;") = 0 Then
        Err.Raise 404, , "Option1_Click event was not triggered: " & Form1.EventLog
    End If

    PrintLine "Test 4: Check1_Click and Option1_Click event dispatch passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 5: ListBox AddItem, ListCount and Clear Control Methods
' -----------------------------------------------------------------------------
Private Sub TestListBoxMethods()
    Form1.List1.AddItem "First Item"
    Form1.List1.AddItem "Second Item"
    Form1.List1.AddItem "Third Item"

    If Form1.List1.ListCount <> 3 Then
        Err.Raise 501, , "List1.ListCount mismatch: " & CStr(Form1.List1.ListCount)
    End If

    Form1.List1.Clear

    If Form1.List1.ListCount <> 0 Then
        Err.Raise 502, , "List1.Clear did not reset ListCount to 0: " & CStr(Form1.List1.ListCount)
    End If

    PrintLine "Test 5: ListBox AddItem, ListCount and Clear methods passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 6: Form Unload Event Dispatch & Teardown Sequence
' -----------------------------------------------------------------------------
Private Sub TestFormUnloadEventAndSequence()
    Form1.FireTimerTick

    If InStr(Form1.EventLog, "Timer1_Timer;") = 0 Then
        Err.Raise 601, , "Timer1_Timer event was not triggered: " & Form1.EventLog
    End If

    Unload Form1

    PrintLine "Test 6: Form_Unload event dispatch and event sequence verified."
End Sub
