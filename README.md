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
├── src/bin/
│   └── vb6_run.rs           # Executável CLI `vb6-run` (PE Loader / Runner nativo)
└── src/pe/
    ├── mod.rs               # Módulo do carregador PE
    ├── headers.rs           # Definições binárias PE32 (IMAGE_DOS_HEADER, NT_HEADERS, etc.)
    ├── loader.rs            # Mapeador de memória virtual, realocações e IAT
    ├── resolver.rs          # Resolução dinâmica de símbolos e ordinais da MSVBVM60
    └── shims.rs             # Shims mínimos para Kernel32 e User32 sem Wine
```

---

## ⚙️ Como Compilar e Usar

### 1. Compilação da Biblioteca Runtime Pura (`msvbvm60`)

#### Windows 32-bit (MSVC DLL nativa)
```bash
rustup target add i686-pc-windows-msvc
cargo build --lib --target i686-pc-windows-msvc --release
```

#### MinGW PE DLL (Cross-compilação no Linux/macOS para Wine)
```bash
rustup target add i686-pc-windows-gnu
cargo build --lib --target i686-pc-windows-gnu --release
```

#### Linux Nativo (Shared Library ELF `libmsvbvm60.so`)
```bash
rustup target add i686-unknown-linux-gnu
cargo build --lib --target i686-unknown-linux-gnu --release
```

---

### 2. Executável Standalone sem Wine: `vb6-run` (Linux / macOS)

O binário `vb6-run` é um **carregador PE32 nativo** capaz de ler executáveis `.exe` legados do VB6, mapear suas seções na memória, resolver realocações de endereço, ligar as importações da `MSVBVM60.DLL` às funções Rust e executar o entry point diretamente:

```bash
# Compilar o runner
cargo build --bin vb6-run --release

# Inspecionar headers, seções e metadados VBHeader sem rodar:
./target/release/vb6-run caminho/para/projeto.exe --inspect

# Dump detalhado de ProjectInfo e ObjectTable:
./target/release/vb6-run caminho/para/projeto.exe --dump-info

# Executar diretamente o projeto legado em Linux:
./target/release/vb6-run caminho/para/projeto.exe
```

---

### 3. Testes Automatizados
```bash
cargo test
```
Os testes incluem validação da máquina virtual, aritmética de variantes, conversão de Moeda (Currency), pointers (`GetMem`/`PutMem`/`VarPtr`) e carregamento de binários PE32 sintéticos via `LoadedPe`.
