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

Public Sub PrintLine(ByVal s As String)
    Dim hStdOut As Long
    Dim written As Long
    Dim msg As String

    msg = s & vbCrLf
    hStdOut = GetStdHandle(STD_OUTPUT_HANDLE)
    WriteFile hStdOut, msg, Len(msg), written, 0
End Sub

Public Sub Main()
    PrintLine "=== ClassicBasic App28: COM Event Sinks & Collection Iterators ==="

    TestEventSinkBindingAndRaising
    TestEventSinkUnhooking
    TestCollectionBasics
    TestForEachObjectIteration
    TestForEachVariantIteration
    TestNestedForEachIteration

    PrintLine "=== App28 Test Suite Finished Successfully ==="
End Sub

' -----------------------------------------------------------------------------
' TEST 1: COM Event Sink Binding and Raising Events
' -----------------------------------------------------------------------------
Private Sub TestEventSinkBindingAndRaising()
    Dim em As CEmitter
    Dim rx As CReceiver

    Set em = New CEmitter
    Set rx = New CReceiver

    rx.Hook em

    em.Trigger 101, "Initialization Event"
    If rx.ReceivedCount <> 1 Then Err.Raise 101, , "Event count mismatch (expected 1)"
    If rx.LastCode <> 101 Then Err.Raise 102, , "Event code mismatch (expected 101)"
    If rx.LastMsg <> "Initialization Event" Then Err.Raise 103, , "Event message mismatch"

    em.Ping
    If rx.ReceivedCount <> 2 Then Err.Raise 104, , "Event count mismatch (expected 2)"
    If rx.LastCode <> 999 Then Err.Raise 105, , "Ping event code mismatch"
    If rx.LastMsg <> "PING" Then Err.Raise 106, , "Ping event message mismatch"

    Set em = Nothing
    Set rx = Nothing

    PrintLine "Test 1: Event sink binding and raising passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 2: Event Sink Unhooking & Connection Detach
' -----------------------------------------------------------------------------
Private Sub TestEventSinkUnhooking()
    Dim em As CEmitter
    Dim rx As CReceiver

    Set em = New CEmitter
    Set rx = New CReceiver

    rx.Hook em
    em.Trigger 201, "First Event"
    If rx.ReceivedCount <> 1 Then Err.Raise 201, , "Event count mismatch"

    rx.Unhook
    em.Trigger 202, "Second Event (Should Be Ignored)"
    If rx.ReceivedCount <> 1 Then Err.Raise 202, , "Received event after unhooking"
    If rx.LastCode <> 201 Then Err.Raise 203, , "LastCode changed after unhooking"

    Set em = Nothing
    Set rx = Nothing

    PrintLine "Test 2: Event sink unhooking passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 3: Visual Basic Collection Basics (Add, Count, Item, Remove)
' -----------------------------------------------------------------------------
Private Sub TestCollectionBasics()
    Dim c As Collection
    Set c = New Collection

    If c.Count <> 0 Then Err.Raise 301, , "Initial collection count not 0"

    c.Add "First Item", "k1"
    c.Add "Second Item", "k2"
    c.Add "Third Item", "k3"

    If c.Count <> 3 Then Err.Raise 302, , "Count after add mismatch"
    If c.Item(1) <> "First Item" Then Err.Raise 303, , "Item(1) mismatch"
    If c.Item("k2") <> "Second Item" Then Err.Raise 304, , "Item(k2) mismatch"
    If c(3) <> "Third Item" Then Err.Raise 305, , "Default property c(3) mismatch"

    c.Remove "k2"
    If c.Count <> 2 Then Err.Raise 306, , "Count after remove mismatch"
    If c.Item(2) <> "Third Item" Then Err.Raise 307, , "Item(2) shifted mismatch"

    c.Remove 1
    If c.Count <> 1 Then Err.Raise 308, , "Count after remove 1 mismatch"
    If c.Item(1) <> "Third Item" Then Err.Raise 309, , "Remaining item mismatch"

    Set c = Nothing
    PrintLine "Test 3: Collection basics passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 4: For Each Object Iteration (__vbaForEachCollObj / NextEachCollObj)
' -----------------------------------------------------------------------------
Private Sub TestForEachObjectIteration()
    Dim c As Collection
    Dim itm As CItem
    Dim total As Long
    Dim names As String
    Dim i As Long

    Set c = New Collection
    For i = 1 To 5
        Set itm = New CItem
        itm.Key = "Obj" & CStr(i)
        itm.Value = i * 10
        c.Add itm, itm.Key
    Next i

    total = 0
    names = ""
    For Each itm In c
        total = total + itm.Value
        names = names & itm.Key & ";"
    Next itm

    If total <> 150 Then Err.Raise 401, , "For Each object sum mismatch: " & CStr(total)
    If names <> "Obj1;Obj2;Obj3;Obj4;Obj5;" Then Err.Raise 402, , "For Each object names mismatch: " & names

    ' Test early exit from For Each
    total = 0
    For Each itm In c
        total = total + itm.Value
        If itm.Value >= 30 Then Exit For
    Next itm
    If total <> 60 Then Err.Raise 403, , "Exit For sum mismatch: " & CStr(total)

    Set c = Nothing
    PrintLine "Test 4: For Each object iteration passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 5: For Each Variant Iteration (__vbaForEachCollVar / NextEachCollVar)
' -----------------------------------------------------------------------------
Private Sub TestForEachVariantIteration()
    Dim c As Collection
    Dim v As Variant
    Dim sum As Long
    Dim textConcat As String

    Set c = New Collection
    c.Add 100
    c.Add 250
    c.Add 50
    c.Add "EXTRA"

    sum = 0
    textConcat = ""
    For Each v In c
        If IsNumeric(v) Then
            sum = sum + CLng(v)
        Else
            textConcat = textConcat & CStr(v)
        End If
    Next v

    If sum <> 400 Then Err.Raise 501, , "Variant For Each sum mismatch: " & CStr(sum)
    If textConcat <> "EXTRA" Then Err.Raise 502, , "Variant For Each text mismatch"

    Set c = Nothing
    PrintLine "Test 5: For Each variant iteration passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 6: Nested For Each Iteration
' -----------------------------------------------------------------------------
Private Sub TestNestedForEachIteration()
    Dim c1 As Collection, c2 As Collection
    Dim v1 As Variant, v2 As Variant
    Dim matchCount As Long

    Set c1 = New Collection
    Set c2 = New Collection

    c1.Add "A"
    c1.Add "B"

    c2.Add "1"
    c2.Add "2"
    c2.Add "3"

    matchCount = 0
    For Each v1 In c1
        For Each v2 In c2
            matchCount = matchCount + 1
        Next v2
    Next v1

    If matchCount <> 6 Then Err.Raise 601, , "Nested For Each iterations mismatch: " & CStr(matchCount)

    Set c1 = Nothing
    Set c2 = Nothing
    PrintLine "Test 6: Nested For Each iteration passed."
End Sub
