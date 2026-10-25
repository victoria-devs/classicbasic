VERSION 5.00
Begin VB.Form Form1 
   Caption         =   "App40 Stream Form"
   ClientHeight    =   3600
   ClientLeft      =   60
   ClientTop       =   345
   ClientWidth     =   4800
   LinkTopic       =   "Form1"
   ScaleHeight     =   3600
   ScaleWidth      =   4800
   StartUpPosition =   3  'Windows Default
   Begin VB.Label Label1 
      Caption         =   "Status: Idle"
      Height          =   315
      Left            =   240
      TabIndex        =   2
      Top             =   1800
      Width           =   4215
   End
   Begin VB.CommandButton Command1 
      Caption         =   "Submit"
      Height          =   495
      Left            =   240
      TabIndex        =   1
      Top             =   1080
      Width           =   1815
   End
   Begin VB.TextBox Text1 
      Height          =   495
      Left            =   240
      TabIndex        =   0
      Text            =   "Initial Stream Text"
      Top             =   360
      Width           =   4215
   End
End
Attribute VB_Name = "Form1"
Attribute VB_GlobalNameSpace = False
Attribute VB_Creatable = False
Attribute VB_PredeclaredId = True
Attribute VB_Exposed = False
Option Explicit

Private Sub Command1_Click()
    Label1.Caption = "Status: Submitted " & Text1.Text
End Sub
