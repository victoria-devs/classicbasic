  Attribute VB_Name = "Module1"
    Option Explicit

    Public gZombieRef As CZombie
    Public gLog As String

    Sub Main()
        On Error Resume Next

        Dim suiteHeader As String
        suiteHeader = "=== ClassicBasic App17: Esoteric Runtime Quirks & Zombie Objects ==="

        ' ------------------------------------------------------------------
        ' 1. Zombie Object State Transition & Invocation Lockout
        ' ------------------------------------------------------------------
        Dim z As CZombie
        Set z = New CZombie
        z.Value = 42

        ' Destroi a referencia local, acionando Class_Terminate
        ' No Class_Terminate, ele faz: Set gZombieRef = Me
        Set z = Nothing

        ' gZombieRef agora aponta para um objeto Zombie
        Err.Clear
        Dim zombieErrNum As Long
        zombieErrNum = 0
        If Not (gZombieRef Is Nothing) Then
            ' Invocacao em objeto Zombie deve disparar Automation Error (E_UNEXPECTED / 440)
            gZombieRef.DoSomething
            zombieErrNum = Err.Number
            Err.Clear
            ' Libera a referencia zombie sem re-disparar Class_Terminate
            Set gZombieRef = Nothing
        End If

        ' ------------------------------------------------------------------
        ' 2. SafeArray Lock Violation (Runtime Error 10)
        ' ------------------------------------------------------------------
        Dim dynArr() As Long
        ReDim dynArr(1 To 5)
        dynArr(1) = 100

        Dim arrErrNum As Long
        arrErrNum = 0

        ' Itera com For Each, o que incrementa cLocks no SafeArray
        Dim elem As Variant
        For Each elem In dynArr
            Err.Clear
            ' Tentativa de ReDim durante iteracao / SafeArray travado
            ReDim dynArr(1 To 10)
            arrErrNum = Err.Number
            Err.Clear
            Exit For
        Next elem

        ' ------------------------------------------------------------------
        ' 3. Memory Aliasing (ByRef apontando para o mesmo endereco)
        ' ------------------------------------------------------------------
        Dim aliasedVar As Long
        aliasedVar = 10
        AliasedMutation aliasedVar, aliasedVar

        ' ------------------------------------------------------------------
        ' 4. Deep ByRef Variant Dereferencing
        ' ------------------------------------------------------------------
        Dim baseVar As Variant
        baseVar = 50
        Dim deepResult As Long
        deepResult = DeepDereferenceVariant(baseVar)

        ' ------------------------------------------------------------------
        ' 5. Cascading On Error GoTo & Reentrant SEH Frames
        ' ------------------------------------------------------------------
        gLog = ""
        CascadingOuter

        ' ------------------------------------------------------------------
        ' 6. Circular Reference & Deterministic Destruction
        ' ------------------------------------------------------------------
        gLog = ""
        TestCircularReferences

        Dim suiteFooter As String
        suiteFooter = "=== App17 Test Suite Finished Successfully ==="
        End
    End Sub

    Private Sub AliasedMutation(ByRef A As Long, ByRef B As Long)
        A = 100
        ' Se A e B sao alias do mesmo endereco, B agora le 100
        B = B + 50
        ' A agora deve ser 150 imediatamente
    End Sub

    Private Function DeepDereferenceVariant(ByRef V As Variant) As Long
        DeepDereferenceVariant = HelperLayer2(V)
    End Function

    Private Function HelperLayer2(ByRef V2 As Variant) As Long
        ' Avalia expressao matematica sobre variante ByRef encadeada
        HelperLayer2 = CLng(V2) + 25
    End Function

    Private Sub CascadingOuter()
        On Error GoTo OuterHandler
        CascadingInner
        Exit Sub
    OuterHandler:
        gLog = gLog & "[OuterHandled:" & Err.Number & "]"
        Err.Clear
    End Sub

    Private Sub CascadingInner()
        On Error GoTo InnerHandler
        ' Falha 1: Divisao por zero
        Err.Raise 11, "Inner", "Div Zero"
        Exit Sub
    InnerHandler:
        gLog = gLog & "[InnerHandled:" & Err.Number & "]"
        Err.Clear
        ' Desativa tratador local e forca segunda falha que desbobina para o Outer
        On Error GoTo 0
        Err.Raise 53, "Inner", "File not found"
    End Sub

    Private Sub TestCircularReferences()
        Dim nA As CNode
        Dim nB As CNode
        Set nA = New CNode
        nA.Name = "NodeA"
        Set nB = New CNode
        nB.Name = "NodeB"

        ' Cria ciclo
        Set nA.Partner = nB
        Set nB.Partner = nA

        ' Quebra o ciclo manualmente antes de soltar a raiz para garantir encerramento
        Set nA.Partner = Nothing
        Set nA = Nothing
        Set nB = Nothing
    End Sub