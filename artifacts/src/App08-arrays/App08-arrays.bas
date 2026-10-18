Attribute VB_Name = "Module1"
    Option Explicit

    ' ---------------------------------------------------------------------------
    ' Win32 Console API Declares para saída e asserções padronizadas
    ' ---------------------------------------------------------------------------
    Private Declare Function AllocConsole Lib "kernel32" () As Long
    Private Declare Function GetStdHandle Lib "kernel32" (ByVal nStdHandle As Long) As Long
    Private Declare Function WriteConsoleA Lib "kernel32" ( _
        ByVal hConsoleOutput As Long, _
        ByVal lpBuffer As String, _
        ByVal nNumberOfCharsToWrite As Long, _
        lpNumberOfCharsWritten As Long, _
        lpReserved As Any) As Long

    Private Const STD_OUTPUT_HANDLE As Long = -11
    Private hConsole As Long

    Private Sub ConsolePrint(ByVal sText As String)
        Dim written As Long
        sText = sText & vbCrLf
        WriteConsoleA hConsole, sText, Len(sText), written, ByVal 0&
    End Sub

    Public Sub Main()
        AllocConsole
        hConsole = GetStdHandle(STD_OUTPUT_HANDLE)

        ConsolePrint "=== ClassicBasic App08: Dynamic Arrays & SafeArray Test Suite ==="

        ' -----------------------------------------------------------------------
        ' Teste 1: Alocação Dinâmica com ReDim, Bounds (LBound/UBound) e População
        ' Emite: __vbaAryConstruct2, __vbaRedim, __vbaLbound, __vbaUbound
        ' -----------------------------------------------------------------------
        Dim nums() As Long
        ReDim nums(0 To 4)

        Dim lb As Long, ub As Long
        lb = LBound(nums)
        ub = UBound(nums)

        nums(0) = 10
        nums(1) = 20
        nums(2) = 30
        nums(3) = 40
        nums(4) = 50

        Dim sum As Long, i As Long
        sum = 0
        For i = lb To ub
            sum = sum + nums(i)
        Next i

        If lb = 0 And ub = 4 And sum = 150 Then
            ConsolePrint "TEST 1 PASSED: Dynamic ReDim, LBound=0, UBound=4, Sum=150"
        Else
            ConsolePrint "TEST 1 FAILED: Invalid ReDim bounds or sum"
        End If

        ' -----------------------------------------------------------------------
        ' Teste 2: ReDim Preserve (Expansão mantendo elementos existentes)
        ' Emite: __vbaRedimPreserve
        ' -----------------------------------------------------------------------
        ReDim Preserve nums(0 To 6)
        nums(5) = 60
        nums(6) = 70

        Dim presOk As Boolean
        presOk = (nums(0) = 10 And nums(4) = 50 And nums(5) = 60 And nums(6) = 70)

        If UBound(nums) = 6 And presOk Then
            ConsolePrint "TEST 2 PASSED: ReDim Preserve expanded to UBound=6 preserving data"
        Else
            ConsolePrint "TEST 2 FAILED: ReDim Preserve corrupted data"
        End If

        ' -----------------------------------------------------------------------
        ' Teste 3: Split e Join de Strings (rtcSplit, rtcJoin)
        ' -----------------------------------------------------------------------
        Dim rawText As String
        rawText = "classic,basic,linux,rust"

        Dim tokens() As String
        tokens = Split(rawText, ",")

        Dim tokenCount As Long
        tokenCount = UBound(tokens) - LBound(tokens) + 1

        Dim reconstructed As String
        reconstructed = Join(tokens, "-")

        If tokenCount = 4 And tokens(0) = "classic" And tokens(3) = "rust" And reconstructed = "classic-basic-linux-rust" Then
            ConsolePrint "TEST 3 PASSED: Split parsed 4 tokens and Join rebuilt string"
        Else
            ConsolePrint "TEST 3 FAILED: Split/Join mismatch"
        End If

        ' -----------------------------------------------------------------------
        ' Teste 4: Filter (rtcFilter)
        ' -----------------------------------------------------------------------
        Dim filtered() As String
        filtered = Filter(tokens, "linux")

        If UBound(filtered) = 0 And filtered(0) = "linux" Then
            ConsolePrint "TEST 4 PASSED: Filter matched 1 token ('linux')"
        Else
            ConsolePrint "TEST 4 FAILED: Filter did not match target token"
        End If

        ' -----------------------------------------------------------------------
        ' Teste 5: Array() em Variant (rtcArray, VT_ARRAY | VT_VARIANT)
        ' -----------------------------------------------------------------------
        Dim vList As Variant
        vList = Array(100, 200, 300)

        If LBound(vList) = 0 And UBound(vList) = 2 And vList(1) = 200 Then
            ConsolePrint "TEST 5 PASSED: Variant Array() populated and indexed correctly"
        Else
            ConsolePrint "TEST 5 FAILED: Variant Array() mismatch"
        End If

        ' -----------------------------------------------------------------------
        ' Teste 6: Erase (Desalocação do SafeArray)
        ' Emite: __vbaErase
        ' -----------------------------------------------------------------------
        Erase nums
        ConsolePrint "TEST 6 PASSED: Erase executed without exceptions"

        ConsolePrint "=== App08 Test Suite Finished Successfully ==="
    End Sub

