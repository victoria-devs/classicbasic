    Attribute VB_Name = "Module1"
    Option Explicit

    Private Declare Function GetStdHandle Lib "kernel32" (ByVal nStdHandle As Long) As Long
    Private Declare Function WriteFile Lib "kernel32" ( _
        ByVal hFile As Long, _
        ByVal lpBuffer As String, _
        ByVal nNumberOfBytesToWrite As Long, _
        lpNumberOfBytesWritten As Long, _
        ByVal lpOverlapped As Long) As Long

    Private Const STD_OUTPUT_HANDLE As Long = -11

    Private Sub PrintLine(ByVal s As String)
        Dim hStdOut As Long
        Dim written As Long
        Dim msg As String

        msg = s & vbCrLf
        hStdOut = GetStdHandle(STD_OUTPUT_HANDLE)
        WriteFile hStdOut, msg, Len(msg), written, 0
    End Sub

    Public Sub Main()
        PrintLine "=== ClassicBasic App22: Extended Standard Library (rtc*) ==="

        TestFinancialFunctions
        TestCalendarAndDateMath
        TestFormatters
        TestSwitchAndPartition
        TestRegistrySettings

        PrintLine "=== App22 Test Suite Finished Successfully ==="
    End Sub

    ' -----------------------------------------------------------------------------
    ' TEST 1: Financial Suite (SLN, SYD, DDB, PMT, IPMT, PPMT, PV, FV, NPer, Rate, NPV, IRR, MIRR)
    ' -----------------------------------------------------------------------------
    Private Sub TestFinancialFunctions()
        Dim slnVal As Double
        Dim sydVal As Double
        Dim ddbVal As Double
        Dim pmtVal As Double
        Dim ipmtVal As Double
        Dim ppmtVal As Double
        Dim pvVal As Double
        Dim fvVal As Double
        Dim nperVal As Double
        Dim rateVal As Double
        Dim npvVal As Double
        Dim irrVal As Double
        Dim mirrVal As Double
        Dim cashFlows(0 To 4) As Double

        ' 1. Depreciation: Asset 10,000, salvage 1,000, life 5 years
        slnVal = SLN(10000, 1000, 5)
        If Abs(slnVal - 1800#) > 0.001 Then
            PrintLine "FAIL: SLN expected 1800 got " & CStr(slnVal)
            Exit Sub
        End If

        sydVal = SYD(10000, 1000, 5, 1)
        If Abs(sydVal - 3000#) > 0.001 Then
            PrintLine "FAIL: SYD expected 3000 got " & CStr(sydVal)
            Exit Sub
        End If

        ddbVal = DDB(10000, 1000, 5, 1)
        If Abs(ddbVal - 4000#) > 0.001 Then
            PrintLine "FAIL: DDB expected 4000 got " & CStr(ddbVal)
            Exit Sub
        End If

        ' 2. Annuities: Loan 10,000, 8% annual (0.08 / 12), 10 years (120 months)
        pmtVal = PMT(0.08 / 12, 120, -10000)
        If Abs(pmtVal - 121.3276) > 0.01 Then
            PrintLine "FAIL: PMT expected ~121.33 got " & CStr(pmtVal)
            Exit Sub
        End If

        ' Identity: IPMT(period 1) + PPMT(period 1) = PMT
        ipmtVal = IPMT(0.08 / 12, 1, 120, -10000)
        ppmtVal = PPMT(0.08 / 12, 1, 120, -10000)
        If Abs((ipmtVal + ppmtVal) - pmtVal) > 0.001 Then
            PrintLine "FAIL: IPMT + PPMT <> PMT"
            Exit Sub
        End If

        ' PV and FV
        pvVal = PV(0.08 / 12, 120, -pmtVal)
        If Abs(pvVal - 10000#) > 0.1 Then
            PrintLine "FAIL: PV expected ~10000 got " & CStr(pvVal)
            Exit Sub
        End If

        fvVal = FV(0.08 / 12, 120, -100, 0)
        If fvVal < 18000# Then
            PrintLine "FAIL: FV invalid"
            Exit Sub
        End If

        nperVal = NPer(0.08 / 12, -pmtVal, 10000)
        If Abs(nperVal - 120#) > 0.1 Then
            PrintLine "FAIL: NPer expected ~120 got " & CStr(nperVal)
            Exit Sub
        End If

        rateVal = Rate(120, -pmtVal, 10000)
        If Abs(rateVal - (0.08 / 12)) > 0.0001 Then
            PrintLine "FAIL: Rate expected ~0.006667 got " & CStr(rateVal)
            Exit Sub
        End If

        ' 3. Cash flows for NPV, IRR, and MIRR
        cashFlows(0) = -1000#
        cashFlows(1) = 300#
        cashFlows(2) = 400#
        cashFlows(3) = 400#
        cashFlows(4) = 300#

        npvVal = NPV(0.08, cashFlows)
        If npvVal < 100# Then
            PrintLine "FAIL: NPV expected positive got " & CStr(npvVal)
            Exit Sub
        End If

        irrVal = IRR(cashFlows)
        If Abs(irrVal - 0.1388) > 0.01 Then
            PrintLine "FAIL: IRR expected ~0.1388 got " & CStr(irrVal)
            Exit Sub
        End If

        mirrVal = MIRR(cashFlows, 0.1, 0.12)
        If mirrVal <= 0# Then
            PrintLine "FAIL: MIRR expected positive"
            Exit Sub
        End If

        PrintLine "TEST 1: FINANCIAL_FUNCTIONS=PASS"
    End Sub

    ' -----------------------------------------------------------------------------
    ' TEST 2: Calendar & Date Math (DateSerial, TimeSerial, DateAdd, DateDiff,
    '         DatePart, Year, Month, Day, Hour, Minute, Second, Weekday, WeekdayName, MonthName)
    ' -----------------------------------------------------------------------------
    Private Sub TestCalendarAndDateMath()
        Dim d1 As Date
        Dim d2 As Date
        Dim dLeap As Date
        Dim t1 As Date

        ' DateSerial & TimeSerial
        dLeap = DateSerial(2024, 2, 29)
        If Day(dLeap) <> 29 Or Month(dLeap) <> 2 Or Year(dLeap) <> 2024 Then
            PrintLine "FAIL: DateSerial leap day"
            Exit Sub
        End If

        t1 = TimeSerial(14, 35, 42)
        If Hour(t1) <> 14 Or Minute(t1) <> 35 Or Second(t1) <> 42 Then
            PrintLine "FAIL: TimeSerial components"
            Exit Sub
        End If

        ' DateAdd
        d1 = DateSerial(2025, 1, 15)
        d2 = DateAdd("m", 3, d1) ' 2025-04-15
        If Month(d2) <> 4 Or Day(d2) <> 15 Then
            PrintLine "FAIL: DateAdd month"
            Exit Sub
        End If

        d2 = DateAdd("d", 20, d1) ' 2025-02-04
        If Month(d2) <> 2 Or Day(d2) <> 4 Then
            PrintLine "FAIL: DateAdd day"
            Exit Sub
        End If

        ' DateDiff
        If DateDiff("d", d1, d2) <> 20 Then
            PrintLine "FAIL: DateDiff days"
            Exit Sub
        End If

        ' DatePart
        If DatePart("yyyy", d1) <> 2025 Then
            PrintLine "FAIL: DatePart yyyy"
            Exit Sub
        End If
        If DatePart("q", d1) <> 1 Then
            PrintLine "FAIL: DatePart quarter"
            Exit Sub
        End If

        ' Weekday & Names
        If Weekday(DateSerial(2025, 1, 5), vbSunday) <> 1 Then ' Jan 5 2025 is Sunday
            PrintLine "FAIL: Weekday Sunday"
            Exit Sub
        End If

        If WeekdayName(1, False, vbSunday) <> "Sunday" Then
            PrintLine "FAIL: WeekdayName Sunday"
            Exit Sub
        End If

        If MonthName(10, False) <> "October" Then
            PrintLine "FAIL: MonthName October"
            Exit Sub
        End If

        ' IsDate, DateValue, TimeValue
        If Not IsDate("2025-10-17") Then
            PrintLine "FAIL: IsDate valid string"
            Exit Sub
        End If
        If IsDate("not_a_date") Then
            PrintLine "FAIL: IsDate invalid string"
            Exit Sub
        End If

        PrintLine "TEST 2: CALENDAR_AND_DATE_MATH=PASS"
    End Sub

    ' -----------------------------------------------------------------------------
    ' TEST 3: Locale Formatters (FormatNumber, FormatCurrency, FormatPercent, FormatDateTime)
    ' -----------------------------------------------------------------------------
    Private Sub TestFormatters()
        Dim sNum As String
        Dim sCur As String
        Dim sPct As String
        Dim sDate As String
        Dim dt As Date

        sNum = FormatNumber(1234567.89, 2)
        If InStr(sNum, "1") = 0 Or InStr(sNum, "234") = 0 Then
            PrintLine "FAIL: FormatNumber digits"
            Exit Sub
        End If

        sCur = FormatCurrency(1234.5, 2)
        If InStr(sCur, "1") = 0 Or InStr(sCur, "234") = 0 Then
            PrintLine "FAIL: FormatCurrency"
            Exit Sub
        End If

        sPct = FormatPercent(0.125, 1)
        If InStr(sPct, "12.5") = 0 Or InStr(sPct, "%") = 0 Then
            PrintLine "FAIL: FormatPercent"
            Exit Sub
        End If

        dt = DateSerial(2025, 5, 20) + TimeSerial(15, 30, 0)
        sDate = FormatDateTime(dt, vbShortDate)
        If InStr(sDate, "2025") = 0 And InStr(sDate, "25") = 0 Then
            PrintLine "FAIL: FormatDateTime short date"
            Exit Sub
        End If

        PrintLine "TEST 3: LOCALE_FORMATTERS=PASS"
    End Sub

    ' -----------------------------------------------------------------------------
    ' TEST 4: Utility functions (Switch, Partition)
    ' -----------------------------------------------------------------------------
    Private Sub TestSwitchAndPartition()
        Dim res As Variant
        Dim partStr As String

        ' Switch evaluates condition-value pairs in order and returns first True match
        res = Switch(1 = 2, "First", 3 = 3, "Second", 4 = 4, "Third")
        If CStr(res) <> "Second" Then
            PrintLine "FAIL: Switch expected Second got " & CStr(res)
            Exit Sub
        End If

        res = Switch(1 = 2, "A", 2 = 3, "B")
        If Not IsNull(res) Then
            PrintLine "FAIL: Switch with no match should return Null"
            Exit Sub
        End If

        ' Partition returns fixed-width string range " start:  stop"
        partStr = Partition(54, 0, 99, 10)
        If InStr(partStr, "50") = 0 Or InStr(partStr, "59") = 0 Then
            PrintLine "FAIL: Partition 54 in 0..99 interval 10 got '" & partStr & "'"
            Exit Sub
        End If

        PrintLine "TEST 4: UTILITY_SWITCH_AND_PARTITION=PASS"
    End Sub

    ' -----------------------------------------------------------------------------
    ' TEST 5: Virtual Registry Settings (SaveSetting, GetSetting, GetAllSettings, DeleteSetting)
    ' -----------------------------------------------------------------------------
    Private Sub TestRegistrySettings()
        Dim valTheme As String
        Dim valFont As String
        Dim allSettings As Variant

        ' Save settings
        SaveSetting "ClassicBasicApp22", "Preferences", "Theme", "DarkSlate"
        SaveSetting "ClassicBasicApp22", "Preferences", "FontSize", "14"

        ' Retrieve settings
        valTheme = GetSetting("ClassicBasicApp22", "Preferences", "Theme", "Default")
        If valTheme <> "DarkSlate" Then
            PrintLine "FAIL: GetSetting Theme expected DarkSlate got " & valTheme
            Exit Sub
        End If

        valFont = GetSetting("ClassicBasicApp22", "Preferences", "FontSize", "10")
        If valFont <> "14" Then
            PrintLine "FAIL: GetSetting FontSize expected 14 got " & valFont
            Exit Sub
        End If

        ' Default fallback
        If GetSetting("ClassicBasicApp22", "Preferences", "NonExistent", "Fallback") <> "Fallback" Then
            PrintLine "FAIL: GetSetting default fallback"
            Exit Sub
        End If

        ' GetAllSettings returns 2D array of (key, value)
        allSettings = GetAllSettings("ClassicBasicApp22", "Preferences")
        If Not IsArray(allSettings) Then
            PrintLine "FAIL: GetAllSettings not an array"
            Exit Sub
        End If

        ' Delete specific key
        DeleteSetting "ClassicBasicApp22", "Preferences", "Theme"
        If GetSetting("ClassicBasicApp22", "Preferences", "Theme", "Deleted") <> "Deleted" Then
            PrintLine "FAIL: DeleteSetting key"
            Exit Sub
        End If

        ' Delete whole section / app
        DeleteSetting "ClassicBasicApp22", "Preferences"
        DeleteSetting "ClassicBasicApp22"

        PrintLine "TEST 5: REGISTRY_SETTINGS=PASS"
    End Sub