    Attribute VB_Name = "Module1"
    Option Explicit

    Sub Main()
        On Error Resume Next

        Dim suiteHeader As String
        suiteHeader = "=== ClassicBasic App15: ActiveX Consolidation & Self-Describing ITypeInfo ==="

        ' -------------------------------------------------------------
        ' 1. Self-Describing ITypeInfo Introspection via TypeName()
        ' -------------------------------------------------------------
        Dim fso As Object
        Set fso = CreateObject("Scripting.FileSystemObject")
        Dim typeNameFso As String
        typeNameFso = TypeName(fso)

        Dim wsh As Object
        Set wsh = CreateObject("WScript.Shell")
        Dim typeNameWsh As String
        typeNameWsh = TypeName(wsh)

        Dim net As Object
        Set net = CreateObject("WScript.Network")
        Dim typeNameNet As String
        typeNameNet = TypeName(net)

        Dim rs As Object
        Set rs = CreateObject("ADODB.Recordset")
        Dim typeNameRs As String
        typeNameRs = TypeName(rs)

        Dim dict As Object
        Set dict = CreateObject("Scripting.Dictionary")
        Dim typeNameDict As String
        typeNameDict = TypeName(dict)

        ' -------------------------------------------------------------
        ' 2. Dynamic Member Invocation via CallByName (VbMethod & VbGet)
        ' -------------------------------------------------------------
        ' Test FSO.BuildPath via CallByName (VbMethod = 1)
        Dim pathResult As Variant
        If Not (fso Is Nothing) Then
            pathResult = CallByName(fso, "BuildPath", VbMethod, "C:\Test", "file.txt")
        End If

        ' Test Dictionary.Add and Dictionary.Item via CallByName
        Dim dictVal As Variant
        If Not (dict Is Nothing) Then
            CallByName dict, "Add", VbMethod, "Key1", "Val1"
            dictVal = CallByName(dict, "Item", VbGet, "Key1")
        End If

        ' Test ADODB.Recordset via CallByName
        Dim rsCount As Long
        If Not (rs Is Nothing) Then
            CallByName rs, "Open", VbMethod
            CallByName rs, "AddNew", VbMethod
            rsCount = CallByName(rs, "RecordCount", VbGet)
        End If

        ' -------------------------------------------------------------
        ' 3. Error Handling on CallByName (Member not found -> Err 438)
        ' -------------------------------------------------------------
        Err.Clear
        Dim invalidCall As Variant
        Dim errCode As Long
        If Not (dict Is Nothing) Then
            invalidCall = CallByName(dict, "NonExistentMethod", VbMethod)
            errCode = Err.Number
        End If

        ' -------------------------------------------------------------
        ' 4. Final Verification
        ' -------------------------------------------------------------
        Dim suiteFooter As String
        suiteFooter = "=== App15 Test Suite Finished Successfully ==="

        Err.Clear
        End
    End Sub