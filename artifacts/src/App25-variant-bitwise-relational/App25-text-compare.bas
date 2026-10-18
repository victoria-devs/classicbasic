Attribute VB_Name = "ModuleText"
Option Explicit
Option Compare Text

' -----------------------------------------------------------------------------
' TEST 4: Variant Comparisons under Option Compare Text (__vbaVarTextTstEq..Ne,
'         __vbaVarTextCmpEq..Ne)
' -----------------------------------------------------------------------------
Public Sub TestVariantTextComparisons()
    Dim v1 As Variant
    Dim v2 As Variant
    Dim bCmp As Boolean

    v1 = "HELLO WORLD"
    v2 = "hello world"

    ' Text conditional relational tests (__vbaVarTextTstEq, Ne, Lt, Le, Gt, Ge)
    If Not (v1 = v2) Then Err.Raise 50, , "VarTextTstEq failed"
    If v1 <> v2 Then Err.Raise 51, , "VarTextTstNe equal failed"
    If Not (v1 <= v2) Then Err.Raise 52, , "VarTextTstLe equal failed"
    If Not (v1 >= v2) Then Err.Raise 53, , "VarTextTstGe equal failed"

    v2 = "ZZZ"
    If Not (v1 < v2) Then Err.Raise 54, , "VarTextTstLt failed"
    If Not (v1 <= v2) Then Err.Raise 55, , "VarTextTstLe failed"

    v2 = "AAA"
    If Not (v1 > v2) Then Err.Raise 56, , "VarTextTstGt failed"
    If Not (v1 >= v2) Then Err.Raise 57, , "VarTextTstGe failed"

    ' Text comparison expressions (__vbaVarTextCmpEq..Ne)
    v1 = "Basic"
    v2 = "BASIC"
    bCmp = (v1 = v2)
    If Not bCmp Then Err.Raise 58, , "VarTextCmpEq failed"

    bCmp = (v1 <> v2)
    If bCmp Then Err.Raise 59, , "VarTextCmpNe failed"

    Module1.PrintLine "Test 4: Variant Text Comparisons (Option Compare Text) passed."
End Sub
