  VERSION 1.0 CLASS
    BEGIN
      MultiUse = -1  'True
      Persistable = 0  'NotPersistable
      DataBindingBehavior = 0  'vbNone
      DataSourceBehavior  = 0  'vbNone
      MTSTransactionMode  = 0  'NotAnMTSObject
    END
    Attribute VB_Name = "CZombie"
    Attribute VB_GlobalNameSpace = False
    Attribute VB_Creatable = True
    Attribute VB_PredeclaredId = False
    Attribute VB_Exposed = False
    Option Explicit

    Public Value As Long

    Public Sub DoSomething()
        Value = Value + 1
    End Sub

    Private Sub Class_Terminate()
        ' Tenta ressuscitar a si mesmo guardando Me numa variavel global
        ' No VB6, o objeto entra no estado ZOMBIE
        Set Module1.gZombieRef = Me
    End Sub