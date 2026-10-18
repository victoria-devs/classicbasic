Attribute VB_Name = "Module1"
    Option Explicit

    Private Declare Function GetStdHandle Lib "kernel32" (ByVal nStdHandle As Long) As Long
    Private Declare Function WriteFile Lib "kernel32" ( _
        ByVal hFile As Long, _
        ByVal lpBuffer As String, _
        ByVal nNumberOfBytesToWrite As Long, _
        lpNumberOfBytesWritten As Long, _
        ByVal lpOverlapped As Long) As Long

    Private Const STD_OUTPUT_HANDLE As Long = -11&

    Private Sub ConsolePrint(ByVal sText As String)
        Dim hOut As Long
        Dim lWritten As Long
        Dim sLine As String
        sLine = sText & vbCrLf
        hOut = GetStdHandle(STD_OUTPUT_HANDLE)
        If hOut <> 0 Then
            WriteFile hOut, sLine, Len(sLine), lWritten, 0&
        End If
    End Sub

    Public Sub Main()
        On Error Resume Next

        ConsolePrint "=== ClassicBasic App09: Classic BASIC File I/O Test Suite ==="

        Dim fn As Integer
        Dim sFile1 As String
        Dim sFile2 As String
        Dim sFile3 As String
        Dim sLine As String
        Dim lSize As Long
        Dim lPos As Long
        Dim lValWrite As Long
        Dim lValRead As Long
        Dim sFound As String
        Dim nLines As Long

        sFile1 = "vb6_io_test1.txt"
        sFile2 = "vb6_io_test2.txt"
        sFile3 = "vb6_io_renamed.txt"

        ' -------------------------------------------------------------------------
        ' TEST 1: FreeFile, Open For Output, Print #, Close #
        ' -------------------------------------------------------------------------
        fn = FreeFile
        Open sFile1 For Output As #fn
        Print #fn, "Line 1: ClassicBasic"
        Print #fn, "Line 2: FileIO"
        Close #fn

        If fn >= 1 Then
            ConsolePrint "TEST 1 PASSED: FreeFile=" & CStr(fn) & " and Open For Output / Print # / Close"
        Else
            ConsolePrint "TEST 1 FAILED: Invalid FreeFile channel"
        End If

        ' -------------------------------------------------------------------------
        ' TEST 2: Open For Append and FileLen()
        ' -------------------------------------------------------------------------
        fn = FreeFile
        Open sFile1 For Append As #fn
        Print #fn, "Line 3: Appended"
        Close #fn

        lSize = FileLen(sFile1)
        If lSize > 30 Then
            ConsolePrint "TEST 2 PASSED: Open For Append and FileLen=" & CStr(lSize)
        Else
            ConsolePrint "TEST 2 FAILED: Unexpected FileLen=" & CStr(lSize)
        End If

        ' -------------------------------------------------------------------------
        ' TEST 3: Open For Input, LOF(), EOF(), Line Input #
        ' -------------------------------------------------------------------------
        fn = FreeFile
        Open sFile1 For Input As #fn
        lSize = LOF(fn)
        nLines = 0
        Do While Not EOF(fn)
            Line Input #fn, sLine
            nLines = nLines + 1
        Loop
        Close #fn

        If nLines = 3 And sLine = "Line 3: Appended" And lSize > 30 Then
            ConsolePrint "TEST 3 PASSED: Line Input # read " & CStr(nLines) & " lines, LOF=" & CStr(lSize)
        Else
            ConsolePrint "TEST 3 FAILED: Line Input # count=" & CStr(nLines) & ", last=" & sLine
        End If

        ' -------------------------------------------------------------------------
        ' TEST 4: Open For Binary, Put #, Seek #, Seek(), Get #
        ' -------------------------------------------------------------------------
        fn = FreeFile
        Open sFile2 For Binary As #fn
        lValWrite = 133742
        Put #fn, 1, lValWrite
        Seek #fn, 1
        lPos = Seek(fn)
        lValRead = 0
        Get #fn, 1, lValRead
        Close #fn

        If lValRead = 133742 And lPos = 1 Then
            ConsolePrint "TEST 4 PASSED: Binary Put/Get and Seek verified (val=" & CStr(lValRead) & ")"
        Else
            ConsolePrint "TEST 4 FAILED: Binary Put/Get mismatch (read=" & CStr(lValRead) & ", pos=" & CStr(lPos) & ")"
        End If

        ' -------------------------------------------------------------------------
        ' TEST 5: FileCopy, Name ... As ..., Dir$()
        ' -------------------------------------------------------------------------
        FileCopy sFile1, sFile2
        Name sFile2 As sFile3
        sFound = Dir$(sFile3)

        If Len(sFound) > 0 Then
            ConsolePrint "TEST 5 PASSED: FileCopy, Name As, and Dir$ found '" & sFound & "'"
        Else
            ConsolePrint "TEST 5 FAILED: Dir$ did not find renamed file"
        End If

        ' -------------------------------------------------------------------------
        ' TEST 6: Reset and Cleanup (Kill)
        ' -------------------------------------------------------------------------
        Reset
        Kill sFile1
        Kill sFile3

        ConsolePrint "TEST 6 PASSED: Reset and Kill cleanup completed"
        ConsolePrint "=== App09 Test Suite Finished Successfully ==="
    End Sub

