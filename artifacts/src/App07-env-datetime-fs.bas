Attribute VB_Name = "Module1"
Option Explicit

    ' --- Win32 Console Output APIs ---
    Private Declare Function AllocConsole Lib "kernel32" () As Long
    Private Declare Function GetStdHandle Lib "kernel32" (ByVal nStdHandle As Long) As Long
    Private Declare Function WriteFile Lib "kernel32" ( _
        ByVal hFile As Long, _
        ByVal lpBuffer As String, _
        ByVal nNumberOfBytesToWrite As Long, _
        ByRef lpNumberOfBytesWritten As Long, _
        ByVal lpOverlapped As Long) As Long

    Private Const STD_OUTPUT_HANDLE As Long = -11&
    Private hStdOut As Long

    Private Sub PrintLine(ByVal sText As String)
        Dim sOut As String
        Dim written As Long
        sOut = sText & vbCrLf
        WriteFile hStdOut, sOut, Len(sOut), written, 0&
    End Sub

    Public Sub Main()
        AllocConsole
        hStdOut = GetStdHandle(STD_OUTPUT_HANDLE)

        PrintLine "=== ClassicBasic App07: Environment, Date/Time & Filesystem Suite ==="

        ' ========================================================
        ' [Test 1] Environment Variables (Environ$)
        ' ========================================================
        Dim sPath As String
        sPath = Environ$("PATH")
        If Len(sPath) > 0 Then
            PrintLine "[Test 1] Environ$(PATH): OK (Length: " & Len(sPath) & ")"
        Else
            PrintLine "[Test 1] Environ$(PATH): Empty or Failed"
        End If

        ' ========================================================
        ' [Test 2] Working Directory (CurDir$)
        ' ========================================================
        Dim sCurDir As String
        sCurDir = CurDir$
        If Len(sCurDir) > 0 Then
            PrintLine "[Test 2] CurDir$: OK (Length: " & Len(sCurDir) & ")"
        Else
            PrintLine "[Test 2] CurDir$: Empty or Failed"
        End If

        ' ========================================================
        ' [Test 3] Date and Time Strings (Date$, Time$, Now)
        ' ========================================================
        Dim sDate As String, sTime As String, dNow As Date
        sDate = Date$
        sTime = Time$
        dNow = Now

        ' Date$ tem formato MM-DD-YYYY (10 chars), Time$ tem formato HH:MM:SS (8 chars)
        If Len(sDate) = 10 And Len(sTime) = 8 Then
            PrintLine "[Test 3] Date$/Time$: OK (Date=" & sDate & ", Time=" & sTime & ")"
        Else
            PrintLine "[Test 3] Date$/Time$: Invalid Format (" & sDate & " " & sTime & ")"
        End If

        ' ========================================================
        ' [Test 4] Timer (Seconds since midnight)
        ' ========================================================
        Dim tVal As Single
        tVal = Timer
        If tVal >= 0# And tVal <= 86400# Then
            PrintLine "[Test 4] Timer: OK (Seconds: " & Int(tVal) & ")"
        Else
            PrintLine "[Test 4] Timer: Failed (" & tVal & ")"
        End If

        ' ========================================================
        ' [Test 5] Directory Operations (MkDir & RmDir)
        ' ========================================================
        On Error Resume Next
        Dim testDir As String
        testDir = "classicbasic_test_dir"

        ' Cria diretório
        MkDir testDir
        If Err.Number = 0 Then
            PrintLine "[Test 5] MkDir: OK"
        Else
            PrintLine "[Test 5] MkDir: Failed (Err " & Err.Number & ")"
        End If

        ' Remove diretório criado
        RmDir testDir
        If Err.Number = 0 Then
            PrintLine "[Test 5] RmDir: OK"
        Else
            PrintLine "[Test 5] RmDir: Failed (Err " & Err.Number & ")"
        End If

        PrintLine "=== All Environment, Date/Time & Filesystem Tests Completed ==="
    End Sub

