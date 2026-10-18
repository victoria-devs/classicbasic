VERSION 1.0 CLASS
BEGIN
  MultiUse = -1  'True
  Persistable = 0  'NotPersistable
  DataBindingBehavior = 0  'vbNone
  DataSourceBehavior  = 0  'vbNone
  MTSTransactionMode  = 0  'NotAnMTSObject
END
Attribute VB_Name = "CRecordHolder"
Attribute VB_GlobalNameSpace = False
Attribute VB_Creatable = True
Attribute VB_PredeclaredId = False
Attribute VB_Exposed = False
Option Explicit

Private m_HeldRecord As Variant

Public Property Let HeldRecord(ByVal v As Variant)
    m_HeldRecord = v
End Property

Public Property Get HeldRecord() As Variant
    HeldRecord = m_HeldRecord
End Property

Public Function IsHoldingRecord() As Boolean
    IsHoldingRecord = Not IsEmpty(m_HeldRecord)
End Function

Public Sub Clear()
    m_HeldRecord = Empty
End Sub
