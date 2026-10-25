VERSION 5.00
Begin VB.Form Form1 
   Caption         =   "App41 WebAssembly Canvas Client"
   ClientHeight    =   4800
   ClientLeft      =   60
   ClientTop       =   345
   ClientWidth     =   6400
   LinkTopic       =   "Form1"
   ScaleHeight     =   4800
   ScaleWidth      =   6400
   StartUpPosition =   3  'Windows Default
   Begin VB.Label Label1 
      Caption         =   "Canvas Status: Disconnected"
      Height          =   375
      Left            =   360
      TabIndex        =   2
      Top             =   2400
      Width           =   4815
   End
   Begin VB.CommandButton Command1 
      Caption         =   "Transmit"
      Height          =   495
      Left            =   360
      TabIndex        =   1
      Top             =   1440
      Width           =   1815
   End
   Begin VB.TextBox Text1 
      Height          =   495
      Left            =   360
      TabIndex        =   0
      Text            =   "WASM Client Stream Buffer"
      Top             =   480
      Width           =   4815
   End
End
Attribute VB_Name = "Form1"
Attribute VB_GlobalNameSpace = False
Attribute VB_Creatable = False
Attribute VB_PredeclaredId = True
Attribute VB_Exposed = False
Option Explicit

Private Sub Command1_Click()
    Label1.Caption = "Canvas Status: Streamed " & Text1.Text
End Sub
