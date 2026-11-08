VERSION 5.00
Begin VB.Form Form1 
   Caption         =   "Session Tenant Workspace"
   ClientHeight    =   4800
   ClientLeft      =   60
   ClientTop       =   345
   ClientWidth     =   5400
   LinkTopic       =   "Form1"
   ScaleHeight     =   4800
   ScaleWidth      =   5400
   StartUpPosition =   3  'Windows Default
   Begin VB.TextBox txtSessionID 
      Height          =   375
      Left            =   1560
      Locked          =   -1  'True
      TabIndex        =   0
      Text            =   "SESSION-INIT"
      Top             =   240
      Width           =   3615
   End
   Begin VB.TextBox txtTenantCounter 
      Alignment       =   1  'Right Justify
      Height          =   375
      Left            =   1560
      TabIndex        =   1
      Text            =   "0"
      Top             =   720
      Width           =   3615
   End
   Begin VB.TextBox txtDataBuffer 
      Height          =   855
      Left            =   240
      MultiLine       =   -1  'True
      ScrollBars      =   2  'Vertical
      TabIndex        =   2
      Text            =   "Tenant Isolated Memory Buffer"
      Top             =   1560
      Width           =   4935
   End
   Begin VB.CommandButton cmdIncrement 
      Caption         =   "Increment Counter"
      Height          =   495
      Left            =   240
      TabIndex        =   3
      Top             =   2640
      Width           =   2415
   End
   Begin VB.CommandButton cmdWriteDisk 
      Caption         =   "Write Tenant File"
      Height          =   495
      Left            =   2760
      TabIndex        =   4
      Top             =   2640
      Width           =   2415
   End
   Begin VB.CommandButton cmdReadDisk 
      Caption         =   "Read Tenant File"
      Height          =   495
      Left            =   240
      TabIndex        =   5
      Top             =   3240
      Width           =   2415
   End
   Begin VB.CommandButton cmdReset 
      Caption         =   "Reset Session"
      Height          =   495
      Left            =   2760
      TabIndex        =   6
      Top             =   3240
      Width           =   2415
   End
   Begin VB.Label lblStatus 
      BorderStyle     =   1  'Fixed Single
      Caption         =   "Status: Ready"
      Height          =   375
      Left            =   240
      TabIndex        =   7
      Top             =   3960
      Width           =   4935
   End
   Begin VB.Label lblSession 
      Caption         =   "Session ID:"
      Height          =   255
      Left            =   240
      TabIndex        =   8
      Top             =   300
      Width           =   1215
   End
   Begin VB.Label lblCounter 
      Caption         =   "Local Counter:"
      Height          =   255
      Left            =   240
      TabIndex        =   9
      Top             =   780
      Width           =   1215
   End
   Begin VB.Label lblBuffer 
      Caption         =   "Tenant Session Buffer:"
      Height          =   255
      Left            =   240
      TabIndex        =   10
      Top             =   1320
      Width           =   2055
   End
End
Attribute VB_Name = "Form1"
Attribute VB_GlobalNameSpace = False
Attribute VB_Creatable = False
Attribute VB_PredeclaredId = True
Attribute VB_Exposed = False
Option Explicit

Private m_Counter As Long
Private m_SessionID As String

Private Sub Form_Load()
    ResetState "TENANT-DEFAULT"
End Sub

Public Sub ResetState(ByVal sId As String)
    m_SessionID = sId
    m_Counter = 0
    txtSessionID.Text = m_SessionID
    txtTenantCounter.Text = CStr(m_Counter)
    txtDataBuffer.Text = "Tenant Buffer: " & m_SessionID
    lblStatus.Caption = "Status: Initialized for " & m_SessionID
End Sub

Public Sub IncrementCounter()
    m_Counter = m_Counter + 1
    txtTenantCounter.Text = CStr(m_Counter)
    lblStatus.Caption = "Status: Counter incremented to " & CStr(m_Counter)
End Sub

Public Function GetCounter() As Long
    GetCounter = m_Counter
End Function

Public Function GetSessionID() As String
    GetSessionID = m_SessionID
End Function

Public Sub SetSessionID(ByVal newId As String)
    m_SessionID = newId
    txtSessionID.Text = m_SessionID
End Sub

Public Sub SetDataBuffer(ByVal txt As String)
    txtDataBuffer.Text = txt
End Sub

Public Function GetDataBuffer() As String
    GetDataBuffer = txtDataBuffer.Text
End Function

Public Function WriteTenantFile(ByVal filename As String, ByVal content As String) As Boolean
    On Error GoTo ErrHandler
    Dim fn As Integer
    fn = FreeFile
    Open filename For Output As #fn
    Print #fn, content;
    Close #fn
    lblStatus.Caption = "Status: File written: " & filename
    WriteTenantFile = True
    Exit Function
ErrHandler:
    lblStatus.Caption = "Status: Write error: " & Err.Description
    WriteTenantFile = False
End Function

Public Function ReadTenantFile(ByVal filename As String) As String
    On Error GoTo ErrHandler
    Dim fn As Integer
    Dim res As String
    Dim lineBuf As String
    fn = FreeFile
    Open filename For Input As #fn
    res = ""
    Do While Not EOF(fn)
        Line Input #fn, lineBuf
        If Len(res) > 0 Then res = res & vbCrLf
        res = res & lineBuf
    Loop
    Close #fn
    lblStatus.Caption = "Status: File read: " & filename
    ReadTenantFile = res
    Exit Function
ErrHandler:
    lblStatus.Caption = "Status: Read error: " & Err.Description
    ReadTenantFile = ""
End Function

Public Sub cmdIncrement_Click()
    IncrementCounter
End Sub

Public Sub cmdWriteDisk_Click()
    Dim fname As String
    fname = "tenant_" & m_SessionID & ".dat"
    WriteTenantFile fname, txtDataBuffer.Text
End Sub

Public Sub cmdReadDisk_Click()
    Dim fname As String
    fname = "tenant_" & m_SessionID & ".dat"
    txtDataBuffer.Text = ReadTenantFile(fname)
End Sub

Public Sub cmdReset_Click()
    ResetState m_SessionID
End Sub
