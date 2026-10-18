VERSION 1.0 CLASS
BEGIN
  MultiUse = -1  'True
  Persistable = 0  'NotPersistable
  DataBindingBehavior = 0  'vbNone
  DataSourceBehavior  = 0  'vbNone
  MTSTransactionMode  = 0  'NotAnMTSObject
END
Attribute VB_Name = "CReceiver"
Attribute VB_GlobalNameSpace = False
Attribute VB_Creatable = True
Attribute VB_PredeclaredId = False
Attribute VB_Exposed = False
Option Explicit

Private WithEvents m_Emitter As CEmitter
Public ReceivedCount As Long
Public LastCode As Long
Public LastMsg As String

Public Sub Hook(ByVal e As CEmitter)
    Set m_Emitter = e
End Sub

Public Sub Unhook()
    Set m_Emitter = Nothing
End Sub

Private Sub m_Emitter_StatusChanged(ByVal code As Long, ByVal msg As String)
    ReceivedCount = ReceivedCount + 1
    LastCode = code
    LastMsg = msg
End Sub

Private Sub m_Emitter_SimplePing()
    ReceivedCount = ReceivedCount + 1
    LastCode = 999
    LastMsg = "PING"
End Sub
