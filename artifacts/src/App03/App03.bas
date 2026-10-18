Attribute VB_Name = "Module1"
Private Declare Function AllocConsole Lib "kernel32" () As Long
    Private Declare Function GetStdHandle Lib "kernel32" (ByVal nStdHandle As Long) As Long
    Private Declare Function WriteConsoleA Lib "kernel32" (ByVal hConsoleOutput As Long, _
        ByVal lpBuffer As String, ByVal nNumberOfCharsToWrite As Long, lpNumberOfCharsWritten As Long, _
        ByVal lpReserved As Long) As Long

    Dim hOut As Long

    Private Sub PrintLine(ByVal txt As String)
        Dim written As Long
        txt = txt & vbCrLf
        WriteConsoleA hOut, txt, Len(txt), written, 0&
    End Sub

    Sub Main()
        AllocConsole
        hOut = GetStdHandle(-11&)

        ' Teste 1: Concatenação simples (&)
        Dim s1 As String, s2 As String, fullStr As String
        s1 = "Classic"
        s2 = "Basic"
        fullStr = s1 & " " & s2 & " v6.0"
        PrintLine "[Test 1] Concat: " & fullStr

        ' Teste 2: Funções de extração (Len, Mid$, Left$, Right$)
        PrintLine "[Test 2] Len: " & Len(fullStr)
        PrintLine "[Test 2] Mid: " & Mid$(fullStr, 9, 5)   ' Esperado: "Basic"
        PrintLine "[Test 2] Left: " & Left$(fullStr, 7)    ' Esperado: "Classic"
        PrintLine "[Test 2] Right: " & Right$(fullStr, 4)  ' Esperado: "v6.0"

        ' Teste 3: Busca e maiúsculas (InStr, UCase$)
        Dim pos As Long
        pos = InStr(fullStr, "Basic")
        PrintLine "[Test 3] InStr 'Basic': " & pos         ' Esperado: 9
        PrintLine "[Test 3] Upper: " & UCase$(s2)          ' Esperado: "BASIC"

        ' Teste 4: Conversão de tipos (Número <-> String)
        Dim n As Long
        n = 100
        PrintLine "[Test 4] Number: " & (n + 23)           ' Esperado: 123
    End Sub
