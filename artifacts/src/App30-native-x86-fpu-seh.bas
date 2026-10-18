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
    PrintLine "=== ClassicBasic App30: Native x86 Execution, x87 FPU & SEH ==="

    TestStackProbeChkstk
    TestX87FpuTranscendentalMath
    TestSehDivisionByZeroTrapping
    TestSehOverflowResumeNext
    TestRecursiveStackUnwinding
    TestFloatByRefByValConventions

    PrintLine "=== App30 Test Suite Finished Successfully ==="
End Sub

' -----------------------------------------------------------------------------
' TEST 1: Stack Probe (__vbaChkstk) with Large Frame (> 8KB)
' -----------------------------------------------------------------------------
Private Sub TestStackProbeChkstk()
    ' A 9000-byte local array forces the compiler to invoke __vbaChkstk
    Dim largeBuffer(1 To 9000) As Byte
    Dim i As Long
    Dim checksum As Long

    For i = 1 To 9000
        largeBuffer(i) = (i Mod 251)
    Next i

    checksum = 0
    For i = 1 To 9000
        checksum = checksum + largeBuffer(i)
    Next i

    If checksum <= 0 Then Err.Raise 101, , "Stack buffer checksum invalid"

    PrintLine "Test 1: Stack probe (__vbaChkstk) passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 2: x87 FPU Transcendental & Arithmetic Math
' -----------------------------------------------------------------------------
Private Sub TestX87FpuTranscendentalMath()
    Dim pi As Double
    Dim s As Double, c As Double, t As Double
    Dim a As Double, e As Double, l As Double, q As Double

    pi = 4# * Atn(1#)
    If Abs(pi - 3.14159265358979) > 0.0000001 Then Err.Raise 201, , "Atn(1) pi mismatch"

    s = Sin(pi / 2#)
    If Abs(s - 1#) > 0.0000001 Then Err.Raise 202, , "Sin(pi/2) mismatch"

    c = Cos(0#)
    If Abs(c - 1#) > 0.0000001 Then Err.Raise 203, , "Cos(0) mismatch"

    t = Tan(pi / 4#)
    If Abs(t - 1#) > 0.0000001 Then Err.Raise 204, , "Tan(pi/4) mismatch"

    e = Exp(1#)
    If Abs(e - 2.718281828459) > 0.000001 Then Err.Raise 205, , "Exp(1) mismatch"

    l = Log(e)
    If Abs(l - 1#) > 0.0000001 Then Err.Raise 206, , "Log(e) mismatch"

    q = Sqr(144#)
    If Abs(q - 12#) > 0.0000001 Then Err.Raise 207, , "Sqr(144) mismatch"

    ' Power operator (x87 fyl2x / f2xm1)
    If Abs((2# ^ 10#) - 1024#) > 0.0000001 Then Err.Raise 208, , "2^10 mismatch"

    PrintLine "Test 2: x87 FPU transcendental math passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 3: SEH Hardware Fault Trapping (Division by Zero) & Resume
' -----------------------------------------------------------------------------
Private Sub TestSehDivisionByZeroTrapping()
    Dim a As Long, b As Long, c As Long
    Dim handled As Boolean
    Dim resumed As Boolean

    handled = False
    resumed = False
    b = 0

    On Error GoTo HandleZero
    c = 100 \ b
    ' Should not reach here initially
    resumed = True
    GoTo Done

HandleZero:
    If Err.Number = 11 Then
        handled = True
        b = 5
        Resume ' Re-execute the faulting instruction with b = 5
    Else
        Err.Raise Err.Number, , "Unexpected error: " & Err.Description
    End If

Done:
    On Error GoTo 0
    If Not handled Then Err.Raise 301, , "Division by zero was not handled"
    If Not resumed Then Err.Raise 302, , "Resume did not re-execute instruction"
    If c <> 20 Then Err.Raise 303, , "Result after resume mismatch: " & CStr(c)

    PrintLine "Test 3: SEH division by zero trapping and resume passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 4: SEH Overflow Trapping & Resume Next
' -----------------------------------------------------------------------------
Private Sub TestSehOverflowResumeNext()
    Dim i As Integer
    Dim stepCount As Long
    Dim caughtOverflow As Boolean

    stepCount = 0
    caughtOverflow = False

    On Error GoTo HandleOverflow
    stepCount = 1
    i = 32767
    i = i + 1 ' Integer overflow
    stepCount = 2
    GoTo Finish

HandleOverflow:
    If Err.Number = 6 Then
        caughtOverflow = True
        Resume Next
    Else
        Err.Raise Err.Number, , "Unexpected error: " & Err.Description
    End If

Finish:
    On Error GoTo 0
    If Not caughtOverflow Then Err.Raise 401, , "Overflow not caught"
    If stepCount <> 2 Then Err.Raise 402, , "Resume Next did not advance to step 2"

    PrintLine "Test 4: SEH overflow and Resume Next passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 5: Deep Recursion Stack Frame & Error Unwinding
' -----------------------------------------------------------------------------
Private Function RecursiveHelper(ByVal depth As Long) As Long
    Dim localPadding(1 To 64) As Long
    localPadding(1) = depth

    If depth >= 20 Then
        Err.Raise 5555, , "Forced recursion unwind"
    End If

    RecursiveHelper = RecursiveHelper(depth + 1) + localPadding(1)
End Function

Private Sub TestRecursiveStackUnwinding()
    Dim caughtUnwind As Boolean
    caughtUnwind = False

    On Error GoTo CatchUnwind
    RecursiveHelper 1
    GoTo FinishedUnwind

CatchUnwind:
    If Err.Number = 5555 Then
        caughtUnwind = True
    End If

FinishedUnwind:
    On Error GoTo 0
    If Not caughtUnwind Then Err.Raise 501, , "Recursive SEH unwind failed"

    PrintLine "Test 5: Recursive stack unwinding passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 6: ByRef vs ByVal Float & Double Calling Conventions
' -----------------------------------------------------------------------------
Private Sub ModifyFloats(ByVal inVal As Double, ByRef outVal As Double, ByVal factor As Single)
    outVal = inVal * CDbl(factor)
End Sub

Private Sub TestFloatByRefByValConventions()
    Dim x As Double
    Dim y As Double
    Dim f As Single

    x = 12.5
    y = 0#
    f = 4#

    ModifyFloats x, y, f
    If Abs(y - 50#) > 0.0000001 Then Err.Raise 601, , "Float calling convention mismatch: " & CStr(y)

    PrintLine "Test 6: Float ByRef/ByVal conventions passed."
End Sub
