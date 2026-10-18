Attribute VB_Name = "Module1"
    Option Explicit

    Sub Main()
        On Error Resume Next

        Dim suiteHeader As String
        suiteHeader = "=== ClassicBasic App12: TypeLib Introspection Test Suite ==="

        ' -------------------------------------------------------------
        ' Teste 1: Instanciação tipada via TypeLib (Early-Bound)
        ' -------------------------------------------------------------
        Dim dict As Scripting.Dictionary
        Set dict = New Scripting.Dictionary

        Dim isInstantiated As Boolean
        isInstantiated = Not (dict Is Nothing)

        ' -------------------------------------------------------------
        ' Teste 2: Invocação de métodos e propriedades
        ' -------------------------------------------------------------
        Dim countInitial As Long
        Dim existsBefore As Boolean
        Dim itemVal As String
        Dim countAfter As Long
        Dim existsAfter As Boolean

        If isInstantiated Then
            countInitial = dict.Count

            dict.Add "Alpha", "Value1"
            dict.Add "Beta", "Value2"

            existsBefore = dict.Exists("Alpha")
            itemVal = dict.Item("Alpha")

            dict.Remove "Alpha"
            existsAfter = dict.Exists("Alpha")
            countAfter = dict.Count
        End If

        ' -------------------------------------------------------------
        ' Teste 3: Limpeza e liberação do objeto
        ' -------------------------------------------------------------
        Set dict = Nothing
        Dim isCleanedUp As Boolean
        isCleanedUp = (dict Is Nothing)

        Dim suiteFooter As String
        suiteFooter = "=== App12 Test Suite Finished Successfully ==="

        Err.Clear
        End
    End Sub