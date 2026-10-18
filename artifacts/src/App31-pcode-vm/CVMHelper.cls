VERSION 1.0 CLASS
BEGIN
  MultiUse = -1  'True
  Persistable = 0  'NotPersistable
  DataBindingBehavior = 0  'vbNone
  DataSourceBehavior  = 0  'vbNone
  MTSTransactionMode  = 0  'NotAnMTSObject
END
Attribute VB_Name = "CVMHelper"
Attribute VB_GlobalNameSpace = False
Attribute VB_Creatable = True
Attribute VB_PredeclaredId = False
Attribute VB_Exposed = False
Option Explicit

Private m_Tag As String
Private m_Counter As Long

Public Property Get Tag() As String
    Tag = m_Tag
End Property

Public Property Let Tag(ByVal s As String)
    m_Tag = s
End Property

Public Function Compute(ByVal op As Long, ByVal a As Long, ByVal b As Long) As Long
    m_Counter = m_Counter + 1
    Select Case op
        Case 1: Compute = a + b
        Case 2: Compute = a - b
        Case 3: Compute = a * b
        Case 4:
            If b <> 0 Then
                Compute = a \ b
            Else
                Compute = -1
            End If
        Case Else
            Compute = 0
    End Select
End Function

Public Function GetCounter() As Long
    GetCounter = m_Counter
End Function
