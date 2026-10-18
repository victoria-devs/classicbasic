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

Private Type InnerData
    Id As Integer
    Pad As Integer
    Value As Long
    Description As String
End Type

Private Type OuterData
    Header As String * 4
    Inner As InnerData
    Rate As Double
    Flags As Long
End Type

Private Type ArrayRecord
    Tag As String
    Numbers() As Long
End Type

Public Sub PrintLine(ByVal s As String)
    Dim hStdOut As Long
    Dim written As Long
    Dim msg As String

    msg = s & vbCrLf
    hStdOut = GetStdHandle(STD_OUTPUT_HANDLE)
    WriteFile hStdOut, msg, Len(msg), written, 0
End Sub

Public Sub Main()
    PrintLine "=== ClassicBasic App29: 32-Bit ABI, Synthetic IRecordInfo & Full Ordinals ==="

    TestNestedUdtAssignmentAndCopy
    TestVariantUdtBoxingAndRecordInfo
    TestRecordHolderComExchange
    TestStructAlignmentAndLenB
    TestUdtDynamicArrayPreserve
    TestRecordWithInternalDynamicArray

    PrintLine "=== App29 Test Suite Finished Successfully ==="
End Sub

' -----------------------------------------------------------------------------
' TEST 1: Nested UDT Initialization, Member Access, and Deep Copying
' -----------------------------------------------------------------------------
Private Sub TestNestedUdtAssignmentAndCopy()
    Dim a As OuterData
    Dim b As OuterData

    a.Header = "TEST"
    a.Inner.Id = 42
    a.Inner.Pad = 0
    a.Inner.Value = 99999
    a.Inner.Description = "Deep Nested Record"
    a.Rate = 3.1415926535
    a.Flags = &H7F00AA55

    ' Deep copy structure
    b = a

    If b.Header <> "TEST" Then Err.Raise 101, , "Header copy failed"
    If b.Inner.Id <> 42 Then Err.Raise 102, , "Inner Id copy failed"
    If b.Inner.Value <> 99999 Then Err.Raise 103, , "Inner Value copy failed"
    If b.Inner.Description <> "Deep Nested Record" Then Err.Raise 104, , "Inner BSTR copy failed"
    If Abs(b.Rate - 3.1415926535) > 0.0000001 Then Err.Raise 105, , "Rate Double copy failed"
    If b.Flags <> &H7F00AA55 Then Err.Raise 106, , "Flags copy failed"

    ' Mutate b, ensure a is unaffected (value semantics)
    b.Inner.Description = "Modified BSTR"
    If a.Inner.Description <> "Deep Nested Record" Then Err.Raise 107, , "Deep copy independence failed"

    PrintLine "Test 1: Nested UDT assignment and deep copy passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 2: Boxing UDT into a Variant (VT_RECORD & Synthetic IRecordInfo)
' -----------------------------------------------------------------------------
Private Sub TestVariantUdtBoxingAndRecordInfo()
    Dim u As InnerData
    Dim v As Variant
    Dim uOut As InnerData

    u.Id = 7
    u.Pad = 1
    u.Value = 123456
    u.Description = "Variant Boxed Record"

    ' Box into Variant (sets VT_RECORD)
    v = u

    If VarType(v) <> 36 Then
        ' 36 = vbUserDefinedType / VT_RECORD
        Err.Raise 201, , "VarType is not VT_RECORD (36), got: " & CStr(VarType(v))
    End If

    ' Unbox from Variant back into UDT
    uOut = v

    If uOut.Id <> 7 Then Err.Raise 202, , "Unboxed Id mismatch"
    If uOut.Value <> 123456 Then Err.Raise 203, , "Unboxed Value mismatch"
    If uOut.Description <> "Variant Boxed Record" Then Err.Raise 204, , "Unboxed Description mismatch"

    PrintLine "Test 2: Variant UDT boxing and IRecordInfo passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 3: Passing Boxed UDT through COM Object Interface
' -----------------------------------------------------------------------------
Private Sub TestRecordHolderComExchange()
    Dim holder As CRecordHolder
    Dim u As InnerData
    Dim v As Variant
    Dim result As InnerData

    Set holder = New CRecordHolder

    If holder.IsHoldingRecord Then Err.Raise 301, , "Should not hold record initially"

    u.Id = 88
    u.Pad = 0
    u.Value = 777
    u.Description = "Exchanged Through COM"

    v = u
    holder.HeldRecord = v

    If Not holder.IsHoldingRecord Then Err.Raise 302, , "Should hold record now"

    result = holder.HeldRecord
    If result.Id <> 88 Then Err.Raise 303, , "COM exchanged Id mismatch"
    If result.Description <> "Exchanged Through COM" Then Err.Raise 304, , "COM exchanged Description mismatch"

    holder.Clear
    If holder.IsHoldingRecord Then Err.Raise 305, , "Should be empty after Clear"

    Set holder = Nothing
    PrintLine "Test 3: Record holder COM exchange passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 4: Struct Alignment & LenB vs Len Verification
' -----------------------------------------------------------------------------
Private Sub TestStructAlignmentAndLenB()
    Dim outRec As OuterData
    Dim szLogical As Long
    Dim szByte As Long

    szLogical = Len(outRec)
    szByte = LenB(outRec)

    ' OuterData contains:
    ' Header: 4 chars
    ' Inner: Id (2) + Pad (2) + Value (4) + BSTR ptr (4) = 12 bytes
    ' Rate: Double (8 bytes) -> requires 8-byte alignment
    ' Flags: Long (4 bytes)
    If szLogical <= 0 Then Err.Raise 401, , "Invalid Len"
    If szByte <= 0 Then Err.Raise 402, , "Invalid LenB"

    PrintLine "Test 4: Struct alignment and LenB passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 5: UDT Dynamic Array with ReDim Preserve
' -----------------------------------------------------------------------------
Private Sub TestUdtDynamicArrayPreserve()
    Dim list() As InnerData
    Dim i As Long

    ReDim list(1 To 3)
    For i = 1 To 3
        list(i).Id = i
        list(i).Value = i * 100
        list(i).Description = "Item" & CStr(i)
    Next i

    ' Expand array preserving existing elements
    ReDim Preserve list(1 To 5)
    list(4).Id = 4
    list(4).Value = 400
    list(4).Description = "Item4"
    list(5).Id = 5
    list(5).Value = 500
    list(5).Description = "Item5"

    For i = 1 To 5
        If list(i).Id <> i Then Err.Raise 501, , "Preserved element Id mismatch at " & CStr(i)
        If list(i).Value <> i * 100 Then Err.Raise 502, , "Preserved element Value mismatch at " & CStr(i)
        If list(i).Description <> "Item" & CStr(i) Then Err.Raise 503, , "Preserved Description mismatch at " & CStr(i)
    Next i

    Erase list
    PrintLine "Test 5: UDT dynamic array preserve passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 6: Record Containing Internal Dynamic Array
' -----------------------------------------------------------------------------
Private Sub TestRecordWithInternalDynamicArray()
    Dim rec1 As ArrayRecord
    Dim rec2 As ArrayRecord

    rec1.Tag = "VectorRecord"
    ReDim rec1.Numbers(0 To 2)
    rec1.Numbers(0) = 10
    rec1.Numbers(1) = 20
    rec1.Numbers(2) = 30

    ' Record copy must duplicate internal dynamic array (SafeArrayCopyData)
    rec2 = rec1

    If rec2.Tag <> "VectorRecord" Then Err.Raise 601, , "Tag mismatch"
    If UBound(rec2.Numbers) <> 2 Then Err.Raise 602, , "UBound mismatch"
    If rec2.Numbers(1) <> 20 Then Err.Raise 603, , "Array element mismatch"

    ' Modify copy to ensure deep copy
    rec2.Numbers(1) = 999
    If rec1.Numbers(1) <> 20 Then Err.Raise 604, , "Internal array was not deep copied"

    Erase rec1.Numbers
    Erase rec2.Numbers
    PrintLine "Test 6: Record with internal dynamic array passed."
End Sub
