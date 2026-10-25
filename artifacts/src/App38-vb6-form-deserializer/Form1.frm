VERSION 5.00
Begin VB.Form Form1 
   Caption         =   "App38 Form Deserializer"
   ClientHeight    =   6750
   ClientLeft      =   60
   ClientTop       =   345
   ClientWidth     =   9000
   LinkTopic       =   "Form1"
   ScaleHeight     =   6750
   ScaleWidth      =   9000
   StartUpPosition =   3  'Windows Default
   Begin VB.PictureBox Picture1 
      Height          =   1500
      Left            =   3300
      ScaleHeight     =   1440
      ScaleWidth      =   2340
      TabIndex        =   9
      Top             =   4200
      Width           =   2400
   End
   Begin VB.Timer Timer1 
      Enabled         =   0   'False
      Interval        =   1000
      Left            =   3300
      Top             =   3600
   End
   Begin VB.ComboBox Combo1 
      Height          =   315
      Left            =   3300
      TabIndex        =   8
      Text            =   "Combo Item"
      Top             =   3000
      Width           =   2400
   End
   Begin VB.ListBox List1 
      Height          =   1230
      Left            =   360
      TabIndex        =   7
      Top             =   3600
      Width           =   2400
   End
   Begin VB.Frame Frame1 
      Caption         =   "Container Frame"
      Height          =   2400
      Left            =   3300
      TabIndex        =   6
      Top             =   360
      Width           =   3600
   End
   Begin VB.OptionButton Option1 
      Caption         =   "Option Choice"
      Height          =   360
      Left            =   360
      TabIndex        =   5
      Top             =   3000
      Value           =   -1  'True
      Width           =   1950
   End
   Begin VB.CheckBox Check1 
      Caption         =   "Check Feature"
      Height          =   360
      Left            =   360
      TabIndex        =   4
      Top             =   2400
      Value           =   1  'Checked
      Width           =   2100
   End
   Begin VB.Label Label1 
      Caption         =   "Form Label"
      Height          =   360
      Left            =   360
      TabIndex        =   3
      Top             =   1800
      Width           =   2400
   End
   Begin VB.TextBox Text1 
      Height          =   420
      Left            =   360
      TabIndex        =   2
      Text            =   "Deserialized Text"
      Top             =   1080
      Width           =   2700
   End
   Begin VB.CommandButton Command1 
      Caption         =   "Test Button"
      Height          =   480
      Left            =   360
      TabIndex        =   0
      Top             =   360
      Width           =   1800
   End
End
Attribute VB_Name = "Form1"
Attribute VB_GlobalNameSpace = False
Attribute VB_Creatable = False
Attribute VB_PredeclaredId = True
Attribute VB_Exposed = False
Option Explicit
