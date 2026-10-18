    Attribute VB_Name = "Module1"
    Option Explicit

    ' Win32 API para saída limpa no console/stdout
    Private Declare Sub ExitProcess Lib "kernel32" (ByVal uExitCode As Long)

    Private Sub Assert(ByVal condition As Boolean, ByVal msg As String)
        If Not condition Then
            MsgBox "ASSERT FAILED: " & msg, vbCritical, "Test Failure"
            Call ExitProcess(1)
        End If
    End Sub

    Public Sub Main()
        On Error GoTo ErrorHandler

        ' =========================================================================
        ' 1. BSTR COM CARACTERES NULOS EMBUTIDOS (EMBEDDED NULLS)
        ' =========================================================================
        Dim sNull As String
        sNull = "Hello" & Chr$(0) & "World" & Chr$(0) & Chr$(0) & "VB6"

        ' Len deve ler do cabeçalho de comprimento (offset -4), não parar no primeiro \0
        Assert Len(sNull) = 14, "Len(sNull) must be exactly 14 characters"
        Assert LenB(sNull) = 28, "LenB(sNull) must be exactly 28 bytes"

        ' Concatenação deve manter os bytes nulos intactos
        Dim sConcat As String
        sConcat = sNull & "End"
        Assert Len(sConcat) = 17, "Len(sConcat) must be 17"
        Assert Right$(sConcat, 6) = Chr$(0) & "VB6End", "Right$ must preserve embedded nulls"
        Assert Mid$(sConcat, 6, 1) = Chr$(0), "Mid$ must extract embedded null"
        Assert Mid$(sConcat, 7, 5) = "World", "Mid$ past embedded null must extract World"

        ' =========================================================================
        ' 2. SEMÂNTICA DE vbNullString (PONTEIRO 0x0) EM TODOS OS HELPERS
        ' =========================================================================
        Dim sRawNull As String
        sRawNull = vbNullString

        ' Len e LenB em vbNullString devem retornar 0 sem dereferenciar ponteiro nulo
        Assert Len(sRawNull) = 0, "Len(vbNullString) must be 0"
        Assert LenB(sRawNull) = 0, "LenB(vbNullString) must be 0"
        Assert sRawNull = "", "vbNullString must evaluate equal to empty string"
        Assert (sRawNull & "Test" = "Test"), "vbNullString concatenation must work"
        Assert ("Prefix" & sRawNull = "Prefix"), "Prefix with vbNullString must work"
        Assert Trim$(sRawNull) = "", "Trim$(vbNullString) must return empty string"
        Assert InStr(1, sRawNull, "A") = 0, "InStr in vbNullString must return 0"
        Assert InStr(1, "Test", sRawNull) = 1, "InStr for vbNullString in string must return 1"

        ' =========================================================================
        ' 3. CONVERSÃO StrConv BIDIRECIONAL (vbFromUnicode e vbUnicode)
        ' =========================================================================
        Dim sOrigin As String
        sOrigin = "Architecture"

        ' vbFromUnicode = 128: Converte BSTR UTF-16 para bytes ANSI
        Dim bAnsi As Variant
        bAnsi = StrConv(sOrigin, vbFromUnicode)
        Assert LenB(bAnsi) = 12, "LenB of ANSI converted string must be 12 bytes"

        ' vbUnicode = 64: Reconstitui UTF-16 a partir de ANSI
        Dim sRestored As String
        sRestored = StrConv(bAnsi, vbUnicode)
        Assert sRestored = sOrigin, "StrConv roundtrip must recover original BSTR"
        Assert Len(sRestored) = 12, "Len of recovered string must be 12"
        Assert LenB(sRestored) = 24, "LenB of recovered string must be 24 bytes"

        ' =========================================================================
        ' 4. FUNÇÃO String$ COM OVERFLOW/WRAPPING DE CÓDIGO DE CARACTERE
        ' =========================================================================
        Dim sA As String
        sA = String$(5, 65) ' Código ASCII de 'A'
        Assert sA = "AAAAA", "String$(5, 65) must produce 'AAAAA'"

        ' Código com valor > 255 (ex: 1024 -> 1024 Mod 256 = 0, produzindo nulos)
        Dim sWrap As String
        sWrap = String$(4, 1024)
        Assert Len(sWrap) = 4, "String$(4, 1024) length must be 4"
        Assert Asc(sWrap) = 0, "String$(4, 1024) must wrap character code mod 256 to 0"

        ' String$ usando string como caractere base (pega o primeiro caractere)
        Dim sChar As String
        sChar = String$(3, "VisualBasic")
        Assert sChar = "VVV", "String$ with string argument must take first char"

        ' =========================================================================
        ' 5. OPERADOR Like COM RANGES COMPLEXOS, NEGAÇÃO E ESCAPES
        ' =========================================================================
        ' Wildcards básicos: ?, *, #
        Assert "Doc1.txt" Like "Doc?.txt", "Like with ? wildcard"
        Assert "Doc12.txt" Like "Doc*.txt", "Like with * wildcard"
        Assert "Item9" Like "Item#", "Like with # digit wildcard"
        Assert Not ("ItemA" Like "Item#"), "Like with # rejecting non-digit"

        ' Range de caracteres [0-9] e [a-z]
        Assert "File09_v2.dat" Like "File[0-9][0-9]_[!a-z]#[.][a-z]*", "Complex Like pattern matching"

        ' Negação de conjunto [!]
        Assert "A" Like "[!0-9]", "Like [!0-9] matching letter"
        Assert Not ("9" Like "[!0-9]"), "Like [!0-9] rejecting digit"
        Assert "abc" Like "[!A-Z]*", "Like [!A-Z]* matching lowercase"
        Assert Not ("ABC" Like "[!A-Z]*"), "Like [!A-Z]* rejecting uppercase"

        ' Escape de colchete e caracteres especiais dentro de [ ]
        Assert "test.dat" Like "*[.]dat", "Like [.] matching literal dot"
        Assert "a[b" Like "a[[]b", "Like [[] matching literal opening bracket"

        ' =========================================================================
        ' 6. Replace COM BINARY/TEXT COMPARE, LIMITES DE COUNT E Start
        ' =========================================================================
        Dim sSource As String
        sSource = "AbcAbaBC"

        ' vbBinaryCompare = 0, Start = 1, Count = 1
        ' Deve substituir apenas a ocorrência exata 'ab' (minúscula) na posição 4
        Dim sRepBin As String
        sRepBin = Replace(sSource, "ab", "XX", 1, 1, vbBinaryCompare)
        Assert sRepBin = "AbcXXaBC", "Replace binary compare count 1"

        ' vbTextCompare = 1 (Case-Insensitive), Start = 1, Count = -1 (todos)
        Dim sRepText As String
        sRepText = Replace(sSource, "ab", "1", 1, -1, vbTextCompare)
        Assert sRepText = "1c1a1C", "Replace text compare case-insensitive"

        ' Semântica do parâmetro Start > 1 no VB6:
        ' No VB6, Replace com Start > 1 retorna a string CORTADA a partir de Start!
        Dim sRepStart As String
        sRepStart = Replace(sSource, "a", "_", 4, -1, vbBinaryCompare)
        ' A partir do índice 4 ("AbaBC"): os 'a' minúsculos viram '_' -> "Ab_BC"
        Assert sRepStart = "Ab_BC", "Replace with Start > 1 truncates prefix in VB6 semantics"

        ' Concluído com sucesso
        Call ExitProcess(0)
        Exit Sub

    ErrorHandler:
        MsgBox "UNHANDLED EXCEPTION: " & Err.Description, vbCritical, "Runtime Error"
        Call ExitProcess(2)
    End Sub