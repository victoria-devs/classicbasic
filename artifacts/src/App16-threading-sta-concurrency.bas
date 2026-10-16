   Attribute VB_Name = "Module1"
    Option Explicit

    Private m_ReentrantCount As Long
    Private m_MaxDepthReached As Long

    Sub Main()
        On Error Resume Next

        Dim suiteHeader As String
        suiteHeader = "=== ClassicBasic App16: STA Concurrency, TLS Isolation & DoEvents ==="

        ' -------------------------------------------------------------
        ' 1. ErrObject TLS & Estado de Erro Isolado
        ' -------------------------------------------------------------
        Err.Clear
        Err.Raise 11, "App16", "Division by zero test"
        Dim errNum1 As Long
        errNum1 = Err.Number

        Err.Clear
        Dim errNumCleared As Long
        errNumCleared = Err.Number

        Err.Raise 53, "App16", "File not found test"
        Dim errNum2 As Long
        errNum2 = Err.Number
        Err.Clear

        ' -------------------------------------------------------------
        ' 2. BSTR Scratch Buffer Stress & Concatenação
        ' -------------------------------------------------------------
        Dim sAccum As String
        sAccum = "Init"
        Dim i As Long
        For i = 1 To 100
            sAccum = sAccum & ":" & CStr(i)
            If Len(sAccum) > 50 Then
                sAccum = Mid$(sAccum, 10, 30)
            End If
        Next i

        ' -------------------------------------------------------------
        ' 3. Contagem de Referências & Ciclo de Vida de Objetos COM
        ' -------------------------------------------------------------
        Dim dict1 As Object
        Dim dict2 As Object
        Dim dict3 As Object

        Set dict1 = CreateObject("Scripting.Dictionary")
        If Not (dict1 Is Nothing) Then
            dict1.Add "Key1", "Val1"
            Set dict2 = dict1
            Set dict3 = dict2

            ' Desfaz referências parciais
            Set dict2 = Nothing
            Dim itemVal As Variant
            itemVal = dict3.Item("Key1")

            ' Libera última referência
            Set dict1 = Nothing
            Set dict3 = Nothing
        End If

        ' -------------------------------------------------------------
        ' 4. DoEvents & Reentrância de Pilha
        ' -------------------------------------------------------------
        m_ReentrantCount = 0
        m_MaxDepthReached = 0
        RecursiveWorker 1, 5

        ' -------------------------------------------------------------
        ' 5. Captura de Erros de Automação COM (Error 440 / 438)
        ' -------------------------------------------------------------
        Dim dummyObj As Object
        Set dummyObj = CreateObject("Scripting.Dictionary")
        Err.Clear
        If Not (dummyObj Is Nothing) Then
            CallByName dummyObj, "NonExistentInvoke", VbMethod
            Dim comErrCode As Long
            comErrCode = Err.Number
            Err.Clear
        End If
        Set dummyObj = Nothing

        Dim suiteFooter As String
        suiteFooter = "=== App16 Test Suite Finished Successfully ==="

        Err.Clear
        End
    End Sub

    Private Sub RecursiveWorker(ByVal currentDepth As Long, ByVal maxDepth As Long)
        On Error Resume Next
        Dim localMarker As Long
        localMarker = currentDepth * 10

        If currentDepth > m_MaxDepthReached Then
            m_MaxDepthReached = currentDepth
        End If

        ' Cede controle via DoEvents
        DoEvents

        If currentDepth < maxDepth Then
            RecursiveWorker currentDepth + 1, maxDepth
        End If

        ' Validação de integridade da pilha local após retorno
        If localMarker <> (currentDepth * 10) Then
            Err.Raise 999, "RecursiveWorker", "Stack corruption detected!"
        End If
    End Sub