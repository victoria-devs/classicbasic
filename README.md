# MSVBVM60.DLL - Native Rust Clone (Study Project)

Este projeto é uma re-implementação nativa em **Rust** da biblioteca em tempo de execução do **Microsoft Visual Basic 6.0 (`msvbvm60.dll`)**, criada para fins de estudo de engenharia reversa, ABI e emulação de binários legados.

A biblioteca cobre a totalidade dos **635 exports nomeados** presentes no arquivo `msvbvm60.txt` original (com ordinais variando entre 100 e 2010), garantindo compatibilidade binária no carregamento e na resolução de símbolos.

---

## 🏛 Arquitetura do MSVBVM60

O `msvbvm60.dll` é o coração de qualquer aplicação compilada em Visual Basic 6.0 (seja em **Native Code** ou **P-Code**):

1. **`ThunRTMain` (Ordinal 100)**:
   - "Thunder Runtime Main": ponto de entrada principal invocado pelo `main` do executável VB6 gerado pelo compilador.
   - Recebe como argumento um ponteiro para a estrutura `VBHeader` (iniciada pela assinatura mágica `VB5!`).
   - Inicializa subsistemas COM/OLE, registra manipuladores de exceção estruturada (SEH), analisa a tabela de objetos (`ObjectTable`), executa a `Sub Main` ou instancia o formulário inicial, e gerencia o loop de mensagens da interface gráfica.

2. **Engines de P-Code (`ProcCallEngine`, `MethCallEngine`)**:
   - Para binários compilados em P-Code, os opcodes da máquina virtual VB6 são despachados através dessas engines.

3. **Manipulação de Memória (`VarPtr`, `GetMem*`, `PutMem*`)**:
   - Como o VB6 não expunha aritmética de ponteiros diretamente, funções não documentadas como `VarPtr` (que simplesmente retorna o endereço da variável) e `GetMem1/2/4/8` / `PutMem1/2/4/8` foram e ainda são extensamente utilizadas pela comunidade para interop com Win32 e manipulação de memória de baixo nível.

4. **Biblioteca Padrão VBA (`rtc*`)**:
   - Rotinas nativas de runtime como `MsgBox` (`rtcMsgBox`), `DoEvents` (`rtcDoEvents`), `RGB` (`rtcRgb`), `Shell` (`rtcShell`), `Trim` (`rtcTrimBstr`), etc.

5. **Ajudantes de Compilação (`__vba*`)**:
   - Gerenciamento de strings OLE `BSTR` (`__vbaStrCopy`, `__vbaStrCat`, `__vbaFreeStr`).
   - Aritmética e conversão de `VARIANT` (`__vbaVarAdd`, `__vbaVarMove`, `__vbaFreeVar`).
   - Alocação e manipulação de `SAFEARRAY` (`__vbaAryConstruct`, `__vbaAryDestruct`, `__vbaAryLock`).

6. **Workarounds Históricos do Pentium FDIV (`_adj_*`) e CRT**:
   - Rotinas matemáticas que o compilador C2 do MSVC gerava para contornar o bug de divisão de ponto flutuante do primeiro Intel Pentium, além de rotinas FPU x87 (`_CI*`) e `_allmul`.

---

## 📁 Estrutura do Código-Fonte

```text
.
├── Cargo.toml               # Configuração do crate (cdylib e rlib)
├── .cargo/
│   └── config.toml          # Flags do linker para associar o msvbvm60.def
├── msvbvm60.def             # Definição dos 635 exports com seus ordinais exatos
├── artifacts/               # Artefatos de entrada para engenharia reversa
│   ├── README.md            # Documentação dos artefatos
│   ├── REFERENCE.md         # Notas de ambiente da VM (Win 2000 SP4, QEMU x86)
│   ├── msvbvm60.dll         # DLL original do Windows 2000 SP4
│   ├── msvbvm60.txt         # Dump original de exports (dumpbin /exports)
│   ├── transfer.img         # Imagem de disco de transferência (UTM/QEMU)
│   └── reports/             # Relatórios de ambiente (rea-doctor, etc.)
├── src/
│   ├── lib.rs               # Raiz da biblioteca, DllMain, testes e documentação
│   ├── thun_rt.rs           # Implementação de ThunRTMain, ProcCallEngine, MethCallEngine
│   ├── mem.rs               # Implementação real de VarPtr e todos os GetMem/PutMem/SetMem
│   ├── com.rs               # DllGetClassObject, IID_IVbaHost, BASIC_CLASS_*, Zombie_*, etc.
│   ├── math.rs              # _CI*, _adj_fdiv_* e _allmul
│   ├── rtc/
│   │   └── mod.rs           # Funções rtc* (rtcMsgBox, rtcDoEvents, rtcRgb, etc.)
│   ├── vba/
│   │   └── mod.rs           # Funções __vba* (__vbaStrCopy, __vbaVarAdd, __vbaAry*, etc.)
│   ├── stubs.rs             # Stubs automáticos dos 517 exports adicionais da ABI
│   └── types/
│       ├── mod.rs           # Definições de HRESULT e re-exports
│       ├── vb_header.rs     # Layout binário exato de VBHeader, ProjectInfo, ObjectTable
│       ├── variant.rs       # Estrutura VARIANT (16 bytes) e constantes VT_*
│       ├── bstr.rs          # Definição e inspeção de BSTR (OLE String)
│       ├── safearray.rs     # Estruturas SAFEARRAY e SAFEARRAYBOUND
│       ├── currency.rs      # Tipo Currency (CY) de 64-bits com escala 10.000
│       └── guid.rs          # Tipo GUID / IID / CLSID
```

---

## ⚙️ Como Compilar

Binários do Visual Basic 6.0 rodam estritamente em arquitetura **32-bit x86 (`i686`)**.

### 1. Compilação para Windows 32-bit (MSVC)
```bash
rustup target add i686-pc-windows-msvc
cargo build --target i686-pc-windows-msvc --release
```

### 2. Cross-compilação com MinGW (Linux/macOS)
```bash
rustup target add i686-pc-windows-gnu
cargo build --target i686-pc-windows-gnu --release
```

O arquivo gerado em `target/<target>/release/msvbvm60.dll` conterá a tabela de exports exatamente compatível com a DLL original graças ao arquivo `msvbvm60.def`.

### 3. Rodar Testes Unitários Locais
```bash
cargo test
```
