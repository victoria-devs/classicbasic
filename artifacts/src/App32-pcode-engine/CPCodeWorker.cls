VERSION 1.0 CLASS
BEGIN
  MultiUse = -1  'True
  Persistable = 0  'NotPersistable
  DataBindingBehavior = 0  'vbNone
  DataSourceBehavior = 0  'vbNone
  MTSTransactionMode = 0  'NotAnMTSObject
END
Attribute VB_Name = "CPCodeWorker"
Attribute VB_GlobalNameSpace = False
Attribute VB_Creatable = True
Attribute VB_PredeclaredId = False
Attribute VB_Exposed = False
Option Explicit

Private m_BaseValue As Long

Public Property Get BaseValue() As Long
    BaseValue = m_BaseValue
End Property

Public Property Let BaseValue(ByVal v As Long)
    m_BaseValue = v
End Property

Public Function ComputeOffset(ByVal factor As Long, ByVal delta As Long) As Long
    ComputeOffset = (m_BaseValue * factor) + delta
End Function

Public Function FormatMessage(ByVal prefix As String) As String
    FormatMessage = prefix & ": " & CStr(m_BaseValue)
End Function
