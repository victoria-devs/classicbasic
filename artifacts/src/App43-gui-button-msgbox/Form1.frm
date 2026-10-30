VERSION 5.00
Begin VB.Form MainForm 
   Caption         =   "XCalculator"
   ClientHeight    =   4305
   ClientLeft      =   60
   ClientTop       =   450
   ClientWidth     =   3465
   LinkTopic       =   "Form1"
   MaxButton       =   0   'False
   MinButton       =   0   'False
   ScaleHeight     =   4305
   ScaleWidth      =   3465
   StartUpPosition =   2  'CenterScreen
   Begin VB.CommandButton Command_divise 
      Caption         =   "/"
      Height          =   615
      Left            =   2640
      TabIndex        =   15
      Top             =   3600
      Width           =   735
   End
   Begin VB.CommandButton Command_multiply 
      Caption         =   "*"
      Height          =   615
      Left            =   2640
      TabIndex        =   14
      Top             =   2880
      Width           =   735
   End
   Begin VB.CommandButton Command_minus 
      Caption         =   "-"
      Height          =   615
      Left            =   2640
      TabIndex        =   13
      Top             =   2160
      Width           =   735
   End
   Begin VB.CommandButton Command_plus 
      Caption         =   "+"
      Height          =   615
      Left            =   2640
      TabIndex        =   12
      Top             =   1440
      Width           =   735
   End
   Begin VB.CommandButton Command_zero 
      Caption         =   "0"
      Height          =   615
      Left            =   120
      TabIndex        =   11
      Top             =   3600
      Width           =   1575
   End
   Begin VB.CommandButton Command_three 
      Caption         =   "3"
      Height          =   615
      Left            =   1800
      TabIndex        =   10
      Top             =   2880
      Width           =   735
   End
   Begin VB.CommandButton Command_two 
      Caption         =   "2"
      Height          =   615
      Left            =   960
      TabIndex        =   9
      Top             =   2880
      Width           =   735
   End
   Begin VB.CommandButton Command_one 
      Caption         =   "1"
      Height          =   615
      Left            =   120
      TabIndex        =   8
      Top             =   2880
      Width           =   735
   End
   Begin VB.CommandButton Command_six 
      Caption         =   "6"
      Height          =   615
      Left            =   1800
      TabIndex        =   7
      Top             =   2160
      Width           =   735
   End
   Begin VB.CommandButton Command_five 
      Caption         =   "5"
      Height          =   615
      Left            =   960
      TabIndex        =   6
      Top             =   2160
      Width           =   735
   End
   Begin VB.CommandButton Command_four 
      Caption         =   "4"
      Height          =   615
      Left            =   120
      TabIndex        =   5
      Top             =   2160
      Width           =   735
   End
   Begin VB.CommandButton Command_nine 
      Caption         =   "9"
      Height          =   615
      Left            =   1800
      TabIndex        =   4
      Top             =   1440
      Width           =   735
   End
   Begin VB.CommandButton Command_eight 
      Caption         =   "8"
      Height          =   615
      Left            =   960
      TabIndex        =   3
      Top             =   1440
      Width           =   735
   End
   Begin VB.CommandButton Command_seven 
      Caption         =   "7"
      Height          =   615
      Left            =   120
      TabIndex        =   2
      Top             =   1440
      Width           =   735
   End
   Begin VB.TextBox Text_calc_display 
      Alignment       =   1  'Right Justify
      BackColor       =   &H00FFC0FF&
      BeginProperty Font 
         Name            =   "Lucida Console"
         Size            =   24
         Charset         =   0
         Weight          =   700
         Underline       =   0   'False
         Italic          =   0   'False
         Strikethrough   =   0   'False
      EndProperty
      ForeColor       =   &H00FFFFFF&
      Height          =   585
      Left            =   120
      TabIndex        =   1
      Text            =   "0"
      Top             =   120
      Width           =   3255
   End
   Begin VB.CommandButton Command_equals 
      Caption         =   "="
      Height          =   615
      Left            =   1800
      TabIndex        =   0
      Top             =   3600
      Width           =   735
   End
End
Attribute VB_Name = "MainForm"
Attribute VB_GlobalNameSpace = False
Attribute VB_Creatable = False
Attribute VB_PredeclaredId = True
Attribute VB_Exposed = False
Public Enum CALC_SYMBOLS
    ZERO = 0
    ONE = 1
    TWO = 2
    THREE = 3
    FOUR = 4
    FIVE = 5
    SIX = 6
    SEVEN = 7
    EIGHT = 8
    NINE = 9
End Enum

Private Sub Command_eight_Click()
    WriteSymbol EIGHT
End Sub

Private Sub Command_five_Click()
    WriteSymbol FIVE
End Sub

Private Sub Command_four_Click()
    WriteSymbol FOUR
End Sub

Private Sub Command_nine_Click()
    WriteSymbol NINE
End Sub

Private Sub Command_plus_Click()
    Text_calc_display.Text = Text_calc_display.Text & " + "
End Sub

Private Sub Command_minus_Click()
    Text_calc_display.Text = Text_calc_display.Text & " - "
End Sub

Private Sub Command_multiply_Click()
    Text_calc_display.Text = Text_calc_display.Text & " * "
End Sub

Private Sub Command_divide_Click()
    Text_calc_display.Text = Text_calc_display.Text & " / "
End Sub

Private Sub Command_one_Click()
    WriteSymbol ONE
End Sub

Private Sub Command_seven_Click()
    WriteSymbol SEVEN
End Sub

Private Sub Command_six_Click()
    WriteSymbol SIX
End Sub

Private Sub Command_three_Click()
    WriteSymbol THREE
End Sub

Private Sub Command_two_Click()
    WriteSymbol TWO
End Sub

Private Sub Command_zero_Click()
    WriteSymbol ZERO
End Sub


Private Sub Command_equals_Click()
    On Error GoTo ErrorHandler
    
    Dim sc As Object
    Set sc = CreateObject("MSScriptControl.ScriptControl")
    sc.Language = "VBScript"
    
    Dim resultado As Variant
    resultado = sc.Eval(Text_calc_display.Text)
    
    Text_calc_display.Text = CStr(resultado)
    Exit Sub

ErrorHandler:
    Text_calc_display.Text = "Erro"
End Sub

Private Sub WriteSymbol(ByVal C_SYMBOL As CALC_SYMBOLS)
    ' Use a variável recebida (C_SYMBOL) e concatene com &
    Text_calc_display.Text = Text_calc_display.Text & CStr(C_SYMBOL)
End Sub

