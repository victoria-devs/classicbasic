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
    PrintLine "=== ClassicBasic App25: Variant Bitwise, Relational Testing & Vector Assignments ==="

    TestVariantBitwiseOperations
    TestVariantRelationalTesting
    TestVariantAssignmentAndVectorOps
    ModuleText.TestVariantTextComparisons

    PrintLine "=== App25 Test Suite Finished Successfully ==="
End Sub

' -----------------------------------------------------------------------------
' TEST 1: Variant Bitwise & Logical Operations (__vbaVarOr, __vbaVarXor,
'         __vbaVarEqv, __vbaVarImp, __vbaVarPow)
' -----------------------------------------------------------------------------
Private Sub TestVariantBitwiseOperations()
    Dim v1 As Variant
    Dim v2 As Variant
    Dim vRes As Variant

    v1 = &H0F
    v2 = &HF0
    vRes = v1 Or v2
    If vRes <> &HFF Then Err.Raise 10, , "VarOr failed"

    v1 = &HFF
    v2 = &H0F
    vRes = v1 Xor v2
    If vRes <> &HF0 Then Err.Raise 11, , "VarXor failed"

    v1 = True
    v2 = True
    vRes = v1 Eqv v2
    If vRes <> True Then Err.Raise 12, , "VarEqv true failed"
    v2 = False
    vRes = v1 Eqv v2
    If vRes <> False Then Err.Raise 13, , "VarEqv false failed"

    v1 = True
    v2 = False
    vRes = v1 Imp v2
    If vRes <> False Then Err.Raise 14, , "VarImp true->false failed"
    v1 = False
    vRes = v1 Imp v2
    If vRes <> True Then Err.Raise 15, , "VarImp false->false failed"

    v1 = 3
    v2 = 4
    vRes = v1 ^ v2
    If vRes <> 81 Then Err.Raise 16, , "VarPow failed"

    PrintLine "Test 1: Variant Bitwise Operations passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 2: Variant Relational Testing in Conditionals (__vbaVarTstEq,
'         __vbaVarTstNe, __vbaVarTstLt, __vbaVarTstLe, __vbaVarTstGt,
'         __vbaVarTstGe)
' -----------------------------------------------------------------------------
Private Sub TestVariantRelationalTesting()
    Dim v1 As Variant
    Dim v2 As Variant

    v1 = 100
    v2 = 100
    If Not (v1 = v2) Then Err.Raise 20, , "VarTstEq failed"
    If v1 <> v2 Then Err.Raise 21, , "VarTstNe equal failed"

    v2 = 200
    If Not (v1 <> v2) Then Err.Raise 22, , "VarTstNe different failed"
    If Not (v1 < v2) Then Err.Raise 23, , "VarTstLt failed"
    If Not (v1 <= v2) Then Err.Raise 24, , "VarTstLe less failed"
    If v1 > v2 Then Err.Raise 25, , "VarTstGt unexpected failed"
    If v1 >= v2 Then Err.Raise 26, , "VarTstGe unexpected failed"

    v1 = 200
    If Not (v1 <= v2) Then Err.Raise 27, , "VarTstLe equal failed"
    If Not (v1 >= v2) Then Err.Raise 28, , "VarTstGe equal failed"

    v1 = 300
    If Not (v1 > v2) Then Err.Raise 29, , "VarTstGt failed"
    If Not (v1 >= v2) Then Err.Raise 30, , "VarTstGe greater failed"

    ' String comparison in binary mode
    v1 = "Apple"
    v2 = "apple"
    If v1 = v2 Then Err.Raise 31, , "VarTstEq binary case sensitivity failed"
    If Not (v1 < v2) Then Err.Raise 32, , "VarTstLt binary case failed"

    PrintLine "Test 2: Variant Relational Testing passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 3: Variant Assignment, Zero and Array Index Storage (__vbaVarSetVar,
'         __vbaVarSetVarAddref, __vbaVarZero, __vbaVarIndexStore, __vbaVar2Vec)
' -----------------------------------------------------------------------------
Private Sub TestVariantAssignmentAndVectorOps()
    Dim vSource As Variant
    Dim vTarget As Variant
    Dim vArr As Variant

    ' Assignment of variant containing value
    vSource = "ClassicBasic Variant"
    vTarget = vSource
    If vTarget <> "ClassicBasic Variant" Then Err.Raise 40, , "VarSetVar failed"

    ' Zeroing variant
    vTarget = 0
    If vTarget <> 0 Then Err.Raise 41, , "VarZero failed"

    ' Variant Array allocation and element indexing
    ReDim vArr(1 To 3) As Variant
    vArr(1) = "First"
    vArr(2) = 42
    vArr(3) = 3.14159

    If vArr(1) <> "First" Then Err.Raise 42, , "VarIndexStore 1 failed"
    If vArr(2) <> 42 Then Err.Raise 43, , "VarIndexStore 2 failed"
    If Abs(vArr(3) - 3.14159) > 0.0001 Then Err.Raise 44, , "VarIndexStore 3 failed"

    PrintLine "Test 3: Variant Assignment and Vector Ops passed."
End Sub
