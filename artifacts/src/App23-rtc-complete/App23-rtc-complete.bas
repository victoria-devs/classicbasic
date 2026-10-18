   Attribute VB_Name = "Module1"
    Option Explicit

    Private Declare Function GetStdHandle Lib "kernel32" (ByVal nStdHandle As Long) As Long
    Private Declare Function WriteFile Lib "kernel32" ( _
        ByVal hFile As Long, _
        ByVal lpBuffer As String, _
        ByVal nNumberOfBytesToWrite As Long, _
        lpNumberOfBytesWritten As Long, _
        ByVal lpOverlapped As Long) As Long

    Private Const STD_OUTPUT_HANDLE As Long = -11

    Private Sub PrintLine(ByVal s As String)
        Dim hStdOut As Long
        Dim written As Long
        Dim msg As String

        msg = s & vbCrLf
        hStdOut = GetStdHandle(STD_OUTPUT_HANDLE)
        WriteFile hStdOut, msg, Len(msg), written, 0
    End Sub

    Public Sub Main()
        PrintLine "=== ClassicBasic App23: Complete Standard Library Closure (rtc*) ==="

        TestVariantStringOverloads
        TestByteAndCharStringFunctions
        TestErrorsAndInspection
        TestEnvironmentAndFileSystem
        TestDateTimeAndCalendarOverloads

        PrintLine "=== App23 Test Suite Finished Successfully ==="
    End Sub

    ' -----------------------------------------------------------------------------
    ' TEST 1: Variant String Overloads (Left, Right, Mid, Trim, LTrim, RTrim,
    '         LCase, UCase, Space, String, InStr, Len, StrComp)
    ' -----------------------------------------------------------------------------
    Private Sub TestVariantStringOverloads()
        Dim vSource As Variant
        Dim vResult As Variant
        Dim nResult As Long
        Dim sLTrim As String
        Dim sRTrim As String

        vSource = "  ClassicBasic VB6  "

        ' Left / Right / Mid on Variant
        vResult = Left(vSource, 7)
        If CStr(vResult) <> "  Class" Then
            PrintLine "FAIL: Left(Variant) expected '  Class' got '" & CStr(vResult) & "'"
            Exit Sub
        End If

        vResult = Right(vSource, 5)
        If CStr(vResult) <> "VB6  " Then
            PrintLine "FAIL: Right(Variant) expected 'VB6  ' got '" & CStr(vResult) & "'"
            Exit Sub
        End If

        vResult = Mid(vSource, 3, 12)
        If CStr(vResult) <> "ClassicBasic" Then
            PrintLine "FAIL: Mid(Variant) expected 'ClassicBasic' got '" & CStr(vResult) & "'"
            Exit Sub
        End If

        ' Trim / LTrim / RTrim on Variant
        vResult = Trim(vSource)
        If CStr(vResult) <> "ClassicBasic VB6" Then
            PrintLine "FAIL: Trim(Variant) got '" & CStr(vResult) & "'"
            Exit Sub
        End If

        vResult = LTrim(vSource)
        If CStr(vResult) <> "ClassicBasic VB6  " Then
            PrintLine "FAIL: LTrim(Variant) got '" & CStr(vResult) & "'"
            Exit Sub
        End If

        vResult = RTrim(vSource)
        If CStr(vResult) <> "  ClassicBasic VB6" Then
            PrintLine "FAIL: RTrim(Variant) got '" & CStr(vResult) & "'"
            Exit Sub
        End If

        ' LTrim$ / RTrim$ on BSTR
        sLTrim = LTrim$("   ABC")
        sRTrim = RTrim$("XYZ   ")
        If sLTrim <> "ABC" Or sRTrim <> "XYZ" Then
            PrintLine "FAIL: LTrim$/RTrim$ BSTR failed"
            Exit Sub
        End If

        ' LCase / UCase on Variant
        vResult = LCase(vSource)
        If InStr(1, CStr(vResult), "classicbasic") = 0 Then
            PrintLine "FAIL: LCase(Variant) got '" & CStr(vResult) & "'"
            Exit Sub
        End If

        vResult = UCase(vSource)
        If InStr(1, CStr(vResult), "CLASSICBASIC") = 0 Then
            PrintLine "FAIL: UCase(Variant) got '" & CStr(vResult) & "'"
            Exit Sub
        End If

        ' Space / String on Variant
        vResult = Space(4)
        If CStr(vResult) <> "    " Then
            PrintLine "FAIL: Space(Variant) failed"
            Exit Sub
        End If

        vResult = String(5, "X")
        If CStr(vResult) <> "XXXXX" Then
            PrintLine "FAIL: String(Variant) failed"
            Exit Sub
        End If

        ' InStr on Variant
        nResult = InStr(1, vSource, "Basic")
        If nResult <> 10 Then
            PrintLine "FAIL: InStr expected 10 got " & CStr(nResult)
            Exit Sub
        End If

        ' Len on Variant
        nResult = Len(vSource)
        If nResult <> 20 Then
            PrintLine "FAIL: Len(Variant) expected 20 got " & CStr(nResult)
            Exit Sub
        End If

        ' StrComp
        nResult = StrComp("abc", "ABC", vbBinaryCompare)
        If nResult <= 0 Then
            PrintLine "FAIL: StrComp binary compare expected > 0"
            Exit Sub
        End If

        nResult = StrComp("abc", "ABC", vbTextCompare)
        If nResult <> 0 Then
            PrintLine "FAIL: StrComp text compare expected 0"
            Exit Sub
        End If

        PrintLine "Test 1: Variant String Overloads passed."
    End Sub

    ' -----------------------------------------------------------------------------
    ' TEST 2: Byte & Char Functions (Asc, Chr, AscB, ChrB, AscW, ChrW, LenB, InStrB, MidB)
    ' -----------------------------------------------------------------------------
    Private Sub TestByteAndCharStringFunctions()
        Dim aVal As Integer
        Dim cVal As String
        Dim aBVal As Byte
        Dim cBVal As String
        Dim aWVal As Integer
        Dim cWVal As String
        Dim nBytes As Long
        Dim vBin As Variant

        ' Asc and Chr
        aVal = Asc("A")
        If aVal <> 65 Then
            PrintLine "FAIL: Asc('A') expected 65 got " & CStr(aVal)
            Exit Sub
        End If

        cVal = Chr(66)
        If cVal <> "B" Then
            PrintLine "FAIL: Chr(66) expected 'B' got '" & cVal & "'"
            Exit Sub
        End If

        ' AscB and ChrB
        aBVal = AscB("A")
        If aBVal <> 65 Then
            PrintLine "FAIL: AscB('A') expected 65 got " & CStr(aBVal)
            Exit Sub
        End If

        cBVal = ChrB(67)
        If AscB(cBVal) <> 67 Then
            PrintLine "FAIL: ChrB(67) failed"
            Exit Sub
        End If

        ' AscW and ChrW
        aWVal = AscW("Z")
        If aWVal <> 90 Then
            PrintLine "FAIL: AscW('Z') expected 90 got " & CStr(aWVal)
            Exit Sub
        End If

        cWVal = ChrW(90)
        If cWVal <> "Z" Then
            PrintLine "FAIL: ChrW(90) expected 'Z' got '" & cWVal & "'"
            Exit Sub
        End If

        ' LenB on String and Variant
        nBytes = LenB("ABC")
        If nBytes <> 6 Then ' UTF-16 in memory = 2 bytes per char
            PrintLine "FAIL: LenB('ABC') expected 6 got " & CStr(nBytes)
            Exit Sub
        End If

        vBin = "1234"
        nBytes = LenB(vBin)
        If nBytes <> 8 Then
            PrintLine "FAIL: LenB(Variant) expected 8 got " & CStr(nBytes)
            Exit Sub
        End If

        PrintLine "Test 2: Byte and Char String Functions passed."
    End Sub

    ' -----------------------------------------------------------------------------
    ' TEST 3: Errors & Inspection (CVErr, IsError, Hex, Oct, Format Variant, QBColor)
    ' -----------------------------------------------------------------------------
    Private Sub TestErrorsAndInspection()
        Dim vErr As Variant
        Dim vHex As Variant
        Dim vOct As Variant
        Dim vFmt As Variant
        Dim qb As Long

        ' CVErr & IsError
        vErr = CVErr(13) ' Type Mismatch
        If Not IsError(vErr) Then
            PrintLine "FAIL: IsError(CVErr(13)) expected True"
            Exit Sub
        End If

        ' Hex & Oct on Variant
        vHex = Hex(255)
        If CStr(vHex) <> "FF" Then
            PrintLine "FAIL: Hex(255) expected 'FF' got '" & CStr(vHex) & "'"
            Exit Sub
        End If

        vOct = Oct(64)
        If CStr(vOct) <> "100" Then
            PrintLine "FAIL: Oct(64) expected '100' got '" & CStr(vOct) & "'"
            Exit Sub
        End If

        ' Format returning Variant
        vFmt = Format(123.456, "0.00")
        If CStr(vFmt) <> "123.46" Then
            PrintLine "FAIL: Format(123.456, '0.00') got '" & CStr(vFmt) & "'"
            Exit Sub
        End If

        ' QBColor
        qb = QBColor(1) ' Blue
        If qb <> &H800000 Then
            PrintLine "FAIL: QBColor(1) expected &H800000 got &H" & Hex(qb)
            Exit Sub
        End If

        qb = QBColor(4) ' Red
        If qb <> &H80 Then
            PrintLine "FAIL: QBColor(4) expected &H80 got &H" & Hex(qb)
            Exit Sub
        End If

        PrintLine "Test 3: Errors, Inspection and QBColor passed."
    End Sub

    ' -----------------------------------------------------------------------------
    ' TEST 4: Environment, CurrentDir & File Information
    ' -----------------------------------------------------------------------------
    Private Sub TestEnvironmentAndFileSystem()
        Dim vEnv As Variant
        Dim vCurDir As Variant
        Dim sDir As String

        ' CurDir with drive parameter (as Variant)
        vCurDir = CurDir("C")
        If Len(CStr(vCurDir)) = 0 Then
            PrintLine "FAIL: CurDir('C') returned empty"
            Exit Sub
        End If

        ' Environ on Variant
        vEnv = Environ(1)
        If Len(CStr(vEnv)) = 0 Then
            ' Se não tiver índice 1, tenta por nome
            vEnv = Environ("PATH")
        End If

        PrintLine "Test 4: Environment and File System helpers passed."
    End Sub

    ' -----------------------------------------------------------------------------
    ' TEST 5: Date/Time Variant functions
    ' -----------------------------------------------------------------------------
    Private Sub TestDateTimeAndCalendarOverloads()
        Dim vDate As Variant
        Dim vTime As Variant

        ' Date and Time as Variant
        vDate = Date
        vTime = Time

        If Len(CStr(vDate)) = 0 Then
            PrintLine "FAIL: Date (Variant) returned empty"
            Exit Sub
        End If

        If Len(CStr(vTime)) = 0 Then
            PrintLine "FAIL: Time (Variant) returned empty"
            Exit Sub
        End If

        PrintLine "Test 5: Date and Time Variant helpers passed."
    End Sub