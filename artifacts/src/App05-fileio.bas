Attribute VB_Name = "Module1"

    Option Explicit

    ' ============================================================================
    ' Win32 KERNEL32 API Declarations
    ' ============================================================================
    Private Declare Function AllocConsole Lib "kernel32" () As Long
    Private Declare Function GetStdHandle Lib "kernel32" (ByVal nStdHandle As Long) As Long
    Private Declare Function WriteConsoleA Lib "kernel32" ( _
        ByVal hConsoleOutput As Long, _
        ByVal lpBuffer As String, _
        ByVal nNumberOfCharsToWrite As Long, _
        lpNumberOfCharsWritten As Long, _
        ByVal lpReserved As Long) As Long

    Private Declare Function CreateFileA Lib "kernel32" ( _
        ByVal lpFileName As String, _
        ByVal dwDesiredAccess As Long, _
        ByVal dwShareMode As Long, _
        ByVal lpSecurityAttributes As Long, _
        ByVal dwCreationDisposition As Long, _
        ByVal dwFlagsAndAttributes As Long, _
        ByVal hTemplateFile As Long) As Long

    Private Declare Function WriteFile Lib "kernel32" ( _
        ByVal hFile As Long, _
        ByVal lpBuffer As String, _
        ByVal nNumberOfBytesToWrite As Long, _
        lpNumberOfBytesWritten As Long, _
        ByVal lpOverlapped As Long) As Long

    Private Declare Function ReadFile Lib "kernel32" ( _
        ByVal hFile As Long, _
        ByVal lpBuffer As String, _
        ByVal nNumberOfBytesToRead As Long, _
        lpNumberOfBytesRead As Long, _
        ByVal lpOverlapped As Long) As Long

    Private Declare Function CloseHandle Lib "kernel32" (ByVal hObject As Long) As Long
    Private Declare Function GetFileSize Lib "kernel32" (ByVal hFile As Long, lpFileSizeHigh As Long) As Long

    ' ============================================================================
    ' Win32 Constants
    ' ============================================================================
    Private Const STD_OUTPUT_HANDLE As Long = -11
    Private Const GENERIC_READ As Long = &H80000000
    Private Const GENERIC_WRITE As Long = &H40000000
    Private Const FILE_SHARE_READ As Long = &H1
    Private Const CREATE_ALWAYS As Long = 2
    Private Const OPEN_EXISTING As Long = 3
    Private Const FILE_ATTRIBUTE_NORMAL As Long = &H80
    Private Const INVALID_HANDLE_VALUE As Long = -1

    Private hStdOut As Long

    ' Helper para exibir saída no console Linux
    Public Sub PrintLine(ByVal text As String)
        Dim written As Long
        text = text & vbCrLf
        WriteConsoleA hStdOut, text, Len(text), written, 0
    End Sub

    Public Sub Main()
        AllocConsole
        hStdOut = GetStdHandle(STD_OUTPUT_HANDLE)

        Dim fileName As String
        fileName = "classicbasic_test.txt"

        ' ------------------------------------------------------------------------
        ' [Test 1] CreateFileA (CREATE_ALWAYS) & WriteFile
        ' ------------------------------------------------------------------------
        Dim hFileWrite As Long
        hFileWrite = CreateFileA(fileName, GENERIC_WRITE, 0, 0, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, 0)

        If hFileWrite <> INVALID_HANDLE_VALUE Then
            PrintLine "[Test 1] CreateFileA (Write): Handle OK"
        Else
            PrintLine "[Test 1] CreateFileA (Write): Failed"
        End If

        Dim payload As String
        payload = "ClassicBasic Linux Native File I/O!"
        Dim bytesWritten As Long
        WriteFile hFileWrite, payload, Len(payload), bytesWritten, 0
        PrintLine "[Test 1] WriteFile bytes: " & CStr(bytesWritten)

        ' ------------------------------------------------------------------------
        ' [Test 2] CloseHandle (Write)
        ' ------------------------------------------------------------------------
        Dim closeRes As Long
        closeRes = CloseHandle(hFileWrite)
        PrintLine "[Test 2] CloseHandle (Write): " & CStr(closeRes)

        ' ------------------------------------------------------------------------
        ' [Test 3] CreateFileA (OPEN_EXISTING) & GetFileSize
        ' ------------------------------------------------------------------------
        Dim hFileRead As Long
        hFileRead = CreateFileA(fileName, GENERIC_READ, FILE_SHARE_READ, 0, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, 0)

        If hFileRead <> INVALID_HANDLE_VALUE Then
            PrintLine "[Test 3] CreateFileA (Read): Handle OK"
        Else
            PrintLine "[Test 3] CreateFileA (Read): Failed"
        End If

        Dim sizeHigh As Long
        Dim fileSize As Long
        fileSize = GetFileSize(hFileRead, sizeHigh)
        PrintLine "[Test 3] GetFileSize: " & CStr(fileSize)

        ' ------------------------------------------------------------------------
        ' [Test 4] ReadFile
        ' ------------------------------------------------------------------------
        Dim readBuffer As String
        readBuffer = Space$(fileSize) ' pré-aloca buffer de leitura
        Dim bytesRead As Long
        ReadFile hFileRead, readBuffer, fileSize, bytesRead, 0
        PrintLine "[Test 4] ReadFile bytes: " & CStr(bytesRead)
        PrintLine "[Test 4] Content: " & readBuffer

        CloseHandle hFileRead

        ' ------------------------------------------------------------------------
        ' [Test 5] Error Handling (Abrir arquivo inexistente)
        ' ------------------------------------------------------------------------
        Dim hMissing As Long
        hMissing = CreateFileA("non_existent_file_xyz.tmp", GENERIC_READ, 0, 0, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, 0)
        PrintLine "[Test 5] Open NonExistent: " & CStr(hMissing)
    End Sub


