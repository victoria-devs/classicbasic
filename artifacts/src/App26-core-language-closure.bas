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
    PrintLine "=== ClassicBasic App26: Dynamic Arrays, VarArgs & Core Language 100% Closure ==="

    TestVarArgsAndParamArray
    TestDynamicVariantArrays
    TestByteStringsAndAdvancedStringOps
    TestVariantLikePatternMatching
    TestTypeCheckingAndControlFlow

    PrintLine "=== App26 Test Suite Finished Successfully ==="
End Sub

' -----------------------------------------------------------------------------
' TEST 1: ParamArray & VarArgs Helpers (__vbaAryVarVarg, __vbaFreeVarg,
'         __vbaVargParmRef, __vbaVargVar, __vbaVargVarCopy, __vbaVargVarMove,
'         __vbaVargVarRef)
' -----------------------------------------------------------------------------
Private Sub TestVarArgsAndParamArray()
    Dim total As Long
    Dim joined As String

    total = SumNumbers(10, 20, 30, 40)
    If total <> 100 Then Err.Raise 10, , "SumNumbers failed"

    joined = ConcatValues("VB6", 6, True, "ClassicBasic")
    If joined <> "VB6-6-True-ClassicBasic" Then Err.Raise 11, , "ConcatValues failed"

    PrintLine "Test 1: ParamArray and VarArgs passed."
End Sub

Private Function SumNumbers(ParamArray nums() As Variant) As Long
    Dim i As Long
    Dim s As Long
    s = 0
    For i = LBound(nums) To UBound(nums)
        s = s + CLng(nums(i))
    Next i
    SumNumbers = s
End Function

Private Function ConcatValues(ParamArray items() As Variant) As String
    Dim i As Long
    Dim res As String
    res = ""
    For i = LBound(items) To UBound(items)
        If i > LBound(items) Then res = res & "-"
        res = res & CStr(items(i))
    Next i
    ConcatValues = res
End Function

' -----------------------------------------------------------------------------
' TEST 2: Dynamic Variant Arrays (__vbaRedimVar, __vbaRedimVar2,
'         __vbaRedimPreserveVar, __vbaRedimPreserveVar2, __vbaEraseKeepData,
'         __vbaEraseNoPop, __vbaAryConstruct2, __vbaAryMove, __vbaDerefAry,
'         __vbaDerefAry1, __vbaLdZeroAry)
' -----------------------------------------------------------------------------
Private Sub TestDynamicVariantArrays()
    Dim vArr As Variant
    Dim i As Long

    ' 1D ReDim on Variant
    ReDim vArr(1 To 5)
    For i = 1 To 5
        vArr(i) = i * 10
    Next i

    ' 1D ReDim Preserve on Variant
    ReDim Preserve vArr(1 To 8)
    vArr(6) = 60
    vArr(7) = 70
    vArr(8) = 80

    If vArr(1) <> 10 Or vArr(5) <> 50 Or vArr(8) <> 80 Then
        Err.Raise 20, , "ReDim Preserve 1D failed"
    End If

    ' 2D ReDim on Variant
    ReDim vArr(1 To 3, 1 To 4)
    vArr(1, 1) = "Row1Col1"
    vArr(3, 4) = "Row3Col4"

    If vArr(1, 1) <> "Row1Col1" Or vArr(3, 4) <> "Row3Col4" Then
        Err.Raise 21, , "ReDim 2D failed"
    End If

    ' 2D ReDim Preserve on Variant (last dimension resizing)
    ReDim Preserve vArr(1 To 3, 1 To 6)
    vArr(3, 6) = "Row3Col6"
    If vArr(1, 1) <> "Row1Col1" Or vArr(3, 6) <> "Row3Col6" Then
        Err.Raise 22, , "ReDim Preserve 2D failed"
    End If

    ' Erase dynamic array inside variant
    Erase vArr

    PrintLine "Test 2: Dynamic Variant Arrays passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 3: Byte Strings & Advanced String Ops (__vbaInStrB, __vbaInStrVarB,
'         __vbaMidStmtBstrB, __vbaMidStmtVar, __vbaMidStmtVarB, __vbaStrCompVar,
'         __vbaStrTextCmp, __vbaStrVarCopy, rtcStrReverse)
' -----------------------------------------------------------------------------
Private Sub TestByteStringsAndAdvancedStringOps()
    Dim sTarget As String
    Dim vTarget As Variant
    Dim vFind As Variant
    Dim pos As Long
    Dim sRev As String
    Dim cmpRes As Long

    sTarget = "ABCDEFGHIJ"
    ' MidB statement on BSTR
    MidB$(sTarget, 3, 2) = "X"
    If InStr(sTarget, "X") = 0 Then Err.Raise 30, , "MidB statement failed"

    ' Mid statement on Variant
    vTarget = "1234567890"
    Mid(vTarget, 4, 3) = "ABC"
    If vTarget <> "123ABC7890" Then Err.Raise 31, , "Mid Variant statement failed"

    ' InStrB on String and Variant
    sTarget = "Visual Basic 6"
    pos = InStrB(1, sTarget, "B")
    If pos <= 0 Then Err.Raise 32, , "InStrB string failed"

    vTarget = "Visual Basic 6"
    vFind = "B"
    pos = InStrB(1, vTarget, vFind)
    If pos <= 0 Then Err.Raise 33, , "InStrB variant failed"

    ' StrComp on Variant and Text mode
    vTarget = "Apple"
    vFind = "apple"
    cmpRes = StrComp(vTarget, vFind)
    If cmpRes = 0 Then Err.Raise 34, , "StrCompVar binary expected non-zero"

    cmpRes = StrComp(vTarget, vFind, vbTextCompare)
    If cmpRes <> 0 Then Err.Raise 35, , "StrTextCmp expected 0"

    ' StrReverse
    sRev = StrReverse("ClassicBasic")
    If sRev <> "cisaBcissalC" Then Err.Raise 36, , "StrReverse failed"

    PrintLine "Test 3: Byte Strings and Advanced String Ops passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 4: Pattern Matching with Like on Variants (__vbaVarLike,
'         __vbaVarLikeVar)
' -----------------------------------------------------------------------------
Private Sub TestVariantLikePatternMatching()
    Dim vStr As Variant
    Dim vPat As Variant

    vStr = "Antigravity-2026"
    vPat = "Anti*-####"
    If Not (vStr Like vPat) Then Err.Raise 40, , "VarLike wildcard match failed"

    vPat = "Anti*-????"
    If Not (vStr Like vPat) Then Err.Raise 41, , "VarLike question mark failed"

    vPat = "Anti*-[0-9][0-9][0-9][0-9]"
    If Not (vStr Like vPat) Then Err.Raise 42, , "VarLike range match failed"

    vPat = "Anti*-[!a-z][0-9][0-9][0-9]"
    If Not (vStr Like vPat) Then Err.Raise 43, , "VarLike negation match failed"

    vStr = "Invalid"
    If (vStr Like vPat) Then Err.Raise 44, , "VarLike unexpected match failed"

    PrintLine "Test 4: Variant Like Pattern Matching passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 5: Type Checking and Control Flow (__vbaCheckType, __vbaCheckTypeVar,
'         __vbaError)
' -----------------------------------------------------------------------------
Private Sub TestTypeCheckingAndControlFlow()
    Dim vNum As Variant
    Dim vStr As Variant

    vNum = 42
    vStr = "Hello"

    If VarType(vNum) <> vbInteger Then Err.Raise 50, , "CheckTypeVar integer failed"
    If VarType(vStr) <> vbString Then Err.Raise 51, , "CheckTypeVar string failed"

    ' Verify structured error raise and catch
    On Error Resume Next
    Err.Clear
    Err.Raise 9999, "App26", "Custom Error"
    If Err.Number <> 9999 Or Err.Source <> "App26" Then
        Err.Raise 52, , "Custom Err.Raise catch failed"
    End If
    Err.Clear
    On Error GoTo 0

    PrintLine "Test 5: Type Checking and Control Flow passed."
End Sub
