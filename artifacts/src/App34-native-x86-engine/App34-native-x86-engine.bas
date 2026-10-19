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
    PrintLine "=== ClassicBasic App34: Native x86 CPU Execution Engine (Instruction Emulator & FS:[0] TIB) ==="

    TestAluArithmeticAndRegisters
    TestCallingConventionsAndStackFrames
    TestBranchingAndLoopConditionals
    TestFpuInstructionPipeline
    TestWin32TibAndSehExceptionFrame
    TestNativeClassMethodDispatch

    PrintLine "=== App34 Test Suite Finished Successfully ==="
End Sub

' -----------------------------------------------------------------------------
' TEST 1: 32-bit ALU Arithmetic, Register Allocation & Bitwise Flags
' -----------------------------------------------------------------------------
Private Sub TestAluArithmeticAndRegisters()
    Dim a As Long, b As Long, c As Long, res As Long
    a = 1000
    b = 350
    c = 75

    ' Exercises EAX, ECX, EDX, EBX register allocation, ADD, SUB, IMUL, IDIV
    res = ((a * 3) + (b * 2) - (c * 4)) \ 5
    ' a * 3 = 3000; b * 2 = 700; c * 4 = 300
    ' (3000 + 700 - 300) = 3400; 3400 \ 5 = 680
    If res <> 680 Then Err.Raise 101, , "ALU arithmetic mismatch: got " & CStr(res)

    ' Bitwise operations: AND, OR, XOR, NOT
    Dim bitResult As Long
    bitResult = (a And &HFF) Or (b And &H0F) Xor &HAA
    ' 1000 = 0x3E8; 1000 And 0xFF = 0xE8 (232)
    ' 350 = 0x15E; 350 And 0x0F = 0x0E (14)
    ' 0xE8 Or 0x0E = 0xEE (238)
    ' 0xEE Xor 0xAA = 0x44 (68)
    If bitResult <> 68 Then Err.Raise 102, , "ALU bitwise mismatch: got " & CStr(bitResult)

    PrintLine "Test 1: 32-bit ALU arithmetic and register operations passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 2: __stdcall Stack Frames, Local Allocation & ByRef Parameter Passing
' -----------------------------------------------------------------------------
Private Sub TestCallingConventionsAndStackFrames()
    Dim val As Long
    val = 50
    ScaleByRef val, 6, 20
    ' (50 * 6) + 20 = 320
    If val <> 320 Then Err.Raise 201, , "ByRef parameter modification mismatch: got " & CStr(val)

    Dim fact As Long
    fact = Factorial(6)
    If fact <> 720 Then Err.Raise 202, , "Recursive call stack preservation mismatch: got " & CStr(fact)

    PrintLine "Test 2: stdcall stack frames and ByRef parameter passing passed."
End Sub

Private Sub ScaleByRef(ByRef target As Long, ByVal multiplier As Long, ByVal addend As Long)
    Dim temp As Long
    temp = (target * multiplier) + addend
    target = temp
End Sub

Private Function Factorial(ByVal n As Long) As Long
    If n <= 1 Then
        Factorial = 1
    Else
        Factorial = n * Factorial(n - 1)
    End If
End Function

' -----------------------------------------------------------------------------
' TEST 3: Native x86 Branching, Loops and Jump Conditionals
' -----------------------------------------------------------------------------
Private Sub TestBranchingAndLoopConditionals()
    Dim i As Long
    Dim sum As Long
    Dim counter As Long

    sum = 0
    For i = 1 To 100
        If (i Mod 2) = 0 Then
            sum = sum + i
        End If
    Next i
    ' Sum of even numbers from 2 to 100 = 2550
    If sum <> 2550 Then Err.Raise 301, , "Loop sum mismatch: got " & CStr(sum)

    ' Do..Loop While branch
    counter = 1
    Do
        counter = counter * 2
    Loop While counter < 1000
    If counter <> 1024 Then Err.Raise 302, , "Do..Loop While mismatch: got " & CStr(counter)

    PrintLine "Test 3: Native x86 branching, loops and jump conditionals passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 4: x87 FPU Instruction Pipeline & Transcendental Math
' -----------------------------------------------------------------------------
Private Sub TestFpuInstructionPipeline()
    Dim x As Double, y As Double, z As Double
    x = 16#
    y = 9#

    ' Tests FLD, FSUB, FMUL, FDIV, FSQRT via Sqr and expressions
    z = Sqr(x) + Sqr(y) + ((x * y) / (x - y))
    ' 4 + 3 + (144 / 7) = 7 + 20.57142857 = 27.57142857
    If Abs(z - 27.5714285714286) > 0.00001 Then Err.Raise 401, , "FPU arithmetic mismatch: got " & CStr(z)

    ' Transcendental: Sin, Cos, Atn
    Dim pi As Double
    pi = 4# * Atn(1#)
    If Abs(pi - 3.14159265358979) > 0.000001 Then Err.Raise 402, , "FPU Atn(1) pi mismatch"

    Dim sinVal As Double
    sinVal = Sin(pi / 6#) ' Sin(30 deg) = 0.5
    If Abs(sinVal - 0.5) > 0.000001 Then Err.Raise 403, , "FPU Sin mismatch: got " & CStr(sinVal)

    PrintLine "Test 4: x87 FPU instruction pipeline and transcendental math passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 5: Win32 FS:[0] TIB & SEH Exception Frame Handling
' -----------------------------------------------------------------------------
Private Sub TestWin32TibAndSehExceptionFrame()
    On Error GoTo CatchZero
    Dim d As Long
    Dim zero As Long
    zero = 0
    d = 500 \ zero
    Err.Raise 501, , "SEH handler was not reached"

CatchZero:
    If Err.Number <> 11 Then Err.Raise 502, , "Expected error 11 (Division by zero)"
    Err.Clear

    ' Test Resume Next via FS:[0] exception unwind
    On Error Resume Next
    d = 1000 \ zero
    If Err.Number <> 11 Then Err.Raise 503, , "Resume Next failed to trap error 11"
    Err.Clear
    On Error GoTo 0

    PrintLine "Test 5: Win32 FS:[0] TIB and SEH exception frame handling passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 6: Native Class Method Dispatch and Object Lifecycle
' -----------------------------------------------------------------------------
Private Sub TestNativeClassMethodDispatch()
    Dim calc As CNativeCompute
    Set calc = New CNativeCompute

    calc.BaseValue = 40
    If calc.BaseValue <> 40 Then Err.Raise 601, , "Class property Let/Get mismatch"

    Dim linear As Long
    linear = calc.ComputeLinear(5, 30)
    ' (40 * 5) + 30 = 230
    If linear <> 230 Then Err.Raise 602, , "Class ComputeLinear mismatch: got " & CStr(linear)

    Dim fpuRes As Double
    fpuRes = calc.EvaluateFpu(10#, 6#)
    ' (10 * 6) + Sqr(4) = 60 + 2 = 62
    If Abs(fpuRes - 62#) > 0.00001 Then Err.Raise 603, , "Class EvaluateFpu mismatch: got " & CStr(fpuRes)

    Dim status As String
    status = calc.FormatStatus("ACTIVE")
    If status <> "ACTIVE: 40" Then Err.Raise 604, , "Class FormatStatus mismatch: got " & status

    Set calc = Nothing
    PrintLine "Test 6: Native class method dispatch and object lifecycle passed."
End Sub
