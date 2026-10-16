Attribute VB_Name = "Module1"
    Option Explicit

    Sub Main()
        On Error Resume Next

        Dim suiteHeader As String
        suiteHeader = "=== ClassicBasic App13: Late-Binding Dispatch Engine Test Suite ==="

        ' -------------------------------------------------------------------------
        ' Teste 1: Instanciação dinâmica Late-Bound (As Object) via CreateObject
        ' -------------------------------------------------------------------------
        Dim dictObj As Object
        Set dictObj = CreateObject("Scripting.Dictionary")

        Dim isObjCreated As Boolean
        isObjCreated = Not (dictObj Is Nothing)

        ' -------------------------------------------------------------------------
        ' Teste 2: Invocação de métodos sem retorno (DISPATCH_METHOD)
        ' -------------------------------------------------------------------------
        If isObjCreated Then
            ' Adiciona múltiplos pares chave/valor com diferentes tipos de Variant
            dictObj.Add "Name", "ClassicBasic"
            dictObj.Add "Year", 1998
            dictObj.Add "Pi", 3.14159
        End If

        ' -------------------------------------------------------------------------
        ' Teste 3: Leitura de propriedades e métodos com retorno (PROPERTYGET / METHOD)
        ' -------------------------------------------------------------------------
        Dim countVal As Long
        Dim existsName As Boolean
        Dim existsNonExistent As Boolean
        Dim nameVal As String
        Dim yearVal As Long

        If isObjCreated Then
            countVal = dictObj.Count
            existsName = dictObj.Exists("Name")
            existsNonExistent = dictObj.Exists("NotFound")

            nameVal = dictObj.Item("Name")
            yearVal = dictObj.Item("Year")
        End If

        ' -------------------------------------------------------------------------
        ' Teste 4: Modificação de propriedades (DISPATCH_PROPERTYPUT / Let)
        ' -------------------------------------------------------------------------
        Dim updatedNameVal As String
        If isObjCreated Then
            ' Atualiza Item via late-bound Property Put
            dictObj.Item("Name") = "ClassicBasic-Rust"
            updatedNameVal = dictObj.Item("Name")

            ' Altera chave existente via put_Key
            dictObj.Key("Year") = "ReleaseYear"
        End If

        ' -------------------------------------------------------------------------
        ' Teste 5: Invocação de Membro Padrão (Default Member / DISPID 0)
        ' No Scripting.Dictionary, Item é o membro padrão
        ' -------------------------------------------------------------------------
        Dim defaultMemberVal As String
        If isObjCreated Then
            ' Leitura pelo membro padrão: dictObj("Name")
            defaultMemberVal = dictObj("Name")

            ' Escrita pelo membro padrão
            dictObj("Framework") = "Antigravity"
        End If

        ' -------------------------------------------------------------------------
        ' Teste 6: Extração de Arrays Variant (Keys e Items)
        ' -------------------------------------------------------------------------
        Dim keysArr As Variant
        Dim itemsArr As Variant
        Dim numKeys As Long

        If isObjCreated Then
            keysArr = dictObj.Keys()
            itemsArr = dictObj.Items()
            numKeys = UBound(keysArr) - LBound(keysArr) + 1
        End If

        ' -------------------------------------------------------------------------
        ' Teste 7: Remoção de itens (Remove e RemoveAll)
        ' -------------------------------------------------------------------------
        Dim countAfterRemove As Long
        Dim countAfterRemoveAll As Long

        If isObjCreated Then
            dictObj.Remove "ReleaseYear"
            countAfterRemove = dictObj.Count

            dictObj.RemoveAll
            countAfterRemoveAll = dictObj.Count
        End If

        ' -------------------------------------------------------------------------
        ' Teste 8: Late-binding através de variável Variant (As Variant)
        ' -------------------------------------------------------------------------
        Dim vObj As Variant
        Set vObj = CreateObject("Scripting.Dictionary")

        Dim isVarObjCreated As Boolean
        isVarObjCreated = Not (vObj Is Nothing)

        Dim varItemVal As String
        If isVarObjCreated Then
            vObj.Add "VarKey", "VarValue"
            varItemVal = vObj.Item("VarKey")
            Set vObj = Nothing
        End If

        ' -------------------------------------------------------------------------
        ' Teste 9: Tratamento de Erro em chamada a membro inexistente
        ' Deve gerar erro 438 ("Object doesn't support this property or method")
        ' -------------------------------------------------------------------------
        Dim errExpected As Long
        If isObjCreated Then
            Err.Clear
            ' Chamada que não existe no IDispatch do Dictionary
            dictObj.MetodoQueNaoExiste 1, 2, 3
            errExpected = Err.Number
            Err.Clear
        End If

        ' -------------------------------------------------------------------------
        ' Teste 10: Limpeza e descarte do objeto
        ' -------------------------------------------------------------------------
        Set dictObj = Nothing
        Dim isCleanedUp As Boolean
        isCleanedUp = (dictObj Is Nothing)

        Dim suiteFooter As String
        suiteFooter = "=== App13 Test Suite Finished Successfully ==="

        Err.Clear
        End
    End Sub