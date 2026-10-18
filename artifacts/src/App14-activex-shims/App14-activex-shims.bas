   Attribute VB_Name = "Module1"
    Option Explicit

    Sub Main()
        On Error Resume Next

        Dim suiteHeader As String
        suiteHeader = "=== ClassicBasic App14: ActiveX Shims Test Suite ==="

        ' =========================================================================
        ' PARTE 1: Scripting.FileSystemObject (FSO) & TextStream
        ' =========================================================================
        Dim fso As Object
        Set fso = CreateObject("Scripting.FileSystemObject")

        Dim isFsoCreated As Boolean
        isFsoCreated = Not (fso Is Nothing)

        Dim builtPath As String
        Dim driveName As String
        Dim parentFolder As String
        Dim fileName As String
        Dim baseName As String
        Dim extName As String

        If isFsoCreated Then
            builtPath = fso.BuildPath("C:\Temp", "test.txt")
            driveName = fso.GetDriveName("C:\Temp\test.txt")
            parentFolder = fso.GetParentFolderName("C:\Temp\test.txt")
            fileName = fso.GetFileName("C:\Temp\test.txt")
            baseName = fso.GetBaseName("C:\Temp\test.txt")
            extName = fso.GetExtensionName("C:\Temp\test.txt")
        End If

        ' Teste de I/O via TextStream
        Dim sampleFile As String
        sampleFile = "app14_sample.txt"

        Dim tsOut As Object
        Dim fileCreated As Boolean
        If isFsoCreated Then
            Set tsOut = fso.CreateTextFile(sampleFile, True)
            If Not (tsOut Is Nothing) Then
                tsOut.WriteLine "Linha 1: ClassicBasic FSO"
                tsOut.WriteLine "Linha 2: Clean-Room ActiveX"
                tsOut.Close
                Set tsOut = Nothing
                fileCreated = fso.FileExists(sampleFile)
            End If
        End If

        ' Leitura via OpenTextFile (1 = ForReading)
        Dim tsIn As Object
        Dim readLine1 As String
        Dim readLine2 As String
        Dim fileSize As Long

        If fileCreated Then
            Set tsIn = fso.OpenTextFile(sampleFile, 1)
            If Not (tsIn Is Nothing) Then
                readLine1 = tsIn.ReadLine
                readLine2 = tsIn.ReadLine
                tsIn.Close
                Set tsIn = Nothing
            End If

            Dim fileItem As Object
            Set fileItem = fso.GetFile(sampleFile)
            If Not (fileItem Is Nothing) Then
                fileSize = fileItem.Size
                Set fileItem = Nothing
            End If

            ' Limpeza do arquivo
            fso.DeleteFile sampleFile
        End If

        Dim fileDeleted As Boolean
        If isFsoCreated Then
            fileDeleted = Not fso.FileExists(sampleFile)
            Set fso = Nothing
        End If

        ' =========================================================================
        ' PARTE 2: WScript.Shell
        ' =========================================================================
        Dim wsh As Object
        Set wsh = CreateObject("WScript.Shell")

        Dim isWshCreated As Boolean
        isWshCreated = Not (wsh Is Nothing)

        Dim curDir As String
        Dim expEnv As String
        Dim regWritten As Boolean
        Dim regReadVal As String

        If isWshCreated Then
            curDir = wsh.CurrentDirectory
            expEnv = wsh.ExpandEnvironmentStrings("%PATH%")

            ' Escrita e leitura no registro virtual
            wsh.RegWrite "HKCU\Software\ClassicBasic\TestKey", "ActiveX_Ready", "REG_SZ"
            regReadVal = wsh.RegRead("HKCU\Software\ClassicBasic\TestKey")
            regWritten = (regReadVal = "ActiveX_Ready")

            ' Limpeza da chave
            wsh.RegDelete "HKCU\Software\ClassicBasic\TestKey"
            Set wsh = Nothing
        End If

        ' =========================================================================
        ' PARTE 3: WScript.Network
        ' =========================================================================
        Dim net As Object
        Set net = CreateObject("WScript.Network")

        Dim isNetCreated As Boolean
        isNetCreated = Not (net Is Nothing)

        Dim userName As String
        Dim compName As String
        Dim userDomain As String

        If isNetCreated Then
            userName = net.UserName
            compName = net.ComputerName
            userDomain = net.UserDomain
            Set net = Nothing
        End If

        ' =========================================================================
        ' PARTE 4: ADODB.Recordset (Disconnected In-Memory Recordset)
        ' =========================================================================
        Dim rs As Object
        Set rs = CreateObject("ADODB.Recordset")

        Dim isRsCreated As Boolean
        isRsCreated = Not (rs Is Nothing)

        Dim rsCount As Long
        Dim firstRowId As Long
        Dim firstRowName As String
        Dim secondRowId As Long
        Dim secondRowName As String
        Dim isAtEof As Boolean

        If isRsCreated Then
            ' adInteger = 3, adVarChar = 200
            rs.Fields.Append "ID", 3
            rs.Fields.Append "Name", 200, 50
            rs.Open

            ' Registro 1
            rs.AddNew
            rs.Fields("ID").Value = 101
            rs.Fields("Name").Value = "Rust"
            rs.Update

            ' Registro 2
            rs.AddNew
            rs.Fields("ID").Value = 102
            rs.Fields("Name").Value = "ClassicBasic"
            rs.Update

            rsCount = rs.RecordCount

            ' Navegacao
            rs.MoveFirst
            firstRowId = rs.Fields("ID").Value
            firstRowName = rs.Fields("Name").Value

            rs.MoveNext
            secondRowId = rs.Fields("ID").Value
            secondRowName = rs.Fields("Name").Value

            rs.MoveNext
            isAtEof = rs.EOF

            rs.Close
            Set rs = Nothing
        End If

        Dim suiteFooter As String
        suiteFooter = "=== App14 Test Suite Finished Successfully ==="

        Err.Clear
        End
    End Sub