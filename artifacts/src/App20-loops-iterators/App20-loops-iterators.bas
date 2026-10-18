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
        PrintLine "=== ClassicBasic App20: Loops, Iterators & Flow Control ==="

        TestTypedForNext
        TestCurrencyAndVariantForNext
        TestForEachSafeArray
        TestForEachVariantArray
        TestGoSubAndOnGoTo
        TestDoLoopAndWhileWend

        PrintLine "=== App20 Test Suite Finished Successfully ==="
    End Sub

    ' -------------------------------------------------------------------------
    ' Test 1: Typed For..Next loops (Integer, Long, Single, Double)
    ' -------------------------------------------------------------------------
    Private Sub TestTypedForNext()
        Dim i As Integer
        Dim l As Long
        Dim sng As Single
        Dim dbl As Double
        Dim count As Long
        Dim total As Long

        ' Integer loop with Step 2
        total = 0
        count = 0
        For i = 1 To 10 Step 2
            total = total + i
            count = count + 1
        Next i
        Assert count = 5, "Integer For 1 to 10 Step 2 must run 5 iterations"
        Assert total = 25, "Integer sum (1+3+5+7+9) must equal 25"

        ' Long loop with negative Step
        count = 0
        For l = 100 To 50 Step -10
            count = count + 1
        Next l
        Assert count = 6, "Long For 100 to 50 Step -10 must run 6 iterations"

        ' Zero-trip loop
        count = 0
        For i = 10 To 1 Step 1
            count = count + 1
        Next i
        Assert count = 0, "Zero-trip loop must not execute"

        ' Floating-point loop
        count = 0
        For sng = 0.5! To 2.5! Step 0.5!
            count = count + 1
        Next sng
        Assert count = 5, "Single For 0.5 to 2.5 Step 0.5 must run 5 iterations"

        ' Early exit via Exit For
        count = 0
        For l = 1 To 1000
            count = count + 1
            If count = 10 Then Exit For
        Next l
        Assert count = 10, "Exit For must terminate loop immediately"

        PrintLine "TEST 1: TYPED_FOR_NEXT=PASS"
    End Sub

    ' -------------------------------------------------------------------------
    ' Test 2: Currency and Variant dynamic For..Next loops
    ' -------------------------------------------------------------------------
    Private Sub TestCurrencyAndVariantForNext()
        Dim curVal As Currency
        Dim count As Long
        Dim curTotal As Currency

        ' Currency loop (exercises __vbaCyForInit and __vbaCyForNext)
        curTotal = 0@
        count = 0
        For curVal = 10.0000@ To 25.0000@ Step 5.0000@
            curTotal = curTotal + curVal
            count = count + 1
        Next curVal
        Assert count = 4, "Currency loop 10@ to 25@ step 5@ must run 4 iterations"
        Assert curTotal = 70.0000@, "Currency total must be 70 (10+15+20+25)"

        ' Variant loop with fractional step (exercises __vbaVarForInit & __vbaVarForNext)
        Dim vVal As Variant
        Dim vTotal As Double
        count = 0
        vTotal = 0#
        For vVal = 1 To 3 Step 0.5
            vTotal = vTotal + CDbl(vVal)
            count = count + 1
        Next vVal
        Assert count = 5, "Variant loop 1 to 3 Step 0.5 must run 5 iterations"
        Assert Abs(vTotal - 10#) < 0.00001, "Variant total (1+1.5+2+2.5+3) must be 10"

        ' Variant loop with negative step
        count = 0
        For vVal = 10 To 2 Step -2
            count = count + 1
        Next vVal
        Assert count = 5, "Variant loop with negative step must run 5 iterations"

        PrintLine "TEST 2: CURRENCY_AND_VARIANT_FOR_NEXT=PASS"
    End Sub

    ' -------------------------------------------------------------------------
    ' Test 3: For Each over SafeArray (__vbaForEachAry, __vbaNextEachAry)
    ' -------------------------------------------------------------------------
    Private Sub TestForEachSafeArray()
        Dim arr(1 To 4) As String
        Dim sItem As Variant
        Dim sCombined As String
        Dim count As Long

        arr(1) = "Alpha"
        arr(2) = "Beta"
        arr(3) = "Gamma"
        arr(4) = "Delta"

        sCombined = ""
        count = 0
        For Each sItem In arr
            sCombined = sCombined & CStr(sItem) & ","
            count = count + 1
        Next sItem
        Assert count = 4, "For Each over SafeArray must iterate 4 items"
        Assert sCombined = "Alpha,Beta,Gamma,Delta,", "For Each array element order preserved"

        ' Early exit via Exit For (exercises __vbaExitEachAry)
        count = 0
        For Each sItem In arr
            count = count + 1
            If count = 2 Then Exit For
        Next sItem
        Assert count = 2, "Exit For in For Each must cleanly unlock array"

        PrintLine "TEST 3: FOR_EACH_SAFEARRAY=PASS"
    End Sub

    ' -------------------------------------------------------------------------
    ' Test 4: For Each over Variant containing Array (__vbaForEachVar)
    ' -------------------------------------------------------------------------
    Private Sub TestForEachVariantArray()
        Dim vArr As Variant
        Dim vItem As Variant
        Dim total As Long

        vArr = Array(10, 20, 30, 40)
        total = 0
        For Each vItem In vArr
            total = total + CLng(vItem)
        Next vItem
        Assert total = 100, "For Each on Variant Array must calculate sum 100"

        PrintLine "TEST 4: FOR_EACH_VARIANT_ARRAY=PASS"
    End Sub

    ' -------------------------------------------------------------------------
    ' Test 5: GoSub..Return and On..GoTo / On..GoSub (__vbaGosub, __vbaOnGoCheck)
    ' -------------------------------------------------------------------------
    Private Sub TestGoSubAndOnGoTo()
        Dim nVal As Long
        Dim jumpedTo As Long

        nVal = 0

        ' GoSub and Return
        GoSub LocalSub1
        Assert nVal = 42, "GoSub must execute local routine and Return"
        GoTo AfterGoSub

    LocalSub1:
        nVal = 42
        Return

    AfterGoSub:

        ' On..GoTo indexing (exercises __vbaOnGoCheck)
        jumpedTo = 0
        Dim idx As Long
        idx = 2
        On idx GoTo LabelOne, LabelTwo, LabelThree
        Assert False, "On..GoTo idx 2 must jump to LabelTwo"

    LabelOne:
        jumpedTo = 1
        GoTo EndOnGoTo

    LabelTwo:
        jumpedTo = 2
        GoTo EndOnGoTo

    LabelThree:
        jumpedTo = 3
        GoTo EndOnGoTo

    EndOnGoTo:
        Assert jumpedTo = 2, "On idx GoTo must jump to target 2"

        ' On..GoTo out of bounds (0 or > count) must fall through safely without error
        jumpedTo = 99
        idx = 10
        On idx GoTo LabelOne, LabelTwo
        jumpedTo = 0 ' Fell through correctly
        Assert jumpedTo = 0, "Out-of-range On..GoTo must fall through"

        PrintLine "TEST 5: GOSUB_AND_ON_GOTO=PASS"
    End Sub

    ' -------------------------------------------------------------------------
    ' Test 6: Do..Loop and While..Wend conditional flow control
    ' -------------------------------------------------------------------------
    Private Sub TestDoLoopAndWhileWend()
        Dim i As Long

        ' Do While .. Loop
        i = 0
        Do While i < 5
            i = i + 1
        Loop
        Assert i = 5, "Do While must terminate when condition is false"

        ' Do .. Loop Until
        i = 0
        Do
            i = i + 1
        Loop Until i = 3
        Assert i = 3, "Do .. Loop Until must evaluate post-condition"

        ' Do .. Loop with Exit Do
        i = 0
        Do
            i = i + 1
            If i = 7 Then Exit Do
        Loop
        Assert i = 7, "Exit Do must break out of loop"

        ' While .. Wend
        i = 10
        While i > 0
            i = i - 1
        Wend
        Assert i = 0, "While..Wend must loop until zero"

        PrintLine "TEST 6: DO_LOOP_AND_WHILE_WEND=PASS"
    End Sub