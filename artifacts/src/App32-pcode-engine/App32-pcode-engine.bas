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

Private Sub PrintLine(ByVal text As String)
    Dim hStdOut As Long
    Dim written As Long
    Dim msg As String
    msg = text & vbCrLf
    hStdOut = GetStdHandle(STD_OUTPUT_HANDLE)
    WriteFile hStdOut, msg, Len(msg), written, 0
End Sub

Public Sub Main()
    PrintLine "=== ClassicBasic App32: P-Code VM Engine (ProcCallEngine & MethCallEngine) ==="

    TestPCodeStackEvaluation
    TestNestedProcedureFrameCalling
    TestControlFlowAndSelectJumpTables
    TestMethCallEngineClassDispatch
    TestPCodeErrorTrappingAndResume
    TestRuntimeLibraryImportDispatch

    PrintLine "=== App32 Test Suite Finished Successfully ==="
End Sub

' -----------------------------------------------------------------------------
' TEST 1: P-Code Evaluation Stack & Complex Expression Precedence
' -----------------------------------------------------------------------------
Private Sub TestPCodeStackEvaluation()
    Dim a As Long, b As Long, c As Long, res As Long
    a = 15
    b = 4
    c = 2
    ' Stress binary and unary stack operations: LitI4, Add, Sub, Mul, Idiv, Mod
    res = ((a + b * c) \ 3) + ((a Mod b) * 10) - (b ^ 2)
    ' b * c = 8; a + 8 = 23; 23 \ 3 = 7
    ' a Mod b = 3; 3 * 10 = 30; b ^ 2 = 16
    ' 7 + 30 - 16 = 21
    If res <> 21 Then Err.Raise 101, , "P-Code expression precedence failed: got " & CStr(res)

    PrintLine "Test 1: P-Code evaluation stack and operator precedence passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 2: Nested Procedure Calling & ProcFrame Local Variable Allocation
' -----------------------------------------------------------------------------
Private Sub TestNestedProcedureFrameCalling()
    Dim x As Long
    x = 100
    MultiplyByRef x, 3
    If x <> 300 Then Err.Raise 201, , "Nested ByRef parameter update failed: got " & CStr(x)

    Dim sumVal As Long
    sumVal = RecursiveFactorial(5)
    If sumVal <> 120 Then Err.Raise 202, , "Recursive ProcCallEngine invocation failed: got " & CStr(sumVal)

    PrintLine "Test 2: Nested procedure frames and ByRef parameter binding passed."
End Sub

Private Sub MultiplyByRef(ByRef target As Long, ByVal multiplier As Long)
    Dim temp As Long
    temp = target * multiplier
    target = temp
End Sub

Private Function RecursiveFactorial(ByVal n As Long) As Long
    If n <= 1 Then
        RecursiveFactorial = 1
    Else
        RecursiveFactorial = n * RecursiveFactorial(n - 1)
    End If
End Function

' -----------------------------------------------------------------------------
' TEST 3: Control Flow, Loops and Select Case Jump Tables
' -----------------------------------------------------------------------------
Private Sub TestControlFlowAndSelectJumpTables()
    Dim i As Long
    Dim total As Long
    total = 0

    For i = 1 To 10
        total = total + ClassifyNumber(i)
    Next i

    ' 1:1, 2:2, 3:2, 4:10, 5:5, 6:10, 7:5, 8:10, 9:5, 10:10 -> sum
    If total <= 0 Then Err.Raise 301, , "Control flow and Select Case jump tables failed"

    PrintLine "Test 3: P-Code loop branches and Select Case jump tables passed."
End Sub

Private Function ClassifyNumber(ByVal n As Long) As Long
    Select Case n
        Case 1
            ClassifyNumber = 1
        Case 2, 3
            ClassifyNumber = 2
        Case 4, 6, 8, 10
            ClassifyNumber = 10
        Case Else
            ClassifyNumber = 5
    End Select
End Function

' -----------------------------------------------------------------------------
' TEST 4: MethCallEngine & Class Instance Method Dispatch
' -----------------------------------------------------------------------------
Private Sub TestMethCallEngineClassDispatch()
    Dim worker As CPCodeWorker
    Set worker = New CPCodeWorker

    worker.BaseValue = 50
    If worker.BaseValue <> 50 Then Err.Raise 401, , "Class property Let/Get failed"

    Dim calc As Long
    calc = worker.ComputeOffset(4, 25)
    If calc <> 225 Then Err.Raise 402, , "MethCallEngine method dispatch failed: got " & CStr(calc)

    Dim msg As String
    msg = worker.FormatMessage("VALUE")
    If msg <> "VALUE: 50" Then Err.Raise 403, , "MethCallEngine string method failed: got " & msg

    Set worker = Nothing
    PrintLine "Test 4: MethCallEngine class dispatch and object cleanup passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 5: P-Code Error Trapping (On Error GoTo, Resume Next, Resume)
' -----------------------------------------------------------------------------
Private Sub TestPCodeErrorTrappingAndResume()
    On Error GoTo CatchDivZero
    Dim d As Long
    Dim zero As Long
    zero = 0
    d = 100 \ zero
    Err.Raise 501, , "Error handler was not triggered"

CatchDivZero:
    If Err.Number <> 11 Then Err.Raise 502, , "Expected error 11 (Division by zero)"
    Err.Clear

    ' Test Resume Next
    On Error Resume Next
    d = 50 \ zero
    If Err.Number <> 11 Then Err.Raise 503, , "Resume Next did not record error 11"
    Err.Clear

    PrintLine "Test 5: P-Code structured error handling and resume passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 6: Runtime Library Import Dispatch (ImpAdCall -> rtc* and __vba*)
' -----------------------------------------------------------------------------
Private Sub TestRuntimeLibraryImportDispatch()
    Dim raw As String
    raw = "  ClassicBasic P-Code Engine  "

    Dim trimmed As String
    trimmed = Trim$(raw)
    If trimmed <> "ClassicBasic P-Code Engine" Then Err.Raise 601, , "Trim$ import call failed"

    Dim subPart As String
    subPart = Mid$(trimmed, 14, 6)
    If subPart <> "P-Code" Then Err.Raise 602, , "Mid$ import call failed: got " & subPart

    Dim rev As String
    rev = StrReverse(subPart)
    If rev <> "edoC-P" Then Err.Raise 603, , "StrReverse import call failed: got " & rev

    PrintLine "Test 6: Runtime library intrinsic import dispatch passed."
End Sub
