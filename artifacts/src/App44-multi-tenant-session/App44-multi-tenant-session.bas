Attribute VB_Name = "Module1"
Option Explicit

' =============================================================================
' ClassicBasic App44: Multi-Tenant Session Broker & State Isolation
' -----------------------------------------------------------------------------
' Phase 3.9 (v0.3.9): Multi-Tenant Session Broker, Virtual Isolation & Process Lifecycles
'
' This test fixture exercises the full lifecycle and state isolation required
' by a multi-tenant session broker:
'   Test 1 -> Form Instantiation & Session Configuration Initialization
'   Test 2 -> Deterministic State Mutation & Session Counter Verification
'   Test 3 -> Isolated File I/O Persistence & Local Storage Verification
'   Test 4 -> Virtual Window Hierarchy & Win32 Controls Resolution
'   Test 5 -> Tenant State Reset & Re-initialization
'   Test 6 -> Complete Form Teardown and Graceful Session Termination
' =============================================================================

Private Type RECT
    Left As Long
    Top As Long
    Right As Long
    Bottom As Long
End Type

Private Const STD_OUTPUT_HANDLE As Long = -11

Private Declare Function GetStdHandle Lib "kernel32" (ByVal nStdHandle As Long) As Long
Private Declare Function WriteFile Lib "kernel32" (ByVal hFile As Long, ByVal lpBuffer As String, ByVal nNumberOfBytesToWrite As Long, lpNumberOfBytesWritten As Long, ByVal lpOverlapped As Long) As Long
Private Declare Function GetClientRect Lib "user32" (ByVal hwnd As Long, lpRect As RECT) As Long

Private Sub PrintLine(ByVal text As String)
    Dim hStdOut As Long
    Dim written As Long
    Dim msg As String
    
    hStdOut = GetStdHandle(STD_OUTPUT_HANDLE)
    msg = text & vbCrLf
    If hStdOut <> 0 And hStdOut <> -1 Then
        WriteFile hStdOut, msg, Len(msg), written, 0
    Else
        Debug.Print text
    End If
End Sub

Public Sub Main()
    PrintLine "=== ClassicBasic App44: Multi-Tenant Session Broker & State Isolation ==="
    
    TestSessionFormInstantiation
    TestSessionStateMutation
    TestSessionIsolatedFileIO
    TestSessionWin32Controls
    TestSessionResetAndReinit
    TestSessionFormTeardown
    
    PrintLine "=== App44 Test Suite Finished Successfully ==="
End Sub

' -----------------------------------------------------------------------------
' TEST 1: Form Instantiation & Session Configuration Initialization
' -----------------------------------------------------------------------------
Private Sub TestSessionFormInstantiation()
    Dim rc As RECT
    
    Load Form1
    Form1.Show
    DoEvents
    
    If Form1.hwnd = 0 Then Err.Raise 101, , "Form1.hWnd is 0"
    If Form1.Caption <> "Session Tenant Workspace" Then Err.Raise 102, , "Form1.Caption mismatch: " & Form1.Caption
    If Form1.txtSessionID.Text <> "TENANT-DEFAULT" Then Err.Raise 103, , "Form1.txtSessionID init mismatch: " & Form1.txtSessionID.Text
    If Form1.txtTenantCounter.Text <> "0" Then Err.Raise 104, , "Form1.txtTenantCounter init mismatch: " & Form1.txtTenantCounter.Text
    
    If GetClientRect(Form1.hwnd, rc) = 0 Then Err.Raise 105, , "GetClientRect(Form1.hWnd) failed"
    If rc.Right <= 0 Or rc.Bottom <= 0 Then Err.Raise 106, , "Form1 client rect dimensions are invalid"
    
    PrintLine "Test 1: Form instantiation and session initialization passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 2: Deterministic State Mutation & Session Counter Verification
' -----------------------------------------------------------------------------
Private Sub TestSessionStateMutation()
    Form1.SetSessionID "TENANT-ALPHA"
    If Form1.GetSessionID() <> "TENANT-ALPHA" Then Err.Raise 201, , "Session ID assignment failed"
    
    Form1.cmdIncrement_Click
    Form1.cmdIncrement_Click
    Form1.cmdIncrement_Click
    
    If Form1.GetCounter() <> 3 Then Err.Raise 202, , "Counter mismatch: expected 3 got " & CStr(Form1.GetCounter())
    If Form1.txtTenantCounter.Text <> "3" Then Err.Raise 203, , "txtTenantCounter text mismatch: " & Form1.txtTenantCounter.Text
    
    Form1.SetDataBuffer "Session ALPHA confidential payload 0xDEADBEEF"
    If InStr(Form1.GetDataBuffer(), "ALPHA") = 0 Then Err.Raise 204, , "Data buffer mismatch"
    
    PrintLine "Test 2: Deterministic state mutation and counter verification passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 3: Isolated File I/O Persistence & Local Storage Verification
' -----------------------------------------------------------------------------
Private Sub TestSessionIsolatedFileIO()
    Dim testFile As String
    Dim readBack As String
    Dim ok As Boolean
    
    testFile = "tenant_alpha_store.txt"
    ok = Form1.WriteTenantFile(testFile, "PAYLOAD_ALPHA_TENANT_SECURE_TOKEN_9988")
    If Not ok Then Err.Raise 301, , "WriteTenantFile returned False"
    
    readBack = Form1.ReadTenantFile(testFile)
    If readBack <> "PAYLOAD_ALPHA_TENANT_SECURE_TOKEN_9988" Then
        Err.Raise 302, , "ReadTenantFile payload mismatch: " & readBack
    End If
    
    ' Cleanup local file
    On Error Resume Next
    Kill testFile
    On Error GoTo 0
    
    PrintLine "Test 3: Isolated file I/O persistence and verification passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 4: Virtual Window Hierarchy & Win32 Controls Resolution
' -----------------------------------------------------------------------------
Private Sub TestSessionWin32Controls()
    If Form1.cmdIncrement.Caption <> "Increment Counter" Then Err.Raise 401, , "cmdIncrement caption mismatch"
    If Form1.cmdWriteDisk.Caption <> "Write Tenant File" Then Err.Raise 402, , "cmdWriteDisk caption mismatch"
    If Form1.cmdReadDisk.Caption <> "Read Tenant File" Then Err.Raise 403, , "cmdReadDisk caption mismatch"
    If Form1.cmdReset.Caption <> "Reset Session" Then Err.Raise 404, , "cmdReset caption mismatch"
    If Form1.lblSession.Caption <> "Session ID:" Then Err.Raise 405, , "lblSession caption mismatch"
    
    PrintLine "Test 4: Virtual window hierarchy and Win32 controls resolution passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 5: Tenant State Reset & Re-initialization
' -----------------------------------------------------------------------------
Private Sub TestSessionResetAndReinit()
    Form1.cmdReset_Click
    
    If Form1.GetCounter() <> 0 Then Err.Raise 501, , "Counter not reset to 0"
    If Form1.txtTenantCounter.Text <> "0" Then Err.Raise 502, , "txtTenantCounter text not reset"
    If InStr(Form1.lblStatus.Caption, "Initialized") = 0 Then Err.Raise 503, , "Status label not reset"
    
    ' Re-initialize with a different tenant ID
    Form1.ResetState "TENANT-BETA"
    If Form1.GetSessionID() <> "TENANT-BETA" Then Err.Raise 504, , "Session ID not updated to TENANT-BETA"
    
    PrintLine "Test 5: Tenant state reset and re-initialization passed."
End Sub

' -----------------------------------------------------------------------------
' TEST 6: Complete Form Teardown and Graceful Session Termination
' -----------------------------------------------------------------------------
Private Sub TestSessionFormTeardown()
    Unload Form1
    PrintLine "Test 6: Form teardown and graceful session termination passed."
End Sub
