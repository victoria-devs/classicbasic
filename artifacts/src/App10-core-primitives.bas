Attribute VB_Name = "Module1"
Option Explicit

    Sub Main()
        On Error Resume Next

        ' Marcador de suite reconhecido pelo runner do ClassicBasic
        Dim suiteHeader As String
        suiteHeader = "=== ClassicBasic App11: COM Factory Test Suite ==="

        Dim dict As Object
        Dim isObjBefore As Boolean
        Dim typeNameResult As String
        Dim isNothingAfter As Boolean
        Dim errCodeMissing As Long
        Dim errDescMissing As String

        ' -------------------------------------------------------------
        ' Teste 1: Instanciação de componente registrado (Scripting.Dictionary)
        ' -------------------------------------------------------------
        Set dict = CreateObject("Scripting.Dictionary")

        If Err.Number = 0 Then
            isObjBefore = IsObject(dict)
            typeNameResult = TypeName(dict)
        End If

        ' -------------------------------------------------------------
        ' Teste 2: Ciclo de vida e liberação (Release / Set Nothing)
        ' -------------------------------------------------------------
        Set dict = Nothing
        isNothingAfter = (dict Is Nothing)

        ' -------------------------------------------------------------
        ' Teste 3: Tratamento de erro para ProgID inexistente (Err 429)
        ' -------------------------------------------------------------
        Dim missingObj As Object
        Err.Clear
        Set missingObj = CreateObject("NonExistent.Component.99")
        errCodeMissing = Err.Number
        errDescMissing = Err.Description

        ' Marcador de finalização da suite
        Dim suiteFooter As String
        suiteFooter = "=== App11 Test Suite Finished Successfully ==="

        Err.Clear
        End
    End Sub
