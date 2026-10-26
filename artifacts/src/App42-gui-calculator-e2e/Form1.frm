VERSION 5.00
Begin VB.Form Form1 
   Caption         =   "ClassicBasic Calculator"
   ClientHeight    =   4200
   ClientLeft      =   60
   ClientTop       =   345
   ClientWidth     =   3600
   LinkTopic       =   "Form1"
   ScaleHeight     =   4200
   ScaleWidth      =   3600
   StartUpPosition =   3  'Windows Default
   Begin VB.TextBox Text1 
      Alignment       =   1  'Right Justify
      Height          =   495
      Left            =   240
      TabIndex        =   0
      Text            =   "0"
      Top             =   240
      Width           =   3135
   End
   Begin VB.Label Label1 
      Caption         =   "Ready"
      Height          =   255
      Left            =   240
      TabIndex        =   1
      Top             =   840
      Width           =   3135
   End
   Begin VB.CommandButton cmd7 
      Caption         =   "7"
      Height          =   495
      Left            =   240
      TabIndex        =   2
      Top             =   1200
      Width           =   615
   End
   Begin VB.CommandButton cmd8 
      Caption         =   "8"
      Height          =   495
      Left            =   1080
      TabIndex        =   3
      Top             =   1200
      Width           =   615
   End
   Begin VB.CommandButton cmd9 
      Caption         =   "9"
      Height          =   495
      Left            =   1920
      TabIndex        =   4
      Top             =   1200
      Width           =   615
   End
   Begin VB.CommandButton cmdDiv 
      Caption         =   "/"
      Height          =   495
      Left            =   2760
      TabIndex        =   5
      Top             =   1200
      Width           =   615
   End
   Begin VB.CommandButton cmd4 
      Caption         =   "4"
      Height          =   495
      Left            =   240
      TabIndex        =   6
      Top             =   1800
      Width           =   615
   End
   Begin VB.CommandButton cmd5 
      Caption         =   "5"
      Height          =   495
      Left            =   1080
      TabIndex        =   7
      Top             =   1800
      Width           =   615
   End
   Begin VB.CommandButton cmd6 
      Caption         =   "6"
      Height          =   495
      Left            =   1920
      TabIndex        =   8
      Top             =   1800
      Width           =   615
   End
   Begin VB.CommandButton cmdMul 
      Caption         =   "*"
      Height          =   495
      Left            =   2760
      TabIndex        =   9
      Top             =   1800
      Width           =   615
   End
   Begin VB.CommandButton cmd1 
      Caption         =   "1"
      Height          =   495
      Left            =   240
      TabIndex        =   10
      Top             =   2400
      Width           =   615
   End
   Begin VB.CommandButton cmd2 
      Caption         =   "2"
      Height          =   495
      Left            =   1080
      TabIndex        =   11
      Top             =   2400
      Width           =   615
   End
   Begin VB.CommandButton cmd3 
      Caption         =   "3"
      Height          =   495
      Left            =   1920
      TabIndex        =   12
      Top             =   2400
      Width           =   615
   End
   Begin VB.CommandButton cmdSub 
      Caption         =   "-"
      Height          =   495
      Left            =   2760
      TabIndex        =   13
      Top             =   2400
      Width           =   615
   End
   Begin VB.CommandButton cmd0 
      Caption         =   "0"
      Height          =   495
      Left            =   240
      TabIndex        =   14
      Top             =   3000
      Width           =   615
   End
   Begin VB.CommandButton cmdDot 
      Caption         =   "."
      Height          =   495
      Left            =   1080
      TabIndex        =   15
      Top             =   3000
      Width           =   615
   End
   Begin VB.CommandButton cmdClear 
      Caption         =   "C"
      Height          =   495
      Left            =   1920
      TabIndex        =   16
      Top             =   3000
      Width           =   615
   End
   Begin VB.CommandButton cmdAdd 
      Caption         =   "+"
      Height          =   495
      Left            =   2760
      TabIndex        =   17
      Top             =   3000
      Width           =   615
   End
   Begin VB.CommandButton cmdEq 
      Caption         =   "="
      Height          =   495
      Left            =   240
      TabIndex        =   18
      Top             =   3600
      Width           =   3135
   End
End
Attribute VB_Name = "Form1"
Attribute VB_GlobalNameSpace = False
Attribute VB_Creatable = False
Attribute VB_PredeclaredId = True
Attribute VB_Exposed = False
Option Explicit

Private m_Op As String
Private m_Operand1 As Double
Private m_NewNumber As Boolean

Private Sub Form_Load()
    ClearAll
End Sub

Public Sub ClearAll()
    m_Op = ""
    m_Operand1 = 0
    m_NewNumber = True
    Text1.Text = "0"
    Label1.Caption = "Ready"
End Sub

Public Sub InputDigit(ByVal d As String)
    If m_NewNumber Or Text1.Text = "0" Then
        Text1.Text = d
        m_NewNumber = False
    Else
        Text1.Text = Text1.Text & d
    End If
End Sub

Public Sub InputDot()
    If m_NewNumber Then
        Text1.Text = "0."
        m_NewNumber = False
    ElseIf InStr(Text1.Text, ".") = 0 Then
        Text1.Text = Text1.Text & "."
    End If
End Sub

Public Sub SetOp(ByVal op As String)
    m_Operand1 = Val(Text1.Text)
    m_Op = op
    m_NewNumber = True
    Label1.Caption = Trim(Str(m_Operand1)) & " " & op
End Sub

Public Sub Compute()
    Dim op2 As Double
    Dim res As Double
    
    If m_Op = "" Then Exit Sub
    
    op2 = Val(Text1.Text)
    Select Case m_Op
        Case "+"
            res = m_Operand1 + op2
        Case "-"
            res = m_Operand1 - op2
        Case "*"
            res = m_Operand1 * op2
        Case "/"
            If op2 = 0 Then
                Text1.Text = "Error"
                Label1.Caption = "Division by zero"
                m_Op = ""
                m_NewNumber = True
                Exit Sub
            Else
                res = m_Operand1 / op2
            End If
    End Select
    
    Label1.Caption = Trim(Str(m_Operand1)) & " " & m_Op & " " & Trim(Str(op2)) & " = " & Trim(Str(res))
    Text1.Text = Trim(Str(res))
    m_Op = ""
    m_NewNumber = True
End Sub

Public Sub cmd0_Click(): InputDigit "0": End Sub
Public Sub cmd1_Click(): InputDigit "1": End Sub
Public Sub cmd2_Click(): InputDigit "2": End Sub
Public Sub cmd3_Click(): InputDigit "3": End Sub
Public Sub cmd4_Click(): InputDigit "4": End Sub
Public Sub cmd5_Click(): InputDigit "5": End Sub
Public Sub cmd6_Click(): InputDigit "6": End Sub
Public Sub cmd7_Click(): InputDigit "7": End Sub
Public Sub cmd8_Click(): InputDigit "8": End Sub
Public Sub cmd9_Click(): InputDigit "9": End Sub
Public Sub cmdDot_Click(): InputDot: End Sub
Public Sub cmdClear_Click(): ClearAll: End Sub
Public Sub cmdAdd_Click(): SetOp "+": End Sub
Public Sub cmdSub_Click(): SetOp "-": End Sub
Public Sub cmdMul_Click(): SetOp "*": End Sub
Public Sub cmdDiv_Click(): SetOp "/": End Sub
Public Sub cmdEq_Click(): Compute: End Sub
