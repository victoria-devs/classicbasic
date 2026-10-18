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

Private m_HeldRecord As InnerData
Private m_HasRecord As Boolean

Friend Property Let HeldRecord(ByRef rec As InnerData)
    m_HeldRecord = rec
    m_HasRecord = True
End Property

Friend Property Get HeldRecord() As InnerData
    HeldRecord = m_HeldRecord
End Property

Public Function IsHoldingRecord() As Boolean
    IsHoldingRecord = m_HasRecord
End Function

Public Sub Clear()
    Dim blank As InnerData
    m_HeldRecord = blank
    m_HasRecord = False
End Sub
