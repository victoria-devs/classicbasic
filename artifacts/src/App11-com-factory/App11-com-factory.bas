Attribute VB_Name = "Module1"
    Option Explicit

    Sub Main()
        On Error Resume Next

        Dim dict As Object
        Dim isObjBefore As Boolean
        Dim isObjAfter As Boolean
        Dim typeNameResult As String
        Dim errCodeMissing As Long
        Dim errDescMissing As String

        ' -------------------------------------------------------------
        ' Teste 1: Instanciação de componente registrado (Scripting.Dictionary)
        ' -------------------------------------------------------------
        Set dict = CreateObject("Scripting.Dictionary")

        If Err.Number <> 0 Then
            Print #1, "[FAIL] CreateObject Scripting.Dictionary failed: " & Err.Description
            Err.Clear
        Else
            isObjBefore = IsObject(dict)
            typeNameResult = TypeName(dict)

            If (dict Is Nothing) Then
                Print #1, "[FAIL] dict is Nothing"
            Else
                Print #1, "[PASS] CreateObject returned non-null instance"
            End If

            Print #1, "[PASS] IsObject: " & isObjBefore
            Print #1, "[PASS] TypeName: " & typeNameResult
        End If

        ' -------------------------------------------------------------
        ' Teste 2: Ciclo de vida e liberação (Release / Set Nothing)
        ' -------------------------------------------------------------
        Set dict = Nothing

        If (dict Is Nothing) Then
            Print #1, "[PASS] Set Nothing cleanly released object"
        Else
            Print #1, "[FAIL] Object still active after Set Nothing"
        End If

        ' -------------------------------------------------------------
        ' Teste 3: Tratamento de erro para ProgID inexistente (Err 429)
        ' -------------------------------------------------------------
        Dim missingObj As Object
        Err.Clear
        Set missingObj = CreateObject("NonExistent.Component.99")
        errCodeMissing = Err.Number
        errDescMissing = Err.Description

        Print #1, "[PASS] Missing ProgID Err.Number: " & errCodeMissing
        Print #1, "[PASS] Missing ProgID Err.Description: " & errDescMissing

        Err.Clear
        End
    End Sub
