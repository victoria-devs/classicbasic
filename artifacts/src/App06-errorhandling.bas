Attribute VB_Name = "Module1"
    Option Explicit

    ' --- Win32 Console Output APIs ---
    Private Declare Function AllocConsole Lib "kernel32" () As Long
    Private Declare Function GetStdHandle Lib "kernel32" (ByVal nStdHandle As Long) As Long
    Private Declare Function WriteFile Lib "kernel32" ( _
        ByVal hFile As Long, _
        ByVal lpBuffer As String, _
        ByVal nNumberOfBytesToWrite As Long, _
        ByRef lpNumberOfBytesWritten As Long, _
        ByVal lpOverlapped As Long) As Long

    Private Const STD_OUTPUT_HANDLE As Long = -11&
    Private hStdOut As Long

    Private Sub PrintLine(ByVal sText As String)
        Dim sOut As String
        Dim written As Long
        sOut = sText & vbCrLf
        WriteFile hStdOut, sOut, Len(sOut), written, 0&
    End Sub

    Public Sub Main()
        AllocConsole
        hStdOut = GetStdHandle(STD_OUTPUT_HANDLE)

        PrintLine "=== ClassicBasic App06: Error Handling Test Suite ==="

        ' ========================================================
        ' [Test 1] On Error Resume Next + Division by Zero (Err 11)
        ' ========================================================
        On Error Resume Next
        Dim x As Long, y As Long
        x = 10
        y = 0
        x = x \ y  ' Gera erro 11: Division by zero

        PrintLine "[Test 1] Resume Next Err.Number: " & Err.Number
        PrintLine "[Test 1] Resume Next Err.Description: " & Err.Description

        ' ========================================================
        ' [Test 2] Err.Clear (Redefinição do estado de erro)
        ' ========================================================
        Err.Clear
        PrintLine "[Test 2] After Err.Clear: " & Err.Number

        ' ========================================================
        ' [Test 3] Err.Raise (Erro customizado com Source e Description)
        ' ========================================================
        On Error Resume Next
        Err.Raise 1001, "ClassicBasicApp06", "Custom Raised Business Error"

        PrintLine "[Test 3] Err.Raise Number: " & Err.Number
        PrintLine "[Test 3] Err.Raise Source: " & Err.Source
        PrintLine "[Test 3] Err.Raise Description: " & Err.Description
        Err.Clear

        ' ========================================================
        ' [Test 4] On Error GoTo Handler (Tratamento com salto para label)
        ' ========================================================
        On Error GoTo ErrorHandler
        Dim a As Long, b As Long
        a = 100
        b = 0
        a = a \ b  ' Dispara salto para ErrorHandler
        PrintLine "[Test 4] ERRO: Nao deveria alcancar esta linha!"
        Exit Sub

ContinueAfterError:
        ' ========================================================
        ' [Test 5] On Error GoTo 0 (Desativação de manipulador)
        ' ========================================================
        On Error GoTo 0
        PrintLine "[Test 5] On Error GoTo 0: Handler Reset OK"
        PrintLine "=== All Error Handling Tests Completed ==="
        Exit Sub

ErrorHandler:
        PrintLine "[Test 4] Caught in Handler Err.Number: " & Err.Number
        PrintLine "[Test 4] Caught in Handler Err.Description: " & Err.Description
        Resume ContinueAfterError
    End Sub

