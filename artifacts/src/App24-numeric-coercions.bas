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

Private Sub PrintLine(ByVal s As String)
    Dim hStdOut As Long
    Dim written As Long
    Dim msg As String

    msg = s & vbCrLf
    hStdOut = GetStdHandle(STD_OUTPUT_HANDLE)
    WriteFile hStdOut, msg, Len(msg), written, 0
End Sub

Public Sub Main()
    PrintLine "=== ClassicBasic App24: Numeric Intrinsics & Primitive Coercions ==="

    TestSignedAndAbsoluteValue
    TestCurrencyMathAndConversions
    TestFloatingPointAndDateConversions
    TestErrorVariantAndTypeCoercions

    PrintLine "=== App24 Test Suite Finished Successfully ==="
End Sub

' -----------------------------------------------------------------------------
' TEST 1: Signed and Absolute Value Helpers (__vbaI2Abs, __vbaI4Abs, __vbaCyAbs,
'         __vbaI2Sgn, __vbaI4Sgn, __vbaUI1Sgn, __vbaCySgn, __vbaR4Sgn, __vbaR8Sgn)
' -----------------------------------------------------------------------------
Private Sub TestSignedAndAbsoluteValue()
    Dim i2 As Integer
    Dim i4 As Long
    Dim cyVal As Currency
    Dim bVal As Byte
    Dim r4 As Single
    Dim r8 As Double

    ' Abs tests
    i2 = -1234
    If Abs(i2) <> 1234 Then Err.Raise 1, , "I2Abs failed"

    i4 = -987654321
    If Abs(i4) <> 987654321 Then Err.Raise 2, , "I4Abs failed"

    cyVal = -1234.5678@
    If Abs(cyVal) <> 1234.5678@ Then Err.Raise 3, , "CyAbs failed"

    ' Sgn tests
    i2 = -42
    If Sgn(i2) <> -1 Then Err.Raise 4, , "I2Sgn negative failed"
    i2 = 0
    If Sgn(i2) <> 0 Then Err.Raise 5, , "I2Sgn zero failed"
    i2 = 42
    If Sgn(i2) <> 1 Then Err.Raise 6, , "I2Sgn positive failed"

    i4 = -100000
    If Sgn(i4) <> -1 Then Err.Raise 7, , "I4Sgn negative failed"
    i4 = 100000
    If Sgn(i4) <> 1 Then Err.Raise 8, , "I4Sgn positive failed"

    bVal = 15
    If Sgn(bVal) <> 1 Then Err.Raise 9, , "UI1Sgn failed"
    bVal = 0
    If Sgn(bVal) <> 0 Then Err.Raise 10, , "UI1Sgn zero failed"

    cyVal = -50.25@
    If Sgn(cyVal) <> -1 Then Err.Raise 11, , "CySgn failed"
    cyVal = 50.25@
    If Sgn(cyVal) <> 1 Then Err.Raise 12, , "CySgn positive failed"

    r4 = -3.14!
    If Sgn(r4) <> -1 Then Err.Raise 13, , "R4Sgn failed"
    r4 = 3.14!
    If Sgn(r4) <> 1 Then Err.Raise 14, , "R4Sgn positive failed"

    r8 = -2.718281828459
    If Sgn(r8) <> -1 Then Err.Raise 15, , "R8Sgn failed"
    r8 = 2.718281828459
    If Sgn(r8) <> 1 Then Err.Raise 16, , "R8Sgn positive failed"

    PrintLine "Test 1: Signed and Absolute Value passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 2: Currency Math and Conversions (__vbaCyFix, __vbaCyInt, __vbaCySub,
'         __vbaCyI2, __vbaCyI4, __vbaCyUI1, __vbaI2Cy, __vbaI4Cy, __vbaR4Cy,
'         __vbaUI1Cy, __vbaFpCmpCy)
' -----------------------------------------------------------------------------
Private Sub TestCurrencyMathAndConversions()
    Dim cy1 As Currency
    Dim cy2 As Currency
    Dim cyRes As Currency
    Dim i2 As Integer
    Dim i4 As Long
    Dim b As Byte
    Dim r4 As Single

    cy1 = -123.4567@
    If Fix(cy1) <> -123@ Then Err.Raise 20, , "CyFix failed"
    If Int(cy1) <> -124@ Then Err.Raise 21, , "CyInt failed"

    cy1 = 500.50@
    cy2 = 125.25@
    cyRes = cy1 - cy2
    If cyRes <> 375.25@ Then Err.Raise 22, , "CySub failed"

    ' Coercion to Currency from primitives
    i2 = 1234
    cyRes = CCur(i2)
    If cyRes <> 1234@ Then Err.Raise 23, , "CyI2 failed"

    i4 = 987654
    cyRes = CCur(i4)
    If cyRes <> 987654@ Then Err.Raise 24, , "CyI4 failed"

    b = 250
    cyRes = CCur(b)
    If cyRes <> 250@ Then Err.Raise 25, , "CyUI1 failed"

    ' Coercion from Currency to primitives
    cy1 = 4567.89@
    i2 = CInt(cy1)
    If i2 <> 4568 Then Err.Raise 26, , "I2Cy failed"

    i4 = CLng(cy1)
    If i4 <> 4568 Then Err.Raise 27, , "I4Cy failed"

    r4 = CSng(cy1)
    If Abs(r4 - 4567.89!) > 0.01! Then Err.Raise 28, , "R4Cy failed"

    cy1 = 200.4@
    b = CByte(cy1)
    If b <> 200 Then Err.Raise 29, , "UI1Cy failed"

    ' Currency comparisons
    cy1 = 100.50@
    cy2 = 100.50@
    If Not (cy1 = cy2) Then Err.Raise 30, , "FpCmpCy equality failed"
    cy2 = 200.75@
    If Not (cy1 < cy2) Then Err.Raise 31, , "FpCmpCy less than failed"
    If Not (cy2 > cy1) Then Err.Raise 32, , "FpCmpCy greater than failed"

    PrintLine "Test 2: Currency Math and Conversions passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 3: Floating Point and Date Conversions (__vbaDateR4, __vbaDateR8,
'         __vbaFpCDblR4, __vbaFpCDblR8, __vbaFpCSngR4, __vbaFpCSngR8,
'         __vbaFpUI1, __vbaPowerR8, __vbaR8FixI2, __vbaR8FixI4, __vbaR8IntI2,
'         __vbaR8IntI4)
' -----------------------------------------------------------------------------
Private Sub TestFloatingPointAndDateConversions()
    Dim dt As Date
    Dim r4 As Single
    Dim r8 As Double
    Dim b As Byte
    Dim i2 As Integer
    Dim i4 As Long

    dt = #1/1/2000 12:00:00 PM#
    r8 = CDbl(dt)
    If Abs(r8 - 36526.5) > 0.0001 Then Err.Raise 40, , "DateR8 failed"

    r4 = CSng(dt)
    If Abs(r4 - 36526.5!) > 0.1! Then Err.Raise 41, , "DateR4 failed"

    ' Float widening & narrowing
    r4 = 12.34!
    r8 = CDbl(r4)
    If Abs(r8 - 12.34) > 0.01 Then Err.Raise 42, , "FpCDblR4 failed"

    r8 = 987.654321
    r4 = CSng(r8)
    If Abs(r4 - 987.654!) > 0.01! Then Err.Raise 43, , "FpCSngR8 failed"

    r8 = 254.6
    b = CByte(r8)
    If b <> 255 Then Err.Raise 44, , "FpUI1 failed"

    ' Power R8
    r8 = 2# ^ 10#
    If r8 <> 1024# Then Err.Raise 45, , "PowerR8 failed"
    r8 = 9# ^ 0.5#
    If r8 <> 3# Then Err.Raise 46, , "PowerR8 sqrt failed"

    ' R8 Fix and Int into integers
    r8 = -15.75
    i2 = Fix(r8)
    If i2 <> -15 Then Err.Raise 47, , "R8FixI2 failed"
    i2 = Int(r8)
    If i2 <> -16 Then Err.Raise 48, , "R8IntI2 failed"

    r8 = -123456.78
    i4 = Fix(r8)
    If i4 <> -123456 Then Err.Raise 49, , "R8FixI4 failed"
    i4 = Int(r8)
    If i4 <> -123457 Then Err.Raise 50, , "R8IntI4 failed"

    PrintLine "Test 3: Floating Point and Date Conversions passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 4: Error Variant Coercions and Trappings (__vbaBoolErrVar, __vbaCyErrVar,
'         __vbaI2ErrVar, __vbaR4ErrVar, __vbaUI1ErrVar, __vbaUI1I4, __vbaUI1Var,
'         __vbaVarDateVar)
' -----------------------------------------------------------------------------
Private Sub TestErrorVariantAndTypeCoercions()
    Dim vErr As Variant
    Dim bBool As Boolean
    Dim cyVal As Currency
    Dim i2 As Integer
    Dim r4 As Single
    Dim bUI1 As Byte
    Dim i4 As Long
    Dim vVal As Variant
    Dim dt As Date

    ' Long to Byte conversion
    i4 = 127
    bUI1 = CByte(i4)
    If bUI1 <> 127 Then Err.Raise 60, , "UI1I4 failed"

    ' Variant to Byte conversion
    vVal = 210
    bUI1 = CByte(vVal)
    If bUI1 <> 210 Then Err.Raise 61, , "UI1Var failed"

    ' Variant to Date conversion
    vVal = "2026-10-17 14:00:00"
    dt = CDate(vVal)
    If Year(dt) <> 2026 Or Month(dt) <> 10 Or Day(dt) <> 17 Then
        Err.Raise 62, , "VarDateVar failed"
    End If

    ' Error variant coercion with error trapping
    vErr = CVErr(5)
    On Error Resume Next
    Err.Clear

    bBool = CBool(vErr)
    If Err.Number <> 13 Then Err.Raise 63, , "BoolErrVar expected Type Mismatch"
    Err.Clear

    cyVal = CCur(vErr)
    If Err.Number <> 13 Then Err.Raise 64, , "CyErrVar expected Type Mismatch"
    Err.Clear

    i2 = CInt(vErr)
    If Err.Number <> 13 Then Err.Raise 65, , "I2ErrVar expected Type Mismatch"
    Err.Clear

    r4 = CSng(vErr)
    If Err.Number <> 13 Then Err.Raise 66, , "R4ErrVar expected Type Mismatch"
    Err.Clear

    bUI1 = CByte(vErr)
    If Err.Number <> 13 Then Err.Raise 67, , "UI1ErrVar expected Type Mismatch"
    Err.Clear
    On Error GoTo 0

    PrintLine "Test 4: Error Variant and Type Coercions passed."
End Sub
