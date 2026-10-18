VERSION 1.0 CLASS
BEGIN
  MultiUse = -1  'True
  Persistable = 0  'NotPersistable
  DataBindingBehavior = 0  'vbNone
  DataSourceBehavior  = 0  'vbNone
  MTSTransactionMode  = 0  'NotAnMTSObject
END
Attribute VB_Name = "CEmitter"
Attribute VB_GlobalNameSpace = False
Attribute VB_Creatable = True
Attribute VB_PredeclaredId = False
Attribute VB_Exposed = False
Option Explicit

Public Event StatusChanged(ByVal code As Long, ByVal msg As String)
Public Event SimplePing()

Public Sub Trigger(ByVal code As Long, ByVal msg As String)
    RaiseEvent StatusChanged(code, msg)
End Sub

Public Sub Ping()
    RaiseEvent SimplePing
End Sub
