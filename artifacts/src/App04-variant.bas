Attribute VB_Name = "Module1"
    Option Explicit

    ' --- Declarações da Win32 Console API ---
    Private Declare Function AllocConsole Lib "kernel32" () As Long
    Private Declare Function GetStdHandle Lib "kernel32" (ByVal nStdHandle As Long) As Long
    Private Declare Function WriteConsoleA Lib "kernel32" ( _
        ByVal hConsoleOutput As Long, _
        ByVal lpBuffer As String, _
        ByVal nNumberOfCharsToWrite As Long, _
        lpNumberOfCharsWritten As Long, _
        lpReserved As Any _
    ) As Long

    Private Const STD_OUTPUT_HANDLE As Long = -11
    Private hStdOut As Long

    Private Sub PrintLine(ByVal sText As String)
        Dim written As Long
        sText = sText & vbCrLf
        WriteConsoleA hStdOut, sText, Len(sText), written, ByVal 0&
    End Sub

    Public Sub Main()
        AllocConsole
        hStdOut = GetStdHandle(STD_OUTPUT_HANDLE)

        ' -------------------------------------------------------------------------
        ' [Test 1] Aritmética Básica de Variantes (__vbaVarAdd, __vbaVarSub)
        ' -------------------------------------------------------------------------
        Dim v1 As Variant, v2 As Variant, vRes As Variant
        v1 = 40
        v2 = 2
        vRes = v1 + v2
        PrintLine "[Test 1] VarAdd (40 + 2): " & CStr(vRes)

        vRes = v1 - v2
        PrintLine "[Test 1] VarSub (40 - 2): " & CStr(vRes)

        ' -------------------------------------------------------------------------
        ' [Test 2] Multiplicação, Divisão Real, Divisão Inteira e Mod
        ' (__vbaVarMul, __vbaVarDiv, __vbaVarIdiv, __vbaVarMod)
        ' -------------------------------------------------------------------------
        v1 = 25
        v2 = 4
        vRes = v1 * v2
        PrintLine "[Test 2] VarMul (25 * 4): " & CStr(vRes)

        vRes = v1 / v2
        PrintLine "[Test 2] VarDiv (25 / 4): " & CStr(vRes)

        vRes = v1 \ v2
        PrintLine "[Test 2] VarIdiv (25 \ 4): " & CStr(vRes)

        vRes = v1 Mod v2
        PrintLine "[Test 2] VarMod (25 Mod 4): " & CStr(vRes)

        ' -------------------------------------------------------------------------
        ' [Test 3] Coerção Dinâmica de Tipos (String Numérica + Inteiro)
        ' Em VB6, vString + vInt converte a string para número se for numérica!
        ' Já vString & vInt faz concatenação explícita de variantes (__vbaVarCat)
        ' -------------------------------------------------------------------------
        Dim vStrNum As Variant, vNum As Variant
        vStrNum = "100"
        vNum = 25
        vRes = vStrNum + vNum
        PrintLine "[Test 3] Dynamic Coercion ('100' + 25): " & CStr(vRes)

        vRes = vStrNum & vNum
        PrintLine "[Test 3] VarCat ('100' & 25): " & CStr(vRes)

        ' -------------------------------------------------------------------------
        ' [Test 4] Booleanos, Negação Aritmética e Negação Lógica
        ' (__vbaVarNeg, __vbaVarNot)
        ' -------------------------------------------------------------------------
        Dim vBool As Variant, vNeg As Variant
        vNum = 42
        vNeg = -vNum
        PrintLine "[Test 4] VarNeg (-42): " & CStr(vNeg)

        vBool = True
        vRes = Not vBool
        PrintLine "[Test 4] VarNot (Not True): " & CStr(vRes)

        ' -------------------------------------------------------------------------
        ' [Test 5] Comparações Relacionais de Variantes
        ' (__vbaVarTstGt, __vbaVarTstEq, __vbaVarTstLt)
        ' -------------------------------------------------------------------------
        Dim vA As Variant, vB As Variant
        vA = 50
        vB = 30
        PrintLine "[Test 5] VarTst (50 > 30): " & CStr(vA > vB)
        PrintLine "[Test 5] VarTst (50 = 30): " & CStr(vA = vB)

        ' -------------------------------------------------------------------------
        ' [Test 6] Inspeção de Tipos Dinâmicos (VarType)
        ' (rtcVarType / ordinal 595: vbInteger = 2, vbLong = 3, vbString = 8, vbBoolean = 11)
        ' -------------------------------------------------------------------------
        Dim vInt As Variant, vStr As Variant
        vInt = CInt(10)
        vStr = "ClassicBasic"
        PrintLine "[Test 6] VarType(Integer 10): " & CStr(VarType(vInt))
        PrintLine "[Test 6] VarType(String): " & CStr(VarType(vStr))
        PrintLine "[Test 6] VarType(Boolean): " & CStr(VarType(vBool))
    End Sub

