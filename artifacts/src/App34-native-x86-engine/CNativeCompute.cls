VERSION 1.0 CLASS
BEGIN
  MultiUse = -1  'True
  Persistable = 0  'NotPersistable
  DataBindingBehavior = 0  'vbNone
  DataSourceBehavior  = 0  'vbNone
  MTSTransactionMode  = 0  'NotAnMTSObject
END
Attribute VB_Name = "CNativeCompute"
Attribute VB_GlobalNameSpace = False
Attribute VB_Creatable = True
Attribute VB_PredeclaredId = False
Attribute VB_Exposed = False
Option Explicit

Private m_BaseValue As Long

Public Property Get BaseValue() As Long
    BaseValue = m_BaseValue
End Property

Public Property Let BaseValue(ByVal val As Long)
    m_BaseValue = val
End Property

Public Function ComputeLinear(ByVal factor As Long, ByVal delta As Long) As Long
    ComputeLinear = (m_BaseValue * factor) + delta
End Function

Public Function EvaluateFpu(ByVal x As Double, ByVal y As Double) As Double
    EvaluateFpu = (x * y) + Sqr(Abs(x - y))
End Function

Public Function FormatStatus(ByVal tag As String) As String
    FormatStatus = tag & ": " & CStr(m_BaseValue)
End Function
