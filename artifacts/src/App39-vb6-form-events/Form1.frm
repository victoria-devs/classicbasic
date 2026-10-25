VERSION 5.00
Begin VB.Form Form1 
   Caption         =   "App39 Form Event Sinks"
   ClientHeight    =   6750
   ClientLeft      =   60
   ClientTop       =   345
   ClientWidth     =   9000
   LinkTopic       =   "Form1"
   ScaleHeight     =   6750
   ScaleWidth      =   9000
   StartUpPosition =   3  'Windows Default
   Begin VB.ListBox List1 
      Height          =   1620
      Left            =   3360
      TabIndex        =   5
      Top             =   360
      Width           =   3135
   End
   Begin VB.Timer Timer1 
      Enabled         =   0   'False
      Interval        =   1000
      Left            =   3360
      Top             =   2400
   End
   Begin VB.OptionButton Option1 
      Caption         =   "Option Choice"
      Height          =   375
      Left            =   360
      TabIndex        =   4
      Top             =   2400
      Value           =   0   'False
      Width           =   2175
   End
   Begin VB.CheckBox Check1 
      Caption         =   "Check Feature"
      Height          =   375
      Left            =   360
      TabIndex        =   3
      Top             =   1800
      Value           =   0   'Unchecked
      Width           =   2175
   End
   Begin VB.TextBox Text1 
      Height          =   495
      Left            =   360
      TabIndex        =   2
      Text            =   "Initial Value"
      Top             =   1080
      Width           =   2535
   End
   Begin VB.CommandButton Command1 
      Caption         =   "Trigger Event"
      Height          =   495
      Left            =   360
      TabIndex        =   0
      Top             =   360
      Width           =   1815
   End
End
Attribute VB_Name = "Form1"
Attribute VB_GlobalNameSpace = False
Attribute VB_Creatable = False
Attribute VB_PredeclaredId = True
Attribute VB_Exposed = False
Option Explicit

Public EventLog As String
Public EventCount As Long

Public Sub LogEvent(ByVal eventName As String)
    EventLog = EventLog & eventName & ";"
    EventCount = EventCount + 1
End Sub

Private Sub Form_Load()
    LogEvent "Form_Load"
End Sub

Private Sub Command1_Click()
    LogEvent "Command1_Click"
End Sub

Private Sub Text1_Change()
    LogEvent "Text1_Change"
End Sub

Private Sub Check1_Click()
    LogEvent "Check1_Click"
End Sub

Private Sub Option1_Click()
    LogEvent "Option1_Click"
End Sub

Private Sub Timer1_Timer()
    LogEvent "Timer1_Timer"
End Sub

Private Sub Form_Unload(Cancel As Integer)
    LogEvent "Form_Unload"
End Sub

Public Sub FireCommandClick()
    Command1_Click
End Sub

Public Sub FireTextChange()
    Text1_Change
End Sub

Public Sub FireCheckClick()
    Check1_Click
End Sub

Public Sub FireOptionClick()
    Option1_Click
End Sub

Public Sub FireTimerTick()
    Timer1_Timer
End Sub
