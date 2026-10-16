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

    Private Sub PrintLine(ByVal sText As String)
        Dim hStdOut As Long
        Dim bytesWritten As Long
        Dim sOut As String

        hStdOut = GetStdHandle(STD_OUTPUT_HANDLE)
        sOut = sText & vbCrLf
        WriteFile hStdOut, sOut, Len(sOut), bytesWritten, 0
    End Sub

    Private Sub Assert(ByVal bCondition As Boolean, ByVal sMsg As String)
        If Not bCondition Then
            PrintLine "ASSERTION FAILED: " & sMsg
            Err.Raise 9999, "Assert", "Assertion failed: " & sMsg
        End If
    End Sub

    Public Sub Main()
        PrintLine "=== ClassicBasic App19: Complex Math & FDIV ==="

        TestBankersRounding
        TestFloatIntDivAndModulo
        TestCurrencyPrecision
        TestTranscendentalAndTraps
        TestVariantDecimalArithmetic
        TestMixedFloatingDivisions

        PrintLine "=== App19 Test Suite Finished Successfully ==="
    End Sub

    ' -------------------------------------------------------------------------
    ' Test 1: Banker's Rounding (Round-to-Nearest-Even)
    ' -------------------------------------------------------------------------
    Private Sub TestBankersRounding()
        ' CInt tie-breaking to nearest even
        Assert CInt(1.5) = 2, "CInt(1.5) must round to 2"
        Assert CInt(2.5) = 2, "CInt(2.5) must round to 2"
        Assert CInt(3.5) = 4, "CInt(3.5) must round to 4"
        Assert CInt(4.5) = 4, "CInt(4.5) must round to 4"
        Assert CInt(-2.5) = -2, "CInt(-2.5) must round to -2"
        Assert CInt(-3.5) = -4, "CInt(-3.5) must round to -4"

        ' CLng tie-breaking
        Assert CLng(1.5) = 2, "CLng(1.5) must round to 2"
        Assert CLng(2.5) = 2, "CLng(2.5) must round to 2"
        Assert CLng(3.5) = 4, "CLng(3.5) must round to 4"
        Assert CLng(4.5) = 4, "CLng(4.5) must round to 4"
        Assert CLng(-2.5) = -2, "CLng(-2.5) must round to -2"

        ' Round() tie-breaking
        Assert Round(1.5) = 2, "Round(1.5) must round to 2"
        Assert Round(2.5) = 2, "Round(2.5) must round to 2"
        Assert Round(3.5) = 4, "Round(3.5) must round to 4"
        Assert Round(4.5) = 4, "Round(4.5) must round to 4"
        Assert Round(-2.5) = -2, "Round(-2.5) must round to -2"

        PrintLine "TEST 1: BANKERS_ROUNDING=PASS"
    End Sub

    ' -------------------------------------------------------------------------
    ' Test 2: Floating point Integer Division '\' and 'Mod' truncation
    ' -------------------------------------------------------------------------
    Private Sub TestFloatIntDivAndModulo()
        ' In VB6, '\' and 'Mod' round operands using Banker's Rounding to Long first
        Dim resIdiv As Long
        Dim resMod As Long

        resIdiv = 10.8 \ 2.4     ' 11 \ 2 = 5
        Assert resIdiv = 5, "10.8 \ 2.4 must equal 5 (11 \ 2)"

        resMod = 10.6 Mod 3.2    ' 11 Mod 3 = 2
        Assert resMod = 2, "10.6 Mod 3.2 must equal 2 (11 Mod 3)"

        ' Test Banker's tie-breaking on division operands:
        ' 11.5 rounds to 12; 2.5 rounds to 2 -> 12 \ 2 = 6
        resIdiv = 11.5 \ 2.5
        Assert resIdiv = 6, "11.5 \ 2.5 must equal 6 (12 \ 2)"

        ' Division by zero trapping (Runtime Error 11)
        On Error Resume Next
        Err.Clear
        resIdiv = 10 \ 0
        Assert Err.Number = 11, "Integer division by zero must raise Error 11"

        Err.Clear
        resMod = 10 Mod 0
        Assert Err.Number = 11, "Mod by zero must raise Error 11"
        On Error GoTo 0

        PrintLine "TEST 2: INTDIV_AND_MOD=PASS"
    End Sub

    ' -------------------------------------------------------------------------
    ' Test 3: Currency 128-bit intermediate precision & overflow protection
    ' -------------------------------------------------------------------------
    Private Sub TestCurrencyPrecision()
        Dim curA As Currency
        Dim curB As Currency
        Dim curRes As Currency

        ' Currency is an 8-byte scaled integer (scaled by 10,000).
        ' Intermediate multiplication without 128-bit precision would overflow 64-bit int.
        curA = 10000@
        curB = 30000@
        curRes = (curA * curB) / 10000@
        Assert curRes = 30000@, "Currency (10000 * 30000) / 10000 must equal 30000"

        ' Currency small fraction preservation
        curA = 1.2345@
        curB = 2.0000@
        curRes = curA * curB
        Assert curRes = 2.469@, "Currency 1.2345 * 2 must equal 2.469"

        PrintLine "TEST 3: CURRENCY_PRECISION=PASS"
    End Sub

    ' -------------------------------------------------------------------------
    ' Test 4: Transcendental math functions and Error traps
    ' -------------------------------------------------------------------------
    Private Sub TestTranscendentalAndTraps()
        Dim dVal As Double
        Dim pi As Double

        ' Basic math functions
        Assert Log(1#) = 0#, "Log(1) must equal 0"
        Assert Exp(0#) = 1#, "Exp(0) must equal 1"
        Assert Sqr(16#) = 4#, "Sqr(16) must equal 4"
        Assert Sqr(0#) = 0#, "Sqr(0) must equal 0"

        ' Trigonometry & Atn
        pi = Atn(1#) * 4#
        Assert Abs(Sin(pi / 2#) - 1#) < 0.000001, "Sin(pi/2) must equal 1"
        Assert Abs(Cos(0#) - 1#) < 0.000001, "Cos(0) must equal 1"
        Assert Abs(Tan(0#)) < 0.000001, "Tan(0) must equal 0"

        ' Error trapping: Log(<= 0) raises Error 5 (Invalid procedure call)
        On Error Resume Next
        Err.Clear
        dVal = Log(0#)
        Assert Err.Number = 5, "Log(0) must raise Runtime Error 5"

        Err.Clear
        dVal = Log(-5#)
        Assert Err.Number = 5, "Log(-5) must raise Runtime Error 5"

        ' Error trapping: Sqr(< 0) raises Error 5
        Err.Clear
        dVal = Sqr(-1#)
        Assert Err.Number = 5, "Sqr(-1) must raise Runtime Error 5"

        ' Error trapping: Exp(large) raises Error 6 (Overflow)
        Err.Clear
        dVal = Exp(800#)
        Assert Err.Number = 6, "Exp(800) must raise Runtime Error 6 (Overflow)"
        On Error GoTo 0

        PrintLine "TEST 4: TRANSCENDENTAL_AND_TRAPS=PASS"
    End Sub

    ' -------------------------------------------------------------------------
    ' Test 5: Large Variant Decimal (VT_DECIMAL) 96-bit integer arithmetic
    ' -------------------------------------------------------------------------
    Private Sub TestVariantDecimalArithmetic()
        Dim d1 As Variant
        Dim d2 As Variant
        Dim dSum As Variant

        ' Decimal Max is 79,228,162,514,264,337,593,543,950,335
        ' We start at Decimal Max - 1
        d1 = CDec("79228162514264337593543950334")
        d2 = CDec(1)

        dSum = d1 + d2
        Assert CStr(dSum) = "79228162514264337593543950335", "Decimal Max addition must preserve 96-bit exact integer"

        ' Adding 1 more must trigger Runtime Error 6 (Overflow)
        On Error Resume Next
        Err.Clear
        dSum = dSum + d2
        Assert Err.Number = 6, "Exceeding Decimal Max must raise Error 6 (Overflow)"
        On Error GoTo 0

        PrintLine "TEST 5: DECIMAL_ARITHMETIC=PASS"
    End Sub

    ' -------------------------------------------------------------------------
    ' Test 6: Mixed floating-point divisions (exercises compiler FDIV paths)
    ' -------------------------------------------------------------------------
    Private Sub TestMixedFloatingDivisions()
        Dim sngVal As Single
        Dim dblVal As Double
        Dim intVal As Integer
        Dim resDbl As Double

        sngVal = 10.5!
        dblVal = 2.5#
        intVal = 2

        ' Single / Double
        resDbl = sngVal / dblVal
        Assert Abs(resDbl - 4.2) < 0.000001, "Single / Double must equal 4.2"

        ' Double / Integer
        resDbl = dblVal / intVal
        Assert Abs(resDbl - 1.25) < 0.000001, "Double / Integer must equal 1.25"

        PrintLine "TEST 6: MIXED_FLOAT_DIV=PASS"
    End Sub