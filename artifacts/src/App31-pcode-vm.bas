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
    PrintLine "=== ClassicBasic App31: P-Code VM Interpreter ==="

    TestPCodeArithmeticExpressions
    TestPCodeSelectCaseJumpTables
    TestPCodeControlFlowLoops
    TestProcCallEngineConventions
    TestMethCallEngineDispatch
    TestPCodeErrorTrapping

    PrintLine "=== App31 Test Suite Finished Successfully ==="
End Sub

' -----------------------------------------------------------------------------
' TEST 1: P-Code Arithmetic Expressions & Operator Precedence
' -----------------------------------------------------------------------------
Private Sub TestPCodeArithmeticExpressions()
    Dim x As Long, y As Long, z As Long
    Dim s1 As String, s2 As String, sOut As String

    x = 10
    y = 20
    z = (x + y) * 2 - (y \ x) + (x Mod 3)

    ' (10 + 20) * 2 - (20 \ 10) + (10 Mod 3)
    ' 30 * 2 - 2 + 1 = 60 - 2 + 1 = 59
    If z <> 59 Then Err.Raise 101, , "Arithmetic expression mismatch: " & CStr(z)

    s1 = "Hello "
    s2 = "P-Code!"
    sOut = s1 & s2
    If sOut <> "Hello P-Code!" Then Err.Raise 102, , "String concatenation mismatch"

    PrintLine "Test 1: P-Code arithmetic expressions passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 2: P-Code Select Case Jump Tables (Dense & Sparse)
' -----------------------------------------------------------------------------
Private Function EvalDenseCase(ByVal val As Long) As String
    Select Case val
        Case 1: EvalDenseCase = "ONE"
        Case 2: EvalDenseCase = "TWO"
        Case 3: EvalDenseCase = "THREE"
        Case 4: EvalDenseCase = "FOUR"
        Case 5: EvalDenseCase = "FIVE"
        Case 6: EvalDenseCase = "SIX"
        Case 7: EvalDenseCase = "SEVEN"
        Case 8: EvalDenseCase = "EIGHT"
        Case Else: EvalDenseCase = "OTHER"
    End Select
End Function

Private Function EvalSparseCase(ByVal text As String) As Long
    Select Case text
        Case "Alpha": EvalSparseCase = 100
        Case "Beta": EvalSparseCase = 200
        Case "Gamma": EvalSparseCase = 300
        Case Else: EvalSparseCase = -1
    End Select
End Function

Private Sub TestPCodeSelectCaseJumpTables()
    If EvalDenseCase(1) <> "ONE" Then Err.Raise 201, , "Dense case 1 failed"
    If EvalDenseCase(5) <> "FIVE" Then Err.Raise 202, , "Dense case 5 failed"
    If EvalDenseCase(8) <> "EIGHT" Then Err.Raise 203, , "Dense case 8 failed"
    If EvalDenseCase(99) <> "OTHER" Then Err.Raise 204, , "Dense case Else failed"

    If EvalSparseCase("Alpha") <> 100 Then Err.Raise 205, , "Sparse Alpha failed"
    If EvalSparseCase("Beta") <> 200 Then Err.Raise 206, , "Sparse Beta failed"
    If EvalSparseCase("Unknown") <> -1 Then Err.Raise 207, , "Sparse Else failed"

    PrintLine "Test 2: P-Code Select Case jump tables passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 3: P-Code Control Flow Loops (For, Do While, While Wend)
' -----------------------------------------------------------------------------
Private Sub TestPCodeControlFlowLoops()
    Dim i As Long, sum1 As Long
    Dim count As Long, sum2 As Long
    Dim wCount As Long, sum3 As Long

    ' For Next
    sum1 = 0
    For i = 1 To 10 Step 2
        sum1 = sum1 + i
    Next i
    ' 1 + 3 + 5 + 7 + 9 = 25
    If sum1 <> 25 Then Err.Raise 301, , "For Next sum mismatch: " & CStr(sum1)

    ' Do While Loop
    count = 1
    sum2 = 0
    Do While count <= 5
        sum2 = sum2 + count
        count = count + 1
    Loop
    ' 1 + 2 + 3 + 4 + 5 = 15
    If sum2 <> 15 Then Err.Raise 302, , "Do While sum mismatch: " & CStr(sum2)

    ' While Wend
    wCount = 1
    sum3 = 1
    While wCount <= 4
        sum3 = sum3 * 2
        wCount = wCount + 1
    Wend
    ' 1 * 2 * 2 * 2 * 2 = 16
    If sum3 <> 16 Then Err.Raise 303, , "While Wend product mismatch: " & CStr(sum3)

    PrintLine "Test 3: P-Code control flow loops passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 4: ProcCallEngine Calling Conventions & Parameter Passing
' -----------------------------------------------------------------------------
Private Function SubProcedure(ByVal a As Long, ByRef b As Long, Optional ByVal c As Long = 50) As Long
    b = b + 10
    SubProcedure = a + b + c
End Function

Private Sub TestProcCallEngineConventions()
    Dim valIn As Long
    Dim valRef As Long
    Dim res1 As Long, res2 As Long

    valIn = 100
    valRef = 20
    res1 = SubProcedure(valIn, valRef)
    ' b became 30, res = 100 + 30 + 50 = 180
    If res1 <> 180 Then Err.Raise 401, , "ProcCallEngine default optional failed: " & CStr(res1)
    If valRef <> 30 Then Err.Raise 402, , "ByRef parameter modification failed: " & CStr(valRef)

    res2 = SubProcedure(valIn, valRef, 200)
    ' b became 40, res = 100 + 40 + 200 = 340
    If res2 <> 340 Then Err.Raise 403, , "ProcCallEngine explicit optional failed: " & CStr(res2)
    If valRef <> 40 Then Err.Raise 404, , "ByRef parameter 2nd modification failed"

    PrintLine "Test 4: ProcCallEngine calling conventions passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 5: MethCallEngine Class Dispatching
' -----------------------------------------------------------------------------
Private Sub TestMethCallEngineDispatch()
    Dim helper As CVMHelper
    Dim resAdd As Long, resSub As Long, resMul As Long, resDiv As Long

    Set helper = New CVMHelper
    helper.Tag = "MathVM"

    resAdd = helper.Compute(1, 15, 25)  ' 40
    resSub = helper.Compute(2, 50, 18)  ' 32
    resMul = helper.Compute(3, 7, 8)    ' 56
    resDiv = helper.Compute(4, 100, 4)  ' 25

    If resAdd <> 40 Then Err.Raise 501, , "MethCallEngine Add failed: " & CStr(resAdd)
    If resSub <> 32 Then Err.Raise 502, , "MethCallEngine Sub failed: " & CStr(resSub)
    If resMul <> 56 Then Err.Raise 503, , "MethCallEngine Mul failed: " & CStr(resMul)
    If resDiv <> 25 Then Err.Raise 504, , "MethCallEngine Div failed: " & CStr(resDiv)

    If helper.Tag <> "MathVM" Then Err.Raise 505, , "Property Get mismatch"
    If helper.GetCounter <> 4 Then Err.Raise 506, , "Invocation counter mismatch: " & CStr(helper.GetCounter)

    Set helper = Nothing
    PrintLine "Test 5: MethCallEngine class dispatch passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 6: P-Code Error Trapping (On Error GoTo)
' -----------------------------------------------------------------------------
Private Sub TestPCodeErrorTrapping()
    Dim trapped As Boolean
    Dim stepNum As Long

    trapped = False
    stepNum = 1

    On Error GoTo CatchPCodeErr
    Err.Raise 9999, , "Simulated P-Code Exception"
    stepNum = 2
    GoTo EndTest

CatchPCodeErr:
    If Err.Number = 9999 Then
        trapped = True
    End If

EndTest:
    On Error GoTo 0
    If Not trapped Then Err.Raise 601, , "P-Code error was not trapped"
    If stepNum <> 1 Then Err.Raise 602, , "Execution continued after Err.Raise"

    PrintLine "Test 6: P-Code error trapping passed."
End Sub
