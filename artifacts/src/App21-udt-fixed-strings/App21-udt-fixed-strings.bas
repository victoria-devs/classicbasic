    Attribute VB_Name = "Module1"
    Option Explicit

    Private Declare Function GetStdHandle Lib "kernel32" (ByVal nStdHandle As Long) As Long
    Private Declare Function WriteFile Lib "kernel32" ( _
        ByVal hFile As Long, _
        ByVal lpBuffer As String, _
        ByVal nNumberOfBytesToWrite As Long, _
        lpNumberOfBytesWritten As Long, _
        ByVal lpOverlapped As Long) As Long
    Private Declare Sub GetStartupInfoA Lib "kernel32" (lpStartupInfo As Any)

    Private Const STD_OUTPUT_HANDLE As Long = -11

    ' Nested UDT with fixed-length string
    Private Type AuditMeta
        CreatedBy As String * 8
        Revision As Integer
    End Type

    ' Complex UDT containing primitives, fixed string, dynamic BSTR, static array, and nested UDT
    ' Exercises __vbaRecAssign, __vbaRecDestruct, CopyRecord, __vbaAryRecCopy
    Private Type EmployeeRecord
        Id As Long
        Code As String * 6
        FullName As String
        Salary As Currency
        Active As Boolean
        Tags(1 To 3) As String
        Meta As AuditMeta
    End Type

    ' Binary layout UDTs for LSet raw memory punning (__vbaCopyBytes)
    Private Type LongWord
        Value As Long
    End Type

    Private Type SplitBytes
        B0 As Byte
        B1 As Byte
        B2 As Byte
        B3 As Byte
    End Type

    ' UDT passed to ANSI Declare function to trigger __vbaRecUniToAnsi / __vbaRecAnsiToUni / __vbaRecDestructAnsi
    Private Type StartupInfoUdt
        cb As Long
        lpReserved As String
        lpDesktop As String
        lpTitle As String
        dwX As Long
        dwY As Long
        dwXSize As Long
        dwYSize As Long
        dwXCountChars As Long
        dwYCountChars As Long
        dwFillAttribute As Long
        dwFlags As Long
        wShowWindow As Integer
        cbReserved2 As Integer
        lpReserved2 As Long
        hStdInput As Long
        hStdOutput As Long
        hStdError As Long
    End Type

    Private Sub PrintLine(ByVal sText As String)
        Dim hStdOut As Long
        Dim bytesWritten As Long
        Dim sOut As String

        hStdOut = GetStdHandle(STD_OUTPUT_HANDLE)
        sOut = sText & vbCrLf
        WriteFile hStdOut, sOut, Len(sOut), bytesWritten, 0
    End Sub

    Private Sub Assert(ByVal bCondition As Boolean, ByVal sMsg As String)
        If Not bCondition Then
            PrintLine "ASSERTION FAILED: " & sMsg
            Err.Raise 9999, "Assert", "Assertion failed: " & sMsg
        End If
    End Sub

    Public Sub Main()
        PrintLine "=== ClassicBasic App21: User-Defined Types (UDT) & Fixed Strings ==="

        TestFixedLengthStrings
        TestLSetAndRSetAlignment
        TestUdtAssignmentAndDeepCopy
        TestLSetUdtBytePunning
        TestUdtDynamicArraysAndCopy
        TestFixedStringAndUdtBinaryFileIO
        TestUdtAnsiUnicodeMarshaling

        PrintLine "=== App21 Test Suite Finished Successfully ==="
    End Sub

    ' -------------------------------------------------------------------------
    ' Test 1: Fixed-Length Strings (String * N)
    ' Exercises __vbaFixstrConstruct, __vbaLsetFixstr, __vbaStrFixstr
    ' -------------------------------------------------------------------------
    Private Sub TestFixedLengthStrings()
        Dim sFixed10 As String * 10
        Dim sFixed5 As String * 5
        Dim sDyn As String

        ' 1. Right-padding when assigning shorter string
        sFixed10 = "ABC"
        Assert Len(sFixed10) = 10, "Len(String * 10) must always be 10"
        Assert sFixed10 = "ABC       ", "Short assignment to String * 10 must right-pad with spaces"

        ' 2. Truncation when assigning longer string
        sFixed5 = "0123456789"
        Assert Len(sFixed5) = 5, "Len(String * 5) must always be 5"
        Assert sFixed5 = "01234", "Long assignment to String * 5 must truncate to 5 chars"

        ' 3. Coercion to dynamic BSTR and Trim$
        sDyn = Trim$(sFixed10)
        Assert Len(sDyn) = 3, "Trim$(sFixed10) must yield length 3"
        Assert sDyn = "ABC", "Trim$(sFixed10) must equal 'ABC'"

        ' 4. Assigning empty / vbNullString fills with spaces
        sFixed5 = vbNullString
        Assert sFixed5 = "     ", "Assigning vbNullString to String * 5 must fill with 5 spaces"

        PrintLine "TEST 1: FIXED_LENGTH_STRINGS=PASS"
    End Sub

    ' -------------------------------------------------------------------------
    ' Test 2: LSet and RSet String Alignment
    ' Exercises __vbaLsetFixstr, __vbaLsetFixstrFree, __vbaRsetFixstr, __vbaRsetFixstrFree
    ' -------------------------------------------------------------------------
    Private Sub TestLSetAndRSetAlignment()
        Dim sBuf8 As String * 8
        Dim sDynBuf As String

        ' RSet on fixed-length string (right-align, left-pad with spaces)
        RSet sBuf8 = "42"
        Assert sBuf8 = "      42", "RSet '42' into String * 8 must equal '      42'"

        ' LSet on fixed-length string (left-align, right-pad with spaces)
        LSet sBuf8 = "42"
        Assert sBuf8 = "42      ", "LSet '42' into String * 8 must equal '42      '"

        ' RSet truncation when source is longer than target
        RSet sBuf8 = "1234567890"
        Assert sBuf8 = "12345678", "RSet longer string into String * 8 must keep first 8 chars"

        ' LSet and RSet on dynamic strings preserve target string length
        sDynBuf = "........"
        LSet sDynBuf = "XY"
        Assert sDynBuf = "XY      ", "LSet on dynamic 8-char string must yield 'XY      '"

        RSet sDynBuf = "99"
        Assert sDynBuf = "      99", "RSet on dynamic 8-char string must yield '      99'"

        PrintLine "TEST 2: LSET_RSET_ALIGNMENT=PASS"
    End Sub

    ' -------------------------------------------------------------------------
    ' Test 3: UDT Value Copy, Deep BSTR/Array Cloning & Scope Destruction
    ' Exercises __vbaRecAssign, CopyRecord, __vbaRecDestruct
    ' -------------------------------------------------------------------------
    Private Sub TestUdtAssignmentAndDeepCopy()
        Dim emp1 As EmployeeRecord
        Dim emp2 As EmployeeRecord

        emp1.Id = 1001
        emp1.Code = "ENG01"
        emp1.FullName = "Ada Lovelace"
        emp1.Salary = 125000.5@
        emp1.Active = True
        emp1.Tags(1) = "Rust"
        emp1.Tags(2) = "VB6"
        emp1.Tags(3) = "ASM"
        emp1.Meta.CreatedBy = "SYSTEM"
        emp1.Meta.Revision = 3

        ' Deep copy UDT record
        emp2 = emp1

        ' Mutate emp2 to prove emp1 was deep-copied without pointer aliasing
        emp2.Id = 1002
        emp2.Code = "QA99"
        emp2.FullName = "Grace Hopper"
        emp2.Tags(1) = "COBOL"
        emp2.Meta.CreatedBy = "ADMIN"
        emp2.Meta.Revision = 4

        Assert emp1.Id = 1001, "emp1.Id must remain 1001"
        Assert emp1.Code = "ENG01 ", "emp1.Code must remain 'ENG01 '"
        Assert emp1.FullName = "Ada Lovelace", "emp1.FullName must remain 'Ada Lovelace'"
        Assert emp1.Salary = 125000.5@, "emp1.Salary must remain 125000.5"
        Assert emp1.Active = True, "emp1.Active must remain True"
        Assert emp1.Tags(1) = "Rust", "emp1.Tags(1) must not be mutated by emp2"
        Assert emp1.Meta.CreatedBy = "SYSTEM  ", "emp1.Meta.CreatedBy must remain 'SYSTEM  '"
        Assert emp1.Meta.Revision = 3, "emp1.Meta.Revision must remain 3"

        Assert emp2.Id = 1002, "emp2.Id must be 1002"
        Assert emp2.Code = "QA99  ", "emp2.Code must be 'QA99  '"
        Assert emp2.FullName = "Grace Hopper", "emp2.FullName must be 'Grace Hopper'"
        Assert emp2.Tags(1) = "COBOL", "emp2.Tags(1) must be 'COBOL'"
        Assert emp2.Meta.CreatedBy = "ADMIN   ", "emp2.Meta.CreatedBy must be 'ADMIN   '"
        Assert emp2.Meta.Revision = 4, "emp2.Meta.Revision must be 4"

        PrintLine "TEST 3: UDT_ASSIGNMENT_DEEP_COPY=PASS"
    End Sub

    ' -------------------------------------------------------------------------
    ' Test 4: LSet Raw Memory Punning Between Distinct UDTs
    ' Exercises __vbaCopyBytes / __vbaCopyBytesZero
    ' -------------------------------------------------------------------------
    Private Sub TestLSetUdtBytePunning()
        Dim lw As LongWord
        Dim sb As SplitBytes
        Dim lwRoundTrip As LongWord

        lw.Value = &H11223344
        LSet sb = lw

        Assert sb.B0 = &H44, "Little-endian byte 0 of 0x11223344 must be 0x44"
        Assert sb.B1 = &H33, "Little-endian byte 1 of 0x11223344 must be 0x33"
        Assert sb.B2 = &H22, "Little-endian byte 2 of 0x11223344 must be 0x22"
        Assert sb.B3 = &H11, "Little-endian byte 3 of 0x11223344 must be 0x11"

        sb.B0 = &HAA
        LSet lwRoundTrip = sb
        Assert lwRoundTrip.Value = &H112233AA, "Modified byte 0 must round-trip via LSet to 0x112233AA"

        PrintLine "TEST 4: LSET_UDT_BYTE_PUNNING=PASS"
    End Sub

    ' -------------------------------------------------------------------------
    ' Test 5: Dynamic Arrays of UDTs & Whole-Array Copy
    ' Exercises __vbaAryRecCopy, __vbaAryRecMove, ReDim Preserve on UDTs
    ' -------------------------------------------------------------------------
    Private Sub TestUdtDynamicArraysAndCopy()
        Dim roster() As EmployeeRecord
        Dim backup() As EmployeeRecord

        ReDim roster(1 To 2)
        roster(1).Id = 10
        roster(1).Code = "DEV1"
        roster(1).FullName = "First Engineer"
        roster(1).Tags(1) = "Core"

        roster(2).Id = 20
        roster(2).Code = "DEV2"
        roster(2).FullName = "Second Engineer"
        roster(2).Tags(1) = "COM"

        ' Resize preserving existing UDT records
        ReDim Preserve roster(1 To 3)
        roster(3).Id = 30
        roster(3).Code = "DEV3"
        roster(3).FullName = "Third Engineer"
        roster(3).Tags(1) = "GUI"

        ' Whole UDT array assignment (__vbaAryRecCopy)
        backup = roster

        ' Mutate original array element and verify backup is isolated
        roster(1).FullName = "Mutated Engineer"
        roster(1).Tags(1) = "MutatedTag"

        Assert UBound(backup) = 3, "Copied UDT array must have UBound 3"
        Assert backup(1).Id = 10, "backup(1).Id must be 10"
        Assert backup(1).Code = "DEV1  ", "backup(1).Code must be 'DEV1  '"
        Assert backup(1).FullName = "First Engineer", "backup(1).FullName must be isolated from mutation"
        Assert backup(1).Tags(1) = "Core", "backup(1).Tags(1) must be isolated from mutation"
        Assert backup(3).Id = 30, "backup(3).Id must be 30"
        Assert backup(3).FullName = "Third Engineer", "backup(3).FullName must be 'Third Engineer'"

        Erase roster
        Erase backup

        PrintLine "TEST 5: UDT_ARRAYS_AND_COPY=PASS"
    End Sub

    ' -------------------------------------------------------------------------
    ' Test 6: Binary File I/O with Fixed-Length Strings and UDTs
    ' Exercises __vbaPutFxStr3/4, __vbaGetFxStr3/4, __vbaPut3/4, __vbaGet3/4
    ' -------------------------------------------------------------------------
    Private Sub TestFixedStringAndUdtBinaryFileIO()
        Dim fn As Integer
        Dim sTempFile As String
        Dim sFixedOut As String * 12
        Dim sFixedIn As String * 12
        Dim metaOut As AuditMeta
        Dim metaIn As AuditMeta

        sTempFile = "app21_udt_test.bin"
        If Dir$(sTempFile) <> "" Then Kill sTempFile

        sFixedOut = "FIXED_HEADER"
        metaOut.CreatedBy = "VICTORIA"
        metaOut.Revision = 777

        fn = FreeFile
        Open sTempFile For Binary Access Read Write As #fn
        Put #fn, 1, sFixedOut
        Put #fn, , metaOut

        Seek #fn, 1
        Get #fn, 1, sFixedIn
        Get #fn, , metaIn
        Close #fn

        If Dir$(sTempFile) <> "" Then Kill sTempFile

        Assert sFixedIn = "FIXED_HEADER", "Binary Get # of fixed-length string must match 'FIXED_HEADER'"
        Assert metaIn.CreatedBy = "VICTORIA", "Binary Get # of UDT fixed string field must match 'VICTORIA'"
        Assert metaIn.Revision = 777, "Binary Get # of UDT integer field must match 777"

        PrintLine "TEST 6: FIXED_STRING_AND_UDT_FILE_IO=PASS"
    End Sub

    ' -------------------------------------------------------------------------
    ' Test 7: UDT ANSI <-> Unicode Marshaling across Win32 Declare Boundary
    ' Exercises __vbaRecUniToAnsi, __vbaRecAnsiToUni, __vbaRecDestructAnsi
    ' -------------------------------------------------------------------------
    Private Sub TestUdtAnsiUnicodeMarshaling()
        Dim si As StartupInfoUdt
        si.cb = LenB(si)
        GetStartupInfoA si
        Assert si.cb > 0, "GetStartupInfoA with UDT must preserve struct size"

        PrintLine "TEST 7: UDT_ANSI_UNICODE_MARSHALING=PASS"
    End Sub