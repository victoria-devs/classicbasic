# Reverse Engineering Artifacts & Test Data

This directory contains test inputs, reference specifications, and symbol export tables used for validating the **`msvbvm60`** runtime clone.

---

## 📦 Directory Contents

| File | Size | Description |
| :--- | :--- | :--- |
| **`App01-helloworld-pcode.exe`** | 12 KB | Sample standalone test binary compiled with Visual Basic 6.0 (PE32 / P-Code Sub Main console application). Used to validate the PE32 loader, `VBHeader` parser, and native Linux container execution. |
| **`App03-bstr-native.exe`** | 20 KB | Compiled Native Code VB6 console application validating BSTR lifecycle and string operations (`Concat`, `Len`, `Mid`, `Left`, `Right`, `InStr`, `UCase`, `CStr`). |
| **`App03-bstr-pcode.exe`** | 12 KB | Compiled P-Code VB6 console application validating BSTR lifecycle and string operations (`Concat`, `Len`, `Mid`, `Left`, `Right`, `InStr`, `UCase`, `CStr`). |
| **`App04-variant-native.exe`** | 20 KB | Compiled Native Code VB6 console application validating the Full VARIANT dynamic coercion engine (`VarAdd`, `VarSub`, `VarMul`, `VarDiv`, `VarIdiv`, `VarMod`, `VarCat`, `VarNeg`, `VarNot`, `VarTst`, and `VarType`). |
| **`App04-variant-pcode.exe`** | 12 KB | Compiled P-Code VB6 console application validating the Full VARIANT dynamic coercion engine (`VarAdd`, `VarSub`, `VarMul`, `VarDiv`, `VarIdiv`, `VarMod`, `VarCat`, `VarNeg`, `VarNot`, `VarTst`, and `VarType`). |
| **`App05-fileio-native.exe`** | 20 KB | Compiled Native Code VB6 console application validating Win32 File I/O shims (`CreateFileA`, `WriteFile`, `ReadFile`, `CloseHandle`, `GetFileSize`, and error handling). |
| **`App05-fileio-pcode.exe`** | 12 KB | Compiled P-Code VB6 console application validating Win32 File I/O shims (`CreateFileA`, `WriteFile`, `ReadFile`, `CloseHandle`, `GetFileSize`, and error handling). |
| **`msvbvm60.txt`** | 27 KB | Export symbol dump generated with `dumpbin /exports`. Contains the complete listing of all 635 named export symbols (ordinals 100 through 2010), hints, and relative virtual addresses (RVAs) used for ABI compatibility mapping. |
| **`REFERENCE.md`** | 277 B | Specification of the reference reverse engineering environment and compiler versions (Windows 2000 SP4, VB6 v8176/6.0.8169). |

> [!NOTE]
> **Proprietary Binaries Notice**:
> In accordance with software copyright standards, this repository **does not distribute** proprietary Microsoft binaries or dynamic-link libraries (`MSVBVM60.DLL`). 
> Users wishing to perform side-by-side differential analysis or binary verification may supply their own legally obtained copy extracted from a licensed Windows installation or official redistributable package.

> [!IMPORTANT]
> **Trademark Notice**:
> Microsoft®, Windows®, Visual Basic®, and MSVB are registered trademarks or trademarks of Microsoft Corporation or their respective owners. ClassicBasic is an independent clean-room implementation not affiliated with Microsoft Corporation.
