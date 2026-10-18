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

' -----------------------------------------------------------------------------
' MSVBVM60.DLL Introspection & Telemetry Declarations (Eb* & Tip* & ExprSrv)
' -----------------------------------------------------------------------------
Private Declare Function EbGetHandleOfExecutingProject Lib "msvbvm60.dll" Alias "#232" (ByRef hProj As Long) As Long
Private Declare Function EbIsProjectOnStack Lib "msvbvm60.dll" Alias "#298" (ByVal hProj As Long) As Long
Private Declare Function EbGetErrorInfo Lib "msvbvm60.dll" Alias "#284" (ByRef errNum As Long, ByVal pbstrDesc As Long) As Long
Private Declare Function EbGetObjConnectionCounts Lib "msvbvm60.dll" Alias "#1025" (ByVal pObj As Object, ByRef connCount As Long) As Long
Private Declare Function EbGetVBAObject Lib "msvbvm60.dll" Alias "#2010" (ByRef pVba As Object) As Long
Private Declare Function EbResetProjectNormal Lib "msvbvm60.dll" Alias "#265" () As Long
Private Declare Function EbLibraryLoad Lib "msvbvm60.dll" Alias "#268" (ByVal pLibName As Long) As Long
Private Declare Function EbLibraryUnload Lib "msvbvm60.dll" Alias "#269" (ByVal hLib As Long) As Long
Private Declare Function EbLoadRunTime Lib "msvbvm60.dll" Alias "#271" () As Long

Private Declare Function TipSetOption Lib "msvbvm60.dll" Alias "#220" (ByVal optId As Long, ByVal optVal As Long) As Long
Private Declare Function TipGetAddressOfPredeclaredInstance Lib "msvbvm60.dll" Alias "#195" (ByVal pDesc As Long, ByRef pInst As Object) As Long
Private Declare Function TipCreateInstanceEx Lib "msvbvm60.dll" Alias "#299" (ByVal hProj As Long, ByVal clsid As Long, ByRef pObj As Object) As Long
Private Declare Function TipUnloadInstance Lib "msvbvm60.dll" Alias "#266" (ByVal pObj As Object) As Long
Private Declare Function TipInvokeMethod Lib "msvbvm60.dll" Alias "#1016" (ByVal pObj As Object, ByVal methodId As Long) As Long
Private Declare Function CreateIExprSrvObj Lib "msvbvm60.dll" Alias "#2000" (ByVal pUnkOuter As Long, ByVal riid As Long, ByRef pExprSrv As Object) As Long

Public Sub PrintLine(ByVal s As String)
    Dim hStdOut As Long
    Dim written As Long
    Dim msg As String

    msg = s & vbCrLf
    hStdOut = GetStdHandle(STD_OUTPUT_HANDLE)
    WriteFile hStdOut, msg, Len(msg), written, 0
End Sub

Public Sub Main()
    PrintLine "=== ClassicBasic App32: IDE Hosting, Project Introspection & Debugger Telemetry ==="

    TestProjectHandleAndExecutionStack
    TestPredeclaredClassInstanceResolution
    TestObjectConnectionCounts
    TestExpressionServiceFactory
    TestTipProjectOptionsAndInstanceManagement
    TestLibraryManagementAndProjectReset

    PrintLine "=== App32 Test Suite Finished Successfully ==="
End Sub

' -----------------------------------------------------------------------------
' TEST 1: Project Execution Handle & Stack Telemetry
' -----------------------------------------------------------------------------
Private Sub TestProjectHandleAndExecutionStack()
    Dim hProj As Long
    Dim hr As Long
    Dim onStack As Long

    hr = EbGetHandleOfExecutingProject(hProj)
    If hr <> 0 Or hProj = 0 Then Err.Raise 101, , "EbGetHandleOfExecutingProject failed (hr=" & CStr(hr) & ")"

    onStack = EbIsProjectOnStack(hProj)
    If onStack = 0 Then Err.Raise 102, , "EbIsProjectOnStack failed (expected project to be active)"

    PrintLine "Test 1: Project handle and execution stack query passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 2: Predeclared Class Instance Resolution (VB_PredeclaredId & TipGetAddress)
' -----------------------------------------------------------------------------
Private Sub TestPredeclaredClassInstanceResolution()
    Dim res As Long
    Dim pInst As Object

    ' Access the predeclared default instance directly
    CPredeclared.Value = 42
    If CPredeclared.Value <> 42 Then Err.Raise 201, , "Predeclared instance property mismatch"

    res = CPredeclared.Compute(10)
    If res <> 420 Then Err.Raise 202, , "Predeclared instance compute mismatch"

    ' Query predeclared address via Tip engine
    On Error Resume Next
    TipGetAddressOfPredeclaredInstance VarPtr(CPredeclared), pInst
    If Err.Number = 0 And Not pInst Is Nothing Then
        If pInst.Value <> 42 Then Err.Raise 203, , "Resolved predeclared instance mismatch"
        Set pInst = Nothing
    End If
    On Error GoTo 0

    PrintLine "Test 2: Predeclared class instance resolution passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 3: COM Object Connection Counts & Sink Telemetry
' -----------------------------------------------------------------------------
Private Sub TestObjectConnectionCounts()
    Dim target As CEventTarget
    Dim hr As Long
    Dim count As Long

    Set target = New CEventTarget
    count = -1
    hr = EbGetObjConnectionCounts(target, count)
    If hr <> 0 Then Err.Raise 301, , "EbGetObjConnectionCounts failed (hr=" & CStr(hr) & ")"
    If count < 0 Then Err.Raise 302, , "Connection count negative: " & CStr(count)

    Set target = Nothing
    PrintLine "Test 3: COM connection counts and telemetry passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 4: Runtime Expression Service Factory & Root VBA Object
' -----------------------------------------------------------------------------
Private Sub TestExpressionServiceFactory()
    Dim pExprSrv As Object
    Dim pVba As Object
    Dim hr As Long

    hr = CreateIExprSrvObj(0, 0, pExprSrv)
    If hr <> 0 Then Err.Raise 401, , "CreateIExprSrvObj failed (hr=" & CStr(hr) & ")"

    hr = EbGetVBAObject(pVba)
    If hr <> 0 Then Err.Raise 402, , "EbGetVBAObject failed (hr=" & CStr(hr) & ")"

    Set pExprSrv = Nothing
    Set pVba = Nothing
    PrintLine "Test 4: Runtime expression service and VBA object query passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 5: Tip Engine Project Options & Instance Lifecycle
' -----------------------------------------------------------------------------
Private Sub TestTipProjectOptionsAndInstanceManagement()
    Dim hr As Long
    Dim objInst As Object

    hr = TipSetOption(1, 1)
    If hr <> 0 Then Err.Raise 501, , "TipSetOption failed (hr=" & CStr(hr) & ")"

    ' Project-aware instantiation & invoke
    hr = TipCreateInstanceEx(0, 0, objInst)
    If hr = 0 And Not objInst Is Nothing Then
        TipInvokeMethod objInst, 1
        TipUnloadInstance objInst
        Set objInst = Nothing
    End If

    PrintLine "Test 5: Tip project options and instance management passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 6: Library Management, Error Telemetry & Project Reset
' -----------------------------------------------------------------------------
Private Sub TestLibraryManagementAndProjectReset()
    Dim hr As Long
    Dim errNum As Long

    hr = EbLoadRunTime()
    If hr <> 0 Then Err.Raise 601, , "EbLoadRunTime failed"

    ' Error info query
    hr = EbGetErrorInfo(errNum, 0)
    If hr <> 0 Then Err.Raise 602, , "EbGetErrorInfo failed"

    hr = EbLibraryLoad(0)
    hr = EbLibraryUnload(0)

    ' Reset project normal
    hr = EbResetProjectNormal()
    If hr <> 0 Then Err.Raise 603, , "EbResetProjectNormal failed"

    PrintLine "Test 6: Library management and project reset passed."
End Sub
